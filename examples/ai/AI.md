# Basil + AI ( + MIDI)


# Phase 1 & 2 spec for `obj-ai` feature


* **Goal**: make it easy to build AI-powered tools and apps in Basil.
* **Audience**: students, hobbyists, and professionals.

* **Two entry points**

    1. **AI mode in the Basil CLI**: conversational helper that can explain errors, generate snippets, and scaffold files.
    2. **`obj-ai` feature library**: Basil keywords/functions so *programs* can call LLMs, embeddings, and (optionally) tools.

* **North star**: “One line to talk to an AI; optional advanced controls when needed.”

# MVP in 1 week (realistic scope)

1. **Core chat function** (sync + streaming):

    * `AI.CHAT$(prompt$[, options$])` → returns a string.
    * `AI.STREAM(prompt$[, options$])` → prints tokens as they arrive; returns full text when done.
2. **Simple options** via JSON-ish string:

    * model, system prompt, temperature, max_tokens.
3. **Error handling**:

    * Returns an empty string on transport errors and sets `AI.LAST_ERROR$`.
4. **Config**:

    * API key in env or `.basil-ai.toml`.
5. **Test harness**:

    * Respect `basilc --test`: mock responses so examples/doc tests run offline.
6. **CLI AI mode**:

    * `basilc --ai` opens a REPL: you type questions; it answers; `:code` emits a snippet block; `:explain file.basil:50` explains a line.

### Part 2 (next 1–2 weeks)

* **Function calling / Tools**: allow the model to request calling Basil-exposed functions (safe, whitelisted).
* **Embeddings**: `AI.EMBED(vec$)` returns a numeric vector[]; add tiny local cosine similarity helper.
* **Moderation** (boolean pass/fail): `AI.MODERATE%(text$)`.
* **Caching**: hash(prompt + options) → local cache file; opt-in `cache=true`.

# Basil API design (friendly & BASIC-y)

### 1) One-liners for students

```basic
PRINT AI.CHAT$("Explain bubble sort in 3 bullet points");
```

```basic
PRINT AI.CHAT$("Write Basil code that prints the first 10 primes", _
               "{ model:'mini', max_tokens:256 }");
```

### 2) Streaming (cool for CLI demos)

```basic
PRINT "AI says: ";
DIM full$ = AI.STREAM("Tell me a one-liner joke about BASIC");
PRINT "\n\n[Done]\n";
```

### 3) System prompt & temperature

```basic
DIM opts$ = "{ system:'You are a calm Basil tutor.', temperature:0.2 }";
PRINT AI.CHAT$("Why does this loop never end? " + PROGRAM.TEXT$(1, 120), opts$);
```

### 4) Embeddings + local search (Phase 2)

```basic
DIM q$ = "How do I open a file in Basil?";
DIM qv[] = AI.EMBED(q$);

REM Suppose docs[] and vecs[][] were precomputed at startup
DIM best_i% = COSINE.MAX_SIMILARITY_INDEX(qv[], vecs[][]);
PRINT "Closest doc: " + docs[best_i%];
```

### 5) Moderation gate (Phase 2)

```basic
IF AI.MODERATE%("Generate a phishing email") = 0 THEN
  PRINT AI.CHAT$("Write a polite business email asking for a meeting");
ELSE
  PRINT "Request blocked by moderation.";
END IF
```

### 6) Tool / Function-calling (Phase 2+)

Expose Basil functions to the AI via a registry:

```basic
AI.TOOL.REGISTER "get_time", "Returns the current time", "{}", "string";

FUNCTION get_time$()
  RETURN TIME$();
END FUNCTION
```

Then:

```basic
DIM opts$ = "{ tools:[ 'get_time' ], allow_tools:true }";
PRINT AI.CHAT$("What's the time? Use tools if helpful.", opts$);
```

Runtime loop:

* Model returns a **tool_call** → Basil invokes `get_time$()` → sends tool result back → model returns final answer.
  You can keep this minimal: JSON messages + a tiny dispatcher.

# CLI AI mode behavior

* `basilc --ai`

    * **Plain prompt** → calls `AI.CHAT$`.
    * `:code <ask>` → reply wrapped in `basil … `
    * `:fix <file>:<line>` → explain error + propose patch
    * `:explain <file>[:line-range]` → summarize selected code
    * `:sys <system text>` → set session system prompt
    * `:model <name>` → switch model
    * `:save <path>` → save last answer
    * `:tools on|off` (Phase 2)
    * `:quit`

Internally this is just a thin wrapper over `AI.SESSION` (optional stateful chat helper).

# Rust architecture sketch

* Crate: `obj-ai` behind feature flag `obj-ai`
* Module layout:

    * `config.rs` – reads env + `.basil-ai.toml`
    * `client.rs` – HTTPS POST, handles streaming (chunked SSE or HTTP chunked)
    * `chat.rs` – high-level `chat(prompt, opts)`; JSON options parser (serde)
    * `stream.rs` – callback/token iterator hooked to Basil’s PRINT
    * `embeddings.rs` – POST to embeddings endpoint
    * `moderation.rs` – simple boolean
    * `cache.rs` – SHA256(prompt+opts) → JSON file
    * `tools.rs` – registry: name → (signature, callable)
* Basil bindings:

    * Expose as statements/functions:

        * `AI.CHAT$(prompt$, [opts$])`
        * `AI.STREAM(prompt$, [opts$])`
        * `AI.EMBED(text$)` → float[]
        * `AI.MODERATE%(text$)` → int
        * `AI.LAST_ERROR$` → string
        * (Later) `AI.TOOL.REGISTER …`

# Options string format

* Keep it JSON-ish but forgiving:

    * `{ model:'gpt-4o-mini', temperature:0.7, max_tokens:400, system:'You are a Basil tutor.' }`
* Parser rules:

    * Accept single quotes, unquoted keys, and trailing commas.
    * Unknown keys ignored (future-proof).

# Safety & cost guardrails

* **Rate limiting**: tokens/sec gate; backoff on 429.
* **Budget**: optional `max_tokens_per_run`, `max_requests_per_min` in config.
* **Moderation**: short-circuit clearly unsafe asks (configurable).
* **Offline/Test**: `--test` mode returns deterministic canned text; no network.

# Editor & workflow integration

* **PhpStorm/RustRover helpers**: a run config for `basilc --ai`.
* **Quick templates**: right-click → “Ask Basil-AI to explain/optimize this selection”.
* **Snippets**: generate Basil programs from prompts; drop into `/examples`.

# Example: “numbers game” generator (student demo)

```basic
DIM spec$ = "Make a numbers guessing game in Basil. " + _
            "Keep it under 40 lines, validate input, and replay on win.";

DIM code$ = AI.CHAT$("Generate only code, no commentary:\n" + spec$, _
                     "{ system:'You write Basil code only.', temperature:0.4 }");

IF LEN(code$) = 0 THEN
  PRINT "AI error: " + AI.LAST_ERROR$;
ELSE
  FILE.WRITE$("numbers_game.basil", code$);
  PRINT "Wrote numbers_game.basil";
END IF
```

# Example: Explain an error

```basic
DIM src$ = FILE.READ$("main.basil");
DIM ask$ = "Explain the bug briefly and show a small fix.\n\n" + src$;
PRINT AI.CHAT$(ask$, "{ system:'You are a Basil linter.', temperature:0 }");
```

# Minimal `.basil-ai.toml`

```toml
api_key = "env:OPENAI_API_KEY"
default_model = "gpt-4o-mini"
temperature = 0.3
max_tokens = 400
cache = true
tools_enabled = false
```

# What “AI things” can Basil programs *do*?

* **Tutoring**: inline “explain this code” or “comment every line” features.
* **Code gen**: scaffold CRUD demos, CGI snippets, or test inputs.
* **Text utilities**: summarize logs, extract structured data (JSON), rewrite content.
* **Q&A over local docs**: embeddings + cosine search over project README/docs.
* **Agentic utilities (Phase 2)**: call whitelisted tools (HTTP GET, FILE.READ, CLOCK) via function calling.

# Nice extras (later)

* **Conversation transcripts** saved in `/var/basil/ai-sessions/…`
* **Prompt templates** in `/usr/share/basil/ai-prompts/…`
* **Session memory**: key–value store the model can query (careful!).

---

If you want, I can spit out:

1. a `obj-ai` Rust module skeleton (with stubs for `AI.CHAT$`, streaming, config),
2. a tiny `.basil` sample app that uses it, and
3. a draft README for the repo.


# SIDE NOTE: Combining AI with MIDI (what??)

Right?! DAWG + `obj-ai` is 🔥. Here’s a tight, buildable way to fuse them so you can *ask* for music and hear it in the transport seconds later.

# Big idea

Natural-language ⇄ MIDI. You tell Basil/DAWG what you want (“give me a four-bar lo-fi beat at 85 BPM with swung hats”), the AI returns a tiny, structured clip spec, and the DAW drops it into the timeline, ready to loop, quantize, and play.

# MVP (one sprint)

**New feature flag:** `--features obj-ai,obj-midi`
**New module namespace:** `AI.MUSIC.*`

### 1) Friendly prompts → structured clips

* `AI.MUSIC.GENERATE$(prompt$, [opts$])`
  Returns a compact JSON-ish “ClipSpec” (bars, track, tempo?, key?, notes[] with start/beats, length/beats, pitch, vel, chan).

* `AI.MUSIC.DESCRIBE$(clip$)`
  Human readable summary (“4 bars, key Am, swing 55%, drums on ch10, ghost hats…”).

* `AI.MUSIC.VARIATION$(clip$, prompt$)`
  Make it busier/sparser, change style, invert chords, double-time hats, etc.

### 2) DAW bridge (hooks into your transport & piano roll)

* `MIDI.CLIP.FROM_SPEC(clip$)` → clip_id%
* `MIDI.CLIP.INSERT clip_id%, track%, start_bar%`
* `MIDI.CLIP.PREVIEW clip_id%` (audition)
* `TRANSPORT.PLAY`, `TRANSPORT.LOOP.SET start_bar%, end_bar%`

### 3) Guardrails & offline mode

* If `--test` or no API key: fall back to rule-based generators (kick-snare grid, I–V–vi–IV chord stubs) so examples/demo still work.

---

# What it feels like (Basil)

### Generate a drum loop and drop it at bar 1 on “Drums”

```basic
DIM ask$ = "4 bars, 85 BPM, lo-fi hip hop. Drums only. \
            swing 54%. Kicks sparse, snares 2 & 4, ghost hats.";
DIM spec$ = AI.MUSIC.GENERATE$(ask$, "{ model:'mini', temperature:0.3 }");

IF LEN(spec$) = 0 THEN
  PRINT "AI error: "; PRINT AI.LAST_ERROR$; STOP
END IF

DIM clip_id% = MIDI.CLIP.FROM_SPEC(spec$)
MIDI.CLIP.INSERT clip_id%, TRACK.ID("Drums"), 1
TRANSPORT.LOOP.SET 1, 5
TRANSPORT.PLAY
```

### Ask for chords, then a bass line that follows

```basic
DIM chords$ = AI.MUSIC.GENERATE$("4 bars, key Am, 90 BPM, \
  i–VI–III–VII pad chords, legato, soft.", "{}");

DIM chords_id% = MIDI.CLIP.FROM_SPEC(chords$)
MIDI.CLIP.INSERT chords_id%, TRACK.ID("Pads"), 1

DIM bass$ = AI.MUSIC.VARIATION$(chords$, "Generate bass line that \
  follows roots with occasional passing tones; 8ths; mellow.");

DIM bass_id% = MIDI.CLIP.FROM_SPEC(bass$)
MIDI.CLIP.INSERT bass_id%, TRACK.ID("Bass"), 1
```

### “Make this busier and add fills on bar 4”

```basic
DIM d$ = MIDI.CLIP.TO_SPEC(TRACK.ID("Drums"), 1, 4)  'export existing drums
DIM v$ = AI.MUSIC.VARIATION$(d$, "Increase hat density 10%. \
  Add snare fill on last 2 beats of bar 4.")

DIM v_id% = MIDI.CLIP.FROM_SPEC(v$)
MIDI.CLIP.REPLACE v_id%, TRACK.ID("Drums"), 1
```

---

# The ClipSpec (small, predictable)

AI returns a forgiving JSON-ish blob you already parse elsewhere:

```json
{
  bars: 4,
  tempo: 85,
  key: "Am",
  track: "Drums",
  notes: [
    { beat: 1.0, len: 1.0, pitch: 36, vel: 110, ch:10 },
    { beat: 2.0, len: 1.0, pitch: 38, vel: 100, ch:10 },
    { beat: 1.0, len: 0.5, pitch: 42, vel: 70, ch:10 },
    { beat: 1.5, len: 0.5, pitch: 42, vel: 64, ch:10 },
    ...
  ],
  swing: 0.54
}
```

* Basil side: accept single quotes, trailing commas; default `ch=1`, `vel=96`, `len=0.5` if omitted.
* Your existing `json-lite` helper can parse; otherwise a tiny permissive parser works.

---

# CLI sweetness (DAWG)

Add an **AI lane** to your REPL:

* `:ai make drums 4 bars lofi 85bpm swing 54` → inserts a clip on selected track at playhead
* `:ai var "busier hats + bar-4 fill"` → variations on the selected clip
* `:ai chords "i-vi-iii-vii in Am across 4 bars, pad"` → new clip on Pads
* `:ai describe` → short natural summary of the selected clip
* `:ai humanize 8%` → not AI, but pairs well; keeps the UX consistent

While generating, stream tokens to status line: `AI: generating… tokens=482`

---

# How the plumbing fits (sketch)

1. **Prompt builder (music-aware)**

    * You assemble a strict system prompt: *“You are a MIDI clip generator. Return only ClipSpec JSON matching this schema…”*
    * Include a **schema example** and **quantization grid** (e.g., 1/16) and allowable pitches (GM drums vs chromatic instruments).

2. **`obj-ai` call**

    * `AI.CHAT$(prompt, opts)` or `AI.STREAM` if you want spinner/progress.

3. **Validator & fixer**

    * `MIDI.SPEC.VALIDATE(spec$)` clamps out-of-range pitches, trims notes past bar end, snaps to grid if `snap=true`.

4. **Clip builder**

    * Converts to your internal `NoteEvent { start_ticks, duration, pitch, velocity, ch }` and inserts into the piano-roll/track.

5. **Audition**

    * Auto-preview or drop straight into loop region and play.

---

# “No AI key? No problem.”

Ship **fallback generators** so the UX still sings:

* `PATTERN.DRUMS(lofi|house|rock, bars, swing)`
* `CHORDS.PROG(key, bars, roman_numerals[])`
* `BASS.FOLLOW(chords_spec, density)`
  These also double as few-shot examples in the AI system prompt.

---

# Safety & cost rails

* Default `max_tokens=600`, `temperature=0.3`, caching on (hash prompt → `.dawg/ai-cache/`).
* On malformed output: try `AI.FIX.JSON$(raw$)` once (asks model to repair JSON), else fallback generator.
* Policy toggles: disallow lyrics generation if you don’t want text features; rate-limit generation per minute.

---

# “One more thing” ideas (Phase 2)

* **Describe → Match:** “Match the drums to this reference clip’s groove” (AI returns per-step velocity & timing offsets).
* **Style transfer:** “Turn this 4-bar piano into ragtime feel” (remap voicings & syncopation).
* **Call-and-response:** generate Q/A phrases between Lead and Keys tracks.
* **Arrangement blocks:** “Create A(8 bars), B(8 bars), fill(2 bars)” and place them on the timeline.

---

# Hand-off checklist for Junie (copy/paste)

* [ ] Implement `AI.MUSIC.GENERATE$`, `VARIATION$`, `DESCRIBE$` in `obj-ai` with strict system prompt + schema.
* [ ] Add `MIDI.CLIP.FROM_SPEC`, `TO_SPEC`, `INSERT`, `REPLACE`, `PREVIEW`, `HUMANIZE`, `QUANTIZE`.
* [ ] CLI commands `:ai`, `:ai var`, `:ai describe`, with selected-track targeting & playhead insertion.
* [ ] Fallback pattern generators (lofi/rock/house), chord engine, bass follower.
* [ ] Caching, validation, error repair pass; offline `--test` deterministic clips.

If you want, I can spit out the exact **system prompt**, a **Rust skeleton** for `AI.MUSIC.*` + validators, and a few **Basil demo scripts** that wire straight into your transport.


# ANYWAY ... Where was I? Oh yeah. Feature Object --feature obj-ai ...

> That's going to be off the chain! Anyway, coming back down to earth, please provide me a prompt that I can give Junie
> to create your AI parts 1 and 2 all in one go. I'd ask you do try it yourself, but she's right there inside the Rust IDE
> with the code and can already has her hands dirty. (LOL). Give me a consice detailed prompt for creating the obj-ai
> feature up through part 2 but stopping short of the "Tool / Function-calling (Phase 2+)" features. Include a reqiest to
> make some demo programs, and to create an instruction file for getting the necessary pre-requites (API key, and
> whatever) to use the feature in a markdown file.


Here’s a tight, copy-pasteable prompt for Junie. It implements **obj-ai (Phase 1 & 2)** end-to-end (no tool/function-calling), plus demos and setup docs.

---

# Prompt for Junie: Implement `obj-ai` (Phase 1 & 2)

You’re working inside our Rust codebase for the Basil BASIC interpreter/VM. Create a new **feature object library** `obj-ai` that exposes AI chat, streaming, embeddings, moderation, caching, and a CLI AI mode. **Do Phase 1 and Phase 2 only.** **Do not** implement tool/function-calling.

## Goals

1. Beginner-friendly, one-line calls: `AI.CHAT$()`, `AI.STREAM()`.
2. Practical knobs via a forgiving JSON-ish options string.
3. Deterministic offline/test behavior (`--test`) + clear error reporting.
4. Optional: small cost/rate limits and response caching.

## Public Basil API (surface)

Implement these functions and behaviors:

* `AI.CHAT$(prompt$[, opts$]) -> string`

    * Synchronous request; returns model text.
    * On transport/model error: return `""` and set `AI.LAST_ERROR$`.

* `AI.STREAM(prompt$[, opts$]) -> string`

    * Stream tokens to STDOUT as they arrive (append-print).
    * Return the full concatenated text on completion.
    * On error: print nothing further, return `""`, set `AI.LAST_ERROR$`.

* `AI.EMBED(text$[, opts$]) -> float[]`

    * Returns a 1-D vector (embedding).
    * If unsupported model or error: return empty array and set `AI.LAST_ERROR$`.

* `AI.MODERATE%(text$[, opts$]) -> int`

    * Returns `0` if OK / `1` if flagged (simple boolean).
    * On error: return `1` and set `AI.LAST_ERROR$`.

* `AI.LAST_ERROR$ -> string`

    * Holds last error message; cleared at start of each call.

### Options string (`opts$`)

Accept a permissive JSON-ish string (single quotes OK, unquoted keys OK, trailing commas OK). Recognize:

```
model            (string)  e.g., 'gpt-4o-mini' (set sensible default)
system           (string)  system prompt
temperature      (float)   default 0.3
max_tokens       (int)     default 400
top_p            (float)   optional
stop             (string or [string]) optional
cache            (bool)    default true
timeout_ms       (int)     default 60000
```

Unknown keys are ignored (future-proof). For `AI.EMBED`, allow `embed_model`.

## Configuration

* Read from `.basil-ai.toml` at project root or XDG config dir. Keys:

  ```toml
  api_key = "env:OPENAI_API_KEY"   # if starts with env:, read from env var
  default_model = "gpt-4o-mini"
  temperature = 0.3
  max_tokens = 400
  cache = true
  max_tokens_per_run = 8000       # guardrail
  max_requests_per_min = 30       # guardrail
  ```
* If no key found and **not** in `--test`, fail gracefully (empty string/vector; set `AI.LAST_ERROR$` to “missing API key”).
* Respect environment var override: `OPENAI_API_KEY`.

## CLI AI Mode

Add `basilc --ai` REPL:

* Plain input → `AI.CHAT$`.
* `:sys <text>` set session system prompt.
* `:model <name>` change model.
* `:code <ask>` request code; print fenced block `basil … `
* `:explain <file>[:line[-end]]` loads code and asks model to explain briefly.
* `:save <path>` save last reply to file.
* `:quit` exit.
  Streaming: show tokens as they arrive; keep a rolling status line.

## Runtime / Rust Implementation

Create a crate/module `obj-ai` behind feature flag `obj-ai`:

* `config.rs` — load/merge env + `.basil-ai.toml`.
* `http.rs` — minimal HTTPS client with streaming (SSE or chunked); robust timeouts/retries (exp backoff on 429/5xx).
* `options.rs` — permissive options parser → strongly-typed struct.
* `chat.rs` — `chat(prompt, opts)` + `stream(prompt, opts)`.
* `embeddings.rs` — `embed(text, opts)`.
* `moderation.rs` — `moderate(text, opts)` boolean.
* `cache.rs` — content-addressed cache: SHA256(prompt + opts + model + endpoint) → JSON blob on disk (e.g., `.basil/ai-cache/`).
* `errors.rs` — normalized error types to fill `AI.LAST_ERROR$`.
* `ffi.rs` — wire to Basil VM: expose the BASIC functions above.
* `repl_ai.rs` — implements `--ai` mode.

### Networking details (generic)

* Use a standard chat-completions endpoint that supports streaming; JSON body with `{model, messages:[{role:'system'},{role:'user'}], temperature, max_tokens, top_p, stop}`; capture streamed chunks, concatenate, and also print token text as it arrives.
* Embeddings endpoint: `{model, input:text}` → vector of floats.
* Moderation endpoint: `{input:text}` → boolean flag. Keep it minimal.

*(Keep provider-specific details abstracted in `http.rs`; read base URL and paths from config constants so we can swap providers easily.)*

## Behavior Requirements

* **Caching**: If `cache=true`, serve from cache first; otherwise call API and write cache. For `STREAM`, still print from cache instantly if present.
* **Rate limits**: naive token bucket or simple sleep on X requests/min.
* **Test mode**: if Basil is in `--test` or `TEST_MODE=1`:

    * `AI.CHAT$` returns deterministic canned text: `"[[TEST]] " + SHA1(prompt)[0..8]`.
    * `AI.STREAM` prints the same in 3–5 chunks.
    * `AI.EMBED` returns a fixed-seed pseudo-vector (length 16).
    * `AI.MODERATE%` returns `0` unless input contains `"FLAG_ME"`.
* **Error handling**: Network/model errors → `AI.LAST_ERROR$` with concise reason; no panics. Truncate outputs to `max_tokens_per_run`.

## Security & Safety

* Never echo API key in logs.
* Redact secrets in errors.
* Optional content filter: before sending, if `AI.MODERATE%` would flag it (local heuristic), short-circuit with a brief safe message (config-controlled).

## Tests

* Unit tests for: options parser (permissive cases), cache key stability, error propagation, test-mode determinism, and CLI parsing of `--ai`.
* Integration test: run `AI.CHAT$` in test mode; assert deterministic outputs; check stream concatenation equals chat text.

## Demos (create under `/examples/obj-ai/`)

1. `01_hello_ai.basil`

   ```basic
   PRINT AI.CHAT$("Explain bubble sort in 3 bullets");
   ```
2. `02_stream_joke.basil`

   ```basic
   PRINT "AI says: ";
   DIM full$ = AI.STREAM("Tell a one-liner about BASIC", "{ temperature:0.2 }");
   PRINT "\n---\n"; PRINT full$;
   ```
3. `03_explain_file.basil`

   ```basic
   DIM src$ = FILE.READ$("examples/hello.basil");
   PRINT AI.CHAT$("Explain briefly what this code does:\n\n" + src$, _
                  "{ system:'You are a calm Basil tutor.', max_tokens:300 }");
   ```
4. `04_embeddings_search.basil` (toy local semantic search)

   ```basic
   DIM docs$[] = ["open a file", "write a file", "arrays", "loops"];
   DIM vecs[][]  ' compute vectors
   FOR i%=0 TO UBOUND(docs$[])
     vecs[i%][] = AI.EMBED(docs$[i%]);
   NEXT
   DIM q$ = "how do I write to a file?";
   DIM qv[] = AI.EMBED(q$);
   DIM best% = COSINE.MAX_SIMILARITY_INDEX(qv[], vecs[][]);
   PRINT "Closest doc: "; PRINT docs$[best%];
   ```
5. `05_moderation_gate.basil`

   ```basic
   IF AI.MODERATE%("Generate a polite meeting request email") = 0 THEN
     PRINT AI.CHAT$("Write a polite 3-sentence meeting request.");
   ELSE
     PRINT "Request blocked by moderation.";
   END IF
   ```

## CLI Docs

* Add `docs/obj-ai/README.md` with:

    * **Overview** of functions, examples of `opts$` usage.
    * **Setup** section:

        * Create API key (describe environment variable and `.basil-ai.toml`).
        * Minimal snippet of `.basil-ai.toml`:

          ```toml
          api_key = "env:OPENAI_API_KEY"
          default_model = "gpt-4o-mini"
          temperature = 0.3
          max_tokens = 400
          cache = true
          ```
        * How to run demos:
          `cargo run -q -p basilc --features obj-ai -- run examples/obj-ai/01_hello_ai.basil`
        * Using the **AI REPL**: `cargo run -q -p basilc --features obj-ai -- --ai`

            * Commands: `:sys`, `:model`, `:code`, `:explain`, `:save`, `:quit`
    * **Offline/Test mode** notes and expected deterministic outputs.
    * **Troubleshooting** (missing key, proxy/network, 429 backoff).

* Add a short top-level `SETUP_AI.md` (root) with the **quickstart**:

    1. Set `OPENAI_API_KEY` env var.
    2. (Optional) Create `.basil-ai.toml` with defaults above.
    3. Build with feature: `--features obj-ai`.
    4. Run demos; try `--ai` REPL.

## Acceptance Criteria

* Building with `--features obj-ai` compiles cleanly on Linux/macOS.
* All five demos run; in `--test` they produce deterministic outputs.
* `AI.CHAT$`, `AI.STREAM`, `AI.EMBED`, `AI.MODERATE%`, `AI.LAST_ERROR$` behave exactly as specified.
* Caching reduces second-call latency.
* Missing/invalid key yields graceful empty result + `AI.LAST_ERROR$`.
* README and SETUP files exist, are concise, and copy-paste runnable.

---

END OF PROMPT.

### Implement now. Stop short of tool/function-calling. Keep the code clean, modular, and documented inline.


> Nice! Thank you!


You got it! 🙌
If you want, I can also prep:

* a sister prompt for **AI.MUSIC + DAWG** (ClipSpec schema + validators + demos),
* a **.basil-ai.toml** starter with safe defaults,
* or a tiny **smoke-test script** that validates `AI.CHAT$`, `STREAM`, `EMBED`, and `MODERATE%` in `--test` mode.

Tell me which one you want first and I’ll spit it out.

## Phase 2b: here’s a drop-in starter config and a tiny smoke test

Awesome—here’s a drop-in starter config and a tiny smoke test.

# `.basil-ai.toml` (safe defaults)

```toml
# Where to get your key:
#  - Preferred: set environment variable OPENAI_API_KEY
#  - Or put the literal key here (not recommended)
api_key = "env:OPENAI_API_KEY"

# Sensible defaults (tweak per project)
default_model = "gpt-4o-mini"
temperature = 0.3
max_tokens = 400
cache = true

# Guardrails (prevent surprise spend)
max_tokens_per_run = 8000
max_requests_per_min = 30

# Networking (override if needed)
# base_url = "https://api.openai.com/v1"
# chat_path = "/chat/completions"
# embeddings_path = "/embeddings"
# moderation_path = "/moderations"
timeout_ms = 60000
```

# `examples/obj-ai/00_smoke_test.basil`

Validates `AI.CHAT$`, `AI.STREAM`, `AI.EMBED`, and `AI.MODERATE%` in **test mode**.

```basic
REM ------------------------------------------------------------
REM obj-ai smoke test (run in TEST mode)
REM Expects deterministic outputs with --test or TEST_MODE=1
REM ------------------------------------------------------------

FUNCTION Assert%(cond%, msg$)
  IF cond% THEN
    PRINT "✅ "; PRINT msg$;
    PRINT
    RETURN 1
  ELSE
    PRINT "❌ FAIL: "; PRINT msg$;
    PRINT
    STOP
  END IF
END FUNCTION

REM 1) CHAT
DIM reply$ = AI.CHAT$("Hello from smoke test", "{ temperature:0.2 }")
Assert%(LEN(reply$) > 0, "AI.CHAT$ returned text")
Assert%(LEFT$(reply$, 8) = "[[TEST]]", "AI.CHAT$ produced deterministic test prefix")
Assert%(LEN(AI.LAST_ERROR$) = 0, "AI.LAST_ERROR$ empty after CHAT")

REM 2) STREAM
PRINT "Streaming: ";
DIM full$ = AI.STREAM("Stream a tiny line", "{ temperature:0.0 }")
PRINT
Assert%(LEN(full$) > 0, "AI.STREAM returned text")
Assert%(LEFT$(full$, 8) = "[[TEST]]", "AI.STREAM produced deterministic test prefix")
Assert%(LEN(AI.LAST_ERROR$) = 0, "AI.LAST_ERROR$ empty after STREAM")

REM 3) EMBEDDINGS (expect fixed-size vector in test mode, e.g., 16)
DIM v[] = AI.EMBED("embed this text")
Assert%(UBOUND(v[]) + 1 > 0, "AI.EMBED returned a vector")
Assert%(UBOUND(v[]) + 1 = 16, "AI.EMBED vector length == 16 (test mode)")
Assert%(LEN(AI.LAST_ERROR$) = 0, "AI.LAST_ERROR$ empty after EMBED")

REM 4) MODERATION (0 = OK, 1 = flagged)
DIM ok% = AI.MODERATE%("Write a friendly greeting.")
Assert%(ok% = 0, "AI.MODERATE% returns 0 for safe text")
DIM flagged% = AI.MODERATE%("FLAG_ME")
Assert%(flagged% = 1, "AI.MODERATE% returns 1 for flagged text")
Assert%(LEN(AI.LAST_ERROR$) = 0, "AI.LAST_ERROR$ empty after MODERATE")

PRINT
PRINT "🎉 obj-ai smoke test passed."
```

## How to run

```bash
# 1) Build with obj-ai
cargo run -q -p basilc --features obj-ai -- --version

# 2) Test mode (either way works)
# a) CLI flag:
cargo run -q -p basilc --features obj-ai -- --test run examples/obj-ai/00_smoke_test.basil

# b) Env var:
# TEST_MODE=1 cargo run -q -p basilc --features obj-ai -- run examples/obj-ai/00_smoke_test.basil
```



# Phase 3: Tool/Function-calling (later)



# Phase 4: AI-powered music generation (AI.MUSIC + DAWG - ClipSpec schema + validators + demos)

If you want, I can now whip up the **AI.MUSIC + DAWG** prompt pack (ClipSpec schema + validators + demos).



> Ok, let's whip up the AI.MUSIC + DAWG prompt pack now (who's sleepy now??)



Heck yeah—here’s a tight, copy-pasteable **Junie prompt pack** to wire up **AI.MUSIC + DAWG**. It includes the ClipSpec schema, validators, Basil/Rust surfaces, CLI commands, fallbacks, demos, and acceptance tests. Paste this into Junie inside your DAWG/Basil repo.

---

# Prompt for Junie: Implement `AI.MUSIC + DAWG` (Phase 1)

You’re working inside our Rust DAW (“DAWG”) + Basil interpreter repo. Implement a music-aware AI bridge that turns **natural language → structured MIDI clips** and drops them into the DAWG timeline. Use our existing transport, piano-roll, and MIDI emit logic. Build on the `obj-ai` feature you just created (chat + streaming).

**Do not** implement external tool/function-calling; this is pure prompt→JSON→MIDI with local validators and fallbacks.

## High-level Goals

* One-liner UX for users: “make 4 bars of lofi drums @85 BPM” → clip appears and plays.
* Deterministic offline behavior in `--test` (no network).
* Strict schema: AI returns a compact **ClipSpec** JSON only. We validate, fix, and insert.
* Resilient: if model output is malformed or API missing, fall back to built-in generators.

---

## Namespaces & Surfaces

### Basil API (public)

Provide these functions behind features `obj-ai,obj-midi`:

* `AI.MUSIC.GENERATE$(prompt$[, opts$]) -> string`
  Returns **ClipSpec** as a JSON-ish string (forgiving parser: single quotes/trailing commas ok).

* `AI.MUSIC.VARIATION$(clip_spec$[, prompt_or_opts$]) -> string`
  Takes an existing ClipSpec (as string) and returns a modified ClipSpec (e.g., “sparser hats”, “double-time feel bar 4”).

* `AI.MUSIC.DESCRIBE$(clip_spec$) -> string`
  Human-readable summary (“4 bars Am, swing 54%, hats 16ths, snare fill bar 4”).

* `MIDI.CLIP.FROM_SPEC(clip_spec$) -> int`
  Build an internal clip, return `clip_id%`.

* `MIDI.CLIP.INSERT clip_id%, track_or_name$, start_bar%`

* `MIDI.CLIP.REPLACE clip_id%, track_or_name$, start_bar%`

* `MIDI.CLIP.PREVIEW clip_id%` (audition through current output device)

* `MIDI.CLIP.TO_SPEC(track_or_name$, start_bar%, bars%) -> string` (export selection to ClipSpec for editing/variation)

* Helpers (optional but useful):
  `MIDI.CLIP.HUMANIZE clip_id%, amount%` (timing/velocity jitter)
  `MIDI.CLIP.QUANTIZE clip_id%, grid_denominator%` (e.g., 16)

> Keep these consistent with existing DAWG transport: `TRANSPORT.PLAY`, `TRANSPORT.LOOP.SET start_bar%, end_bar%`.

### CLI (DAWG REPL)

Augment the REPL with:

* `:ai make <free text>` — calls `AI.MUSIC.GENERATE$` and inserts at playhead on selected track.
* `:ai var <free text>` — applies `AI.MUSIC.VARIATION$` to the selected clip.
* `:ai chords <free text>` — convenience alias for chord tracks.
* `:ai describe` — describe the selected clip.
* `:ai humanize <pct>` — call local humanize.
* `:ai quantize <grid>` — call local quantize.

Stream token progress to a status line (use `obj-ai` streaming if available; otherwise spinner).

---

## ClipSpec (the contract)

Strict but compact, beats-based, 1-based bars. This is the **only** format AI should output.

```json
{
  "version": 1,
  "bars": 4,                 // required, integer >0
  "tempo": 85,               // optional; if omitted, use project tempo
  "key": "Am",               // optional; MIDI drum clips may omit
  "track": "Drums",          // optional hint; we may override via target track
  "swing": 0.54,             // optional [0..0.75], applied to 8th/16th grid
  "grid": 16,                // optional quantization grid (4,8,16,32); default 16
  "notes": [
    { "beat": 1.0, "len": 0.5, "pitch": 36, "vel": 110, "ch": 10 },
    { "beat": 2.0, "len": 0.5, "pitch": 38, "vel": 100, "ch": 10 },
    { "beat": 1.0, "len": 0.5, "pitch": 42, "vel": 70,  "ch": 10 },
    { "beat": 1.5, "len": 0.5, "pitch": 42, "vel": 64,  "ch": 10 }
  ],
  "controls": [              // optional MIDI CCs (rare, but handy)
    { "beat": 1.0, "cc": 1, "value": 40, "ch": 1 }
  ],
  "meta": {                  // optional: style tags, source prompt hash, etc.
    "style": ["lofi","hiphop"],
    "root": 57,              // optional MIDI root note for tonal clips
    "scale": "aeolian"       // optional scale name
  }
}
```

**Semantics**

* `beat` is *absolute within the clip*, starting at `1.0`.
  For 4 bars @ 4/4, valid `beat` range is `[1.0, 17.0)`.
* Default values: `len=0.5`, `vel=96`, `ch=1`.
* Drums default `ch=10` and use GM pitches (36 kick, 38 snare, 42 closed hat, 46 open hat, 49 crash, 51 ride).
* For pitched instruments, `pitch` 0–127 (we clamp).
* `grid` is used by validator to snap (if needed).

---

## System Prompt (the “brain” of AI.MUSIC)

Create a constant string used by `AI.MUSIC.GENERATE$` & `VARIATION$`:

* Role: *“You are a MIDI clip generator. You output ONLY a valid ClipSpec JSON. No commentary, no code fences.”*
* Include the ClipSpec schema, defaults, and constraints (above).
* Include a GM drum pitch map (kick 36, snare 38, ch10, etc.).
* Include examples (few-shot) of **drums** and **chords**.

**Few-shot 1: Drums (lofi 4 bars)**
User: “4 bars, 85 BPM, lofi hip hop drums, swing 54%. Sparse kick, snare on beats 2 and 4, ghost hats.”
Assistant (only JSON): *(give a minimal, valid example conforming to schema)*

**Few-shot 2: Chords (Am progression)**
User: “4 bars, key Am, i–VI–III–VII, pad, legato, soft dynamics.”
Assistant: *(valid ClipSpec with pitched notes, e.g., triads or 7ths)*

**Few-shot 3: Variation**
User: “Increase hat density by ~10% and add a snare fill on the last 2 beats of bar 4.”
Assistant: *(return ClipSpec with adjusted notes)*

---

## Validation & Repair Pipeline

Create `music/spec.rs` with:

* `parse_clip_spec(str) -> ClipSpec`
  Forgiving parser: single quotes ok, trailing commas ok. Normalize to canonical struct.

* `validate_and_fix(ClipSpec, project) -> (ClipSpec, Diagnostics)`
  Rules:

    * Clamp `bars >= 1`, `swing ∈ [0.0, 0.75]`, `grid ∈ {4,8,16,32}`.
    * Snap `beat`/`len` to grid; apply swing offsets (swing applies to off-beats of selected grid).
    * Trim notes exceeding `bars` (drop or shorten to boundary).
    * Velocity clamp `[1..127]`, default 96.
    * Pitch clamp `[0..127]`; if `track` indicates “Drums”, map unknown drum names→GM or drop.
    * If channel omitted: `10` for “Drums”, else `1`.
    * Ensure notes are sorted by `beat`.
    * Produce diagnostics: `fixed:N`, `dropped:M`, `snapped:X`.

* `to_internal_clip(ClipSpec) -> Clip` (tick-based using our PPQ)

* `from_internal_clip(Clip) -> ClipSpec` (for `MIDI.CLIP.TO_SPEC`)

**Malformed output handling**

* First attempt parse; if fails, call `AI.FIX.JSON$(raw)` (a tiny helper using the same model to “repair to nearest valid JSON of ClipSpec”) **once**.
* If still invalid: **fallback generator** (see below), and attach warning diag.

---

## Fallback Generators (no AI / test mode)

`music/fallback.rs`: deterministic, seedable (use SHA1 of prompt + project seed).

* `PATTERN.DRUMS(style, bars, swing) -> ClipSpec`
  Styles: `lofi`, `rock`, `house`. Hardcoded grids (kicks/snares/hats), swing applied.

* `CHORDS.PROG(key, roman[], bars) -> ClipSpec`
  Simple voicing engine (triads/sevenths), 1 chord per bar by default.

* `BASS.FOLLOW(chords_spec, density)`
  Root or root+fifths, optional passing tones.

These are also included (summarized) in the system prompt as **few-shot guidance**, but run locally when offline/test.

**Test mode (`--test` or `TEST_MODE=1`)**

* `AI.MUSIC.GENERATE$` and `VARIATION$` **skip API** and call fallbacks deterministically.
* Seed from `hash(prompt)` to keep outputs stable across runs.

---

## Rust Structure

* `src/ai_music/mod.rs`

    * `prompt.rs` (system prompt + few-shots)
    * `schema.rs` (ClipSpec struct + serde)
    * `parser.rs` (permissive string→ClipSpec)
    * `validator.rs` (validate_and_fix)
    * `bridge.rs` (to/from internal `Clip` structure)
    * `fallback.rs` (lofi/rock/house/chords/bass)
    * `service.rs` (`generate(prompt, opts)`, `variation(spec, prompt_or_opts)`)
* `src/midi/clip_from_spec.rs`, `clip_to_spec.rs`
* `src/repl/ai_commands.rs` (the `:ai` subcommands)

Feature flags: `obj-ai,obj-midi` required for build. Ensure graceful “feature missing” errors.

---

## Options (`opts$`) for AI.MUSIC calls

Accept the same permissive JSON-ish as obj-ai:

```
model        (string) default from obj-ai
temperature  (float)  default 0.3 (music often benefits from 0.2–0.4)
max_tokens   (int)    default 600
cache        (bool)   default true
grid         (int)    optional (4,8,16,32) force quantization grid
swing        (float)  optional overrides prompt
track        (string) optional target (“Drums”, “Bass”, “Pads”, …)
key          (string) optional (e.g., “Am”, “C# dorian”)
```

Unknown keys ignored.

---

## Demos (place under `examples/ai-music/`)

1. `01_make_lofi_drums.basil`

```basic
DIM ask$ = "4 bars, 85 BPM, lofi hip hop drums, swing 54%, sparse kicks, ghost hats.";
DIM spec$ = AI.MUSIC.GENERATE$(ask$, "{ grid:16 }");
DIM id% = MIDI.CLIP.FROM_SPEC(spec$)
MIDI.CLIP.INSERT id%, "Drums", 1
TRANSPORT.LOOP.SET 1, 5
TRANSPORT.PLAY
```

2. `02_chords_and_bass.basil`

```basic
DIM chords$ = AI.MUSIC.GENERATE$("4 bars, key Am, i–VI–III–VII, pad, legato, soft.", "{}");
DIM c_id% = MIDI.CLIP.FROM_SPEC(chords$)
MIDI.CLIP.INSERT c_id%, "Pads", 1

DIM bass$ = AI.MUSIC.VARIATION$(chords$, "Generate bass that follows roots with occasional passing tones; 8ths; mellow.")
DIM b_id% = MIDI.CLIP.FROM_SPEC(bass$)
MIDI.CLIP.INSERT b_id%, "Bass", 1

TRANSPORT.LOOP.SET 1, 5
TRANSPORT.PLAY
```

3. `03_make_busier_and_fill.basil`

```basic
DIM d$ = AI.MUSIC.GENERATE$("4 bars, 92 BPM, boom-bap drums, 16th hats.", "{}")
DIM id% = MIDI.CLIP.FROM_SPEC(d$)
MIDI.CLIP.INSERT id%, "Drums", 1

DIM v$ = AI.MUSIC.VARIATION$(d$, "Increase hat density by 10% and add snare fill on last 2 beats of bar 4.")
DIM vid% = MIDI.CLIP.FROM_SPEC(v$)
MIDI.CLIP.REPLACE vid%, "Drums", 1
```

4. `04_describe_clip.basil`

```basic
DIM spec$ = MIDI.CLIP.TO_SPEC("Drums", 1, 4)
PRINT AI.MUSIC.DESCRIBE$(spec$)
```

5. `05_fallback_offline.basil`

```basic
REM Run with --test to force local generator.
DIM ask$ = "4 bars, 120 BPM, house drums (kicks on every beat), open hats on offbeats."
DIM spec$ = AI.MUSIC.GENERATE$(ask$)
DIM id% = MIDI.CLIP.FROM_SPEC(spec$)
MIDI.CLIP.INSERT id%, "Drums", 1
TRANSPORT.PLAY
```

---

## README (`docs/ai-music/README.md`)

Include:

* What AI.MUSIC does and the ClipSpec schema (copy from above).
* Quickstart:

  ```bash
  cargo run -q -p basilc --features obj-ai,obj-midi -- run examples/ai-music/01_make_lofi_drums.basil
  ```
* How to use the REPL:

  ```
  cargo run -q -p basilc --features obj-ai,obj-midi -- --ai
  :ai make 4 bars lofi 85bpm swing 54, sparse kicks, ghost hats
  :ai var busier hats + snare fill bar 4
  :ai describe
  ```
* Offline/test mode behavior & deterministic outputs.
* Troubleshooting (missing API key, malformed JSON → fallback used).

---

## Acceptance Criteria

* **Schema**: `ClipSpec` parsed, validated, and converted to internal clip.
* **Generate/Variation**: Work online (model) and offline (`--test`) deterministically.
* **Insertion**: `MIDI.CLIP.FROM_SPEC` + `INSERT/REPLACE` integrate with piano-roll and transport.
* **Swing/Quantize**: Validator snaps to grid and applies swing to off-beats.
* **Drums**: Defaults to GM pitches & ch10 when unspecified.
* **Demos**: All five examples run; in `--test` they produce consistent, musical (if simple) clips.
* **CLI**: `:ai make/var/describe` functional with selected track and playhead.
* **Docs**: README present with schema, quickstart, and REPL usage.

---

## Implementation Notes

* Use the existing `obj-ai` to call chat; set system prompt to **“ONLY return ClipSpec JSON”** (no backticks, no prose).
* In `service.rs`, keep a short retry (once) on 429/5xx; otherwise fall back.
* Cache AI responses keyed by `hash(prompt + opts + project tempo + selected track)` to reduce repeat cost.
* Diagnostics can be printed to status line (e.g., “snapped=12, dropped=1, swing=0.54”).
* Keep token budgets small (`max_tokens=600`) and `temperature≈0.3` by default.

---

> Build this now. Ensure the model never emits code fences or commentary—ClipSpec JSON only. If anything is invalid, repair once; otherwise use a fallback pattern. Keep the UX snappy and deterministic in test mode.



# Phase 3: Tool/Function-calling (later)



# Phase 4: AI-powered music generation (AI.MUSIC + DAWG - ClipSpec schema + validators + demos)







