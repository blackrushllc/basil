Phase 3 is where `obj-ai` stops being “just text” and becomes an **agentic copilot** inside Basil. The model can *ask*
to call safe, whitelisted Basil functions (tools), you run them, then feed results back for a final answer. Below is a
crisp spec you can adopt—and a copy-paste Junie prompt at the end if you want to build it straight away.

# What “tool/function-calling” means

* The model returns a structured **tool call** (name + JSON args) instead of or before a final message.
* Basil validates the request against a **registry** of exposed functions you define.
* Basil executes, captures the result (string/JSON/array), and returns it to the model as a **tool result** message.
* The model may chain multiple calls, then produce a final normal message.

# Design goals

* **Zero-magic**: explicit register → explicit allowlist.
* **Deterministic & safe**: permissions + timeouts + resource limits + pure-ish defaults.
* **Simple in BASIC**: two or three statements to register tools; one call to run a tool-enabled chat.
* **Pluggable**: works in CLI `--ai` and inside user programs.

---

# Public Basil API (proposed)

### Register tools

```basic
' Register a tool by name with a callable Basil function
AI.TOOL.REGISTER "get_time", "Returns the current local time.", _
                  "{ }", "string", ADDRESSOF(get_time$)

FUNCTION get_time$()
  RETURN TIME$()
END FUNCTION

' Optional: attach a per-tool rate limit / timeout
AI.TOOL.LIMITS "get_time", "{ timeout_ms: 800, rate_per_min: 20 }"
```

### Unregister / list

```basic
AI.TOOL.UNREGISTER "get_time"
DIM tools$[] = AI.TOOL.LIST$()
```

### Tool-enabled chat

```basic
DIM opts$ = "{ system:'You can call tools to help answer.', allow_tools:true, max_tool_calls:3 }"
PRINT AI.CHAT$( "What time is it? Use tools if helpful.", opts$ )
```

### Streamed, tool-aware session (optional sugar)

```basic
AI.SESSION.OPEN "{ allow_tools:true }"
AI.SESSION.USER "Plan a 3-step checklist; call `get_time` once."
AI.SESSION.RUN()       'streams tokens, handles tool calls
AI.SESSION.CLOSE
```

---

# Tool schema (what the model sees)

Each registered tool is described to the model with a compact JSON schema:

```json
{
  "name": "get_weather",
  "description": "Get current weather by city and country code.",
  "input_schema": {
    "type": "object",
    "properties": {
      "city": {"type":"string"},
      "country": {"type":"string","minLength":2,"maxLength":2}
    },
    "required":["city","country"],
    "additionalProperties": false
  },
  "returns": "json"
}
```

Basil will:

* compile a **tool catalog** (array of tool descriptors),
* include it in the chat request (e.g., `tools:[…]`, `tool_choice:'auto'`),
* parse tool calls from model responses: `{ "tool_call": { "name": "...", "arguments": {…} } }`.

---

# Minimal tool-call lifecycle

1. **User**: “Add 4 bars of lofi drums at bar 1.”
2. **Model**: returns `tool_call` → `insert_clip` with `{track:"Drums", bars:4, style:"lofi", start_bar:1}`.
3. **Basil dispatcher**:

    * Validate: exists? schema ok? permissions ok?
    * Execute registered BASIC function (or Rust-backed native).
    * Capture result (string/JSON), clamp size.
4. **Basil** sends **tool result** message back to model.
5. **Model**: may call more tools, or produce final answer.
6. **Basil**: stream/print final answer (and do any side effects already done by tools).

---

# Safety model (important)

* **Allowlist only**: tools must be explicitly registered during program startup or REPL session. No global wildcards.
* **Sandbox**:

    * No filesystem writes by default; read-only helpers must specify allowed paths or use virtual in-memory stores.
    * No shell execution tools unless explicitly registered with constraints (generally discouraged).
    * Network: provide **zero** network tools by default; if you expose HTTP, enforce DNS allowlist + per-call timeout + size limit.
* **Runtime guardrails**:

    * `max_tool_calls` per request/session (default 1–3).
    * Per-tool `timeout_ms`, `rate_per_min`, and max output bytes (e.g., 64KB).
    * Automatic **argument JSON validation** against input_schema.
* **Audit**:

    * Emit a compact log per tool call: timestamp, tool, args hash, duration, status.
    * Optionally, show a user-visible “Tool used: …” line in CLI with `--verbose`.
* **Test mode**:

    * Tool calls still run, but if a tool is “unsafe” (network, FS) it becomes a **no-op with canned output** unless explicitly allowed via `TEST_ALLOW=1`.

---

# Starter tool ideas (safe & useful)

* `get_time` → returns ISO timestamp string.
* `kv_get` / `kv_set` → in-memory session KV store (size-limited).
* `math_eval` → deterministic evaluator for simple expressions.
* `doc_search` → local embeddings lookup (Phase 2) wrapped as a tool.
* `insert_clip` (DAWG) → add a ClipSpec at bar N on a track (use your validator); returns `{clip_id, bars, track}`.

---

# Example: Registering a DAWG tool

```basic
AI.TOOL.REGISTER "insert_clip", _
  "Insert a validated ClipSpec into the DAWG timeline. Args: {track, start_bar, spec}.", _
  "{
     type:'object',
     properties:{
       track:{type:'string'},
       start_bar:{type:'integer', minimum:1},
       spec:{type:'object'}
     },
     required:['track','start_bar','spec'],
     additionalProperties:false
   }",
  "json",
  ADDRESSOF(insert_clip_json$)

FUNCTION insert_clip_json$(args_json$)
  ' Parse args
  DIM track$ = JSON.GET$(args_json$, "track")
  DIM start% = JSON.GETINT%(args_json$, "start_bar")
  DIM spec$  = JSON.GETOBJ$(args_json$, "spec")

  ' Validate/fix ClipSpec then build clip
  DIM fixed$ = MIDI.SPEC.VALIDATE$(spec$)
  DIM id% = MIDI.CLIP.FROM_SPEC(fixed$)
  MIDI.CLIP.INSERT id%, track$, start%

  RETURN "{ ""ok"": true, ""clip_id"": " + STR$(id%) + " }"
END FUNCTION
```

Then a tool-aware ask:

```basic
PRINT AI.CHAT$("Create a 4-bar lofi drum ClipSpec and insert it at bar 1 on 'Drums'.", _
               "{ allow_tools:true, max_tool_calls:2 }");
```

---

# Rust plumbing (overview)

* `tools/registry.rs`

    * `register(name, schema, returns, limits, fnptr)`
    * `get(name) -> Tool`
    * `list() -> Vec<ToolDescriptor>`
* `tools/validator.rs`

    * JSON Schema-ish validator (keep it small; or implement “shape” checks)
* `tools/dispatcher.rs`

    * `invoke(tool, args_json, ctx) -> ToolResult`
    * Enforce limits/timeouts (tokio timeout), cap output size
* `chat/tools_protocol.rs`

    * Map model tool_call <-> internal structs
    * Assemble tool catalog into the outbound request
    * Append tool result messages for the follow-up call
* `ffi.rs`

    * BASIC bindings for REGISTER/UNREGISTER/LIST/LIMITS
    * Bridge tool-enabled `AI.CHAT$` / `AI.SESSION.RUN`

**Streaming UX**
Print assistant tokens as usual. When a tool call occurs:

* show a short status line: `↪ tool: insert_clip(args…)`
* after dispatch: `✔ tool result received (34ms)`, then resume stream

---

# Error handling

* Bad tool name → model gets a brief tool error result `{error:"unknown_tool"}` and we ask it to continue without that tool.
* Arg validation error → `{error:"invalid_args", details:"…"}`
* Timeout → `{error:"timeout"}`
* Tool panicked → `{error:"internal"}` (message redacted), and you log details privately.
* If **every** tool attempt fails: fall back to a normal `AI.CHAT$` answer.

---

# Testing checklist

* Unit:

    * registry add/remove/list
    * schema validator happy/sad paths
    * timeout fires
    * output-size clamp
* Integration:

    * single-call path (model → tool → final)
    * multi-call path (2–3 tools chained)
    * malformed tool name/args
    * test mode with unsafe tool blocked
* CLI:

    * `--ai` can toggle `allow_tools` and show compact logs

---

# Demo scripts (put in `examples/obj-ai-tools/`)

**01_time_tool.basil**

```basic
AI.TOOL.REGISTER "get_time", "Return local time ISO string", "{}", "string", ADDRESSOF(get_time$)
FUNCTION get_time$(): RETURN DATE$() + "T" + TIME$(): END FUNCTION
PRINT AI.CHAT$("What's the time? Use tools if needed.", "{ allow_tools:true }")
```

**02_insert_clip_tool.basil**

```basic
REM Register insert_clip as shown above, then:
PRINT AI.CHAT$("Make 4 bars of lofi drums and insert at bar 1 on Drums.", "{ allow_tools:true }")
TRANSPORT.PLAY
```

**03_kv_store.basil**

```basic
AI.TOOL.REGISTER "kv_set", "Set a small key/value", "{type:'object',properties:{k:{type:'string'},v:{type:'string'}},required:['k','v']}", "json", ADDRESSOF(kv_set$)
AI.TOOL.REGISTER "kv_get", "Get a small value by key", "{type:'object',properties:{k:{type:'string'}},required:['k']}", "json", ADDRESSOF(kv_get$)
' Implement kv_set$/kv_get$ using a module-level dictionary
PRINT AI.CHAT$("Remember my name is Erik; then tell me what I said my name is.", "{ allow_tools:true, max_tool_calls:2 }")
```

---

# Copy-paste prompt for Junie (Phase 3 build)

**Prompt:**
Implement **Phase 3 tool/function-calling** for `obj-ai`.

Scope:

* Add a **tool registry** with BASIC bindings:

    * `AI.TOOL.REGISTER name, description, input_schema$, returns$, fn_address`
    * `AI.TOOL.UNREGISTER name`
    * `AI.TOOL.LIST$() -> string[]`
    * `AI.TOOL.LIMITS name, limits_json$` (supports `{timeout_ms, rate_per_min, max_output_bytes}`)
* Extend `AI.CHAT$` and `AI.SESSION.RUN()` with `allow_tools` + `max_tool_calls` options.
* Chat loop:

    1. Send tool catalog when `allow_tools=true`.
    2. If response contains a `tool_call`, validate args vs `input_schema`, enforce per-tool limits.
    3. Execute the mapped BASIC/Rust function (tokio timeout), capture output (string/JSON).
    4. Append a `tool_result` message and continue once.
    5. Cap at `max_tool_calls` (default 1).
* Safety defaults:

    * No tools registered by default.
    * In `--test`, unsafe tools (network/FS-exec) are no-op unless `TEST_ALLOW=1`.
    * Global `max_tool_calls` default 1; `timeout_ms` default 1000; `max_output_bytes` default 64k.
* Logging/audit: per call `{tool, ms, ok/err}` to debug log; `--ai --verbose` prints one-line status.
* Docs:

    * `docs/obj-ai/TOOLS.md`: how to register tools; schema examples; safety/limits; test guidelines.
* Demos:

    * `examples/obj-ai-tools/01_time_tool.basil`
    * `examples/obj-ai-tools/02_insert_clip_tool.basil`
    * `examples/obj-ai-tools/03_kv_store.basil`

Acceptance:

* Builds with `--features obj-ai`.
* Demo scripts run; tool calls execute; limits/timeouts enforced; errors surfaced to `AI.LAST_ERROR$` only when fatal.
* In `--test`, unsafe tools are blocked by default.

---

If you want, I can tailor the **schema validator** to your existing JSON helper, or sketch the Rust `registry/dispatcher` modules next.
