# Basil obj-ai: AI chat, streaming, embeddings, moderation

This feature object provides beginner‑friendly AI helpers you can call directly from Basil BASIC programs:

- AI.CHAT$(prompt$[, conversation_id$[, opts$]]) -> string
- AI.STREAM(prompt$[, opts$]) -> string  (streams tokens to STDOUT while building the full reply)
- AI.EMBED(text$[, opts$]) -> float[]    (1‑D embedding vector)
- AI.MODERATE%(text$[, opts$]) -> int    (0 = OK, 1 = flagged)
- AI.CONVERSATION.CREATE$([opts$]) -> string (returns a new conversation ID)
- AI.CONVERSATION.DELETE(id$)            (cleanup conversation server-side or locally)
- AI.CONVERSATION.LIST$() -> string      (returns a JSON list of active conversation IDs)
- AI.LAST_RESPONSE_ID$() -> string       (ID of the last response for turn-chaining)
- AI.LAST_CONVERSATION_ID$() -> string   (ID of the last conversation used)
- AI.LAST_META$() -> string              (JSON metadata of the last call)
- AI.LAST_ERROR$ -> string               (last error message)

It’s designed to be simple by default, with practical knobs via a forgiving JSON‑ish options string.

Note: In offline/test mode, all functions return deterministic results (see Test mode below), making CI and demos fast and stable.


## Options string (opts$)

The optional opts$ accepts a permissive JSON‑ish string (single quotes OK, unquoted keys OK, trailing commas OK). Recognized keys:

- model (string) default comes from config (e.g., 'gpt-4o-mini')
- system (string)
- temperature (float) default 0.3
- max_tokens (int) default 400
- store (bool) whether to store the response for chaining
- previous_response_id (string) for turn-chaining without conversation objects
- metadata (object) optional key-value pairs
- top_p (float) optional
- stop (string or [string]) optional
- cache (bool) default true
- timeout_ms (int) default 60000

Unknown keys are ignored for forward compatibility. For AI.EMBED, you may also pass embed_model.

## Conversations and Persistence

Basil now supports persistent conversations across sessions and devices.

### Server-side (OpenAI) Persistence
Create a conversation ID and pass it to subsequent `AI.CHAT$` calls. OpenAI will maintain the full message history server-side.

```basil
id$ = AI.CONVERSATION.CREATE$()
PRINT AI.CHAT$("My name is Basil.", id$)
PRINT AI.CHAT$("What is my name?", id$)
AI.CONVERSATION.DELETE(id$)
```

### Local Persistence
If you prefer to keep message history on your own machine, use the `local` backend.

```basil
id$ = AI.CONVERSATION.CREATE$("{\"backend\":\"local\"}")
PRINT AI.CHAT$("This history is stored in .basil/ai/conversations/", id$)
```

### Turn-Chaining (Stateless)
You can also chain messages without a formal conversation object by using `previous_response_id`.

```basil
PRINT AI.CHAT$("Think of a secret word.")
resp_id$ = AI.LAST_RESPONSE_ID$()
PRINT AI.CHAT$("What was that word?", "", "{\"previous_response_id\":\"" + resp_id$ + "\"}")
```

Note: `conversation_id` and `previous_response_id` are mutually exclusive.

## Helper Functions

- `AI.LAST_RESPONSE_ID$()`: Returns the ID of the last response (e.g., `resp_...`). Useful for chaining.
- `AI.LAST_CONVERSATION_ID$()`: Returns the last conversation ID used.
- `AI.LAST_META$()`: Returns a JSON string with metadata about the last call, including model used and IDs.

## Privacy Note
Conversation mode stores state server-side at OpenAI until `AI.CONVERSATION.DELETE` is called. For sensitive data, prefer using the local backend or stateless chaining.

Examples:

- AI.CHAT$("Explain bubble sort in 3 bullets")
- AI.CHAT$("Write a haiku about BASIC", "{ temperature: 0.5, max_tokens: 100 }")
- AI.STREAM("Summarize QUICKLY", "{ system:'Be terse', model:'gpt-4o-mini' }")


## Configuration

The library reads configuration from environment and optionally a .basil-ai.toml at your project root (or XDG config dir).

Example .basil-ai.toml:

```
api_key = "env:OPENAI_API_KEY"   # if starts with env:, read from env var
default_model = "gpt-4o-mini"
temperature = 0.3
max_tokens = 400
cache = true
max_tokens_per_run = 8000
max_requests_per_min = 30
```

Environment override: set OPENAI_API_KEY directly to take precedence.

If no key is available and you’re not in test mode, requests will fail gracefully (empty string/vector) and AI.LAST_ERROR$ will contain "missing API key".


## Demos

Build with the AI feature and run the demos:

- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\01_hello_ai.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\02_stream_joke.basil
- cargo run -q -p basilc --features obj-ai -- run examples\ai\01_conversation.basil
- cargo run -q -p basilc --features obj-ai -- run examples\ai\02_chaining.basil

Tip: To get deterministic outputs locally without hitting any network, either:

- Run via test harness: cargo run -q -p basilc --features obj-ai -- test examples\obj-ai\01_hello_ai.basil
- Or set TEST_MODE=1 in your environment before running.


## AI REPL

You can start an interactive AI REPL that streams tokens as they arrive:

- cargo run -q -p basilc --features obj-ai -- --ai

Commands:

- :sys <text>     set session system prompt
- :model <name>   change model
- :code <ask>     request code; prints a fenced basil block
- :explain <file>[:line[-end]]  load code and ask the model to explain briefly
- :save <path>    save last reply to a file
- :quit           exit


## Test mode (deterministic offline behavior)

When Basil is in test mode (basilc test …) or TEST_MODE=1 is set:

- AI.CHAT$ → returns "[[TEST]] " + SHA1(prompt)[0..8]
- AI.STREAM → prints the same in 3 chunks and returns the full text
- AI.EMBED → returns a fixed‑seed pseudo‑vector of length 16
- AI.MODERATE% → returns 0 unless input contains "FLAG_ME"

This enables fast, stable CI and allows you to develop without an internet connection or API key.


## Caching

If cache=true (default), a content‑addressed cache is used to avoid repeated calls. For STREAM, cached results are printed instantly.

Cache path: .basil/ai-cache/


## Troubleshooting

- Empty string or vector, AI.LAST_ERROR$ = "missing API key":
  - Ensure OPENAI_API_KEY is set, or .basil-ai.toml points to it via env:OPENAI_API_KEY.
  - If you just want deterministic offline results, run in test mode.
- Network/proxy issues:
  - Ensure your environment allows outbound HTTPS to api.openai.com. If you need a proxy, set HTTPS_PROXY/HTTP_PROXY.
- http 429 (rate limit or quota):
  - Meaning: Your request was received, but refused due to rate limiting or account quota/billing limits.
  - Common causes:
    - Too many requests per minute or tokens per minute for your org/model.
    - No active billing or exhausted credits.
  - What to do:
    - Reduce request rate, batch less, or use a smaller model (e.g., gpt-4o-mini).
    - Implement retries with exponential backoff (coming to library; for now just re-run after a short wait).
    - Check usage and limits in your OpenAI dashboard and ensure billing is set up.
- http 401 (unauthorized):
  - Check that OPENAI_API_KEY is valid and not expired/revoked.
- http 5xx (server error):
  - Temporary server-side issues. Wait and retry; consider exponential backoff.

## License

MIT License. Copyright (c) Blackrush LLC.
