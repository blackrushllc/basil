# Basil obj-ai: AI chat, streaming, embeddings, moderation

This feature object provides beginner‑friendly AI helpers you can call directly from Basil BASIC programs:

- AI.CHAT$(prompt$[, opts$]) -> string
- AI.STREAM(prompt$[, opts$]) -> string  (streams tokens to STDOUT while building the full reply)
- AI.EMBED(text$[, opts$]) -> float[]    (1‑D embedding vector)
- AI.MODERATE%(text$[, opts$]) -> int    (0 = OK, 1 = flagged)
- AI.LAST_ERROR$ -> string               (last error message)

It’s designed to be simple by default, with practical knobs via a forgiving JSON‑ish options string.

Note: In offline/test mode, all functions return deterministic results (see Test mode below), making CI and demos fast and stable.


## Options string (opts$)

The optional opts$ accepts a permissive JSON‑ish string (single quotes OK, unquoted keys OK, trailing commas OK). Recognized keys:

- model (string) default comes from config (e.g., 'gpt-4o-mini')
- system (string)
- temperature (float) default 0.3
- max_tokens (int) default 400
- top_p (float) optional
- stop (string or [string]) optional
- cache (bool) default true
- timeout_ms (int) default 60000

Unknown keys are ignored for forward compatibility. For AI.EMBED, you may also pass embed_model.

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
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\03_explain_file.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\04_embeddings_search.basil
- cargo run -q -p basilc --features obj-ai -- run examples\obj-ai\05_moderation_gate.basil

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
  - For now the HTTP provider is kept minimal; ensure your environment allows outbound HTTPS.
- 429/5xx backoff:
  - Minimal exponential backoff is planned; in test mode this isn’t applicable.


## License

MIT License. Copyright (c) Blackrush LLC.
