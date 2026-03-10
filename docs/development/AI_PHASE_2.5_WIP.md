You are Junie working inside the RustRover project `basil` (Rust workspace). Please extend the Basil `obj-ai` feature library to support **persistent conversation state** across sessions/devices and to support **turn-chaining** via `previous_response_id`.

# Goals (Basil `obj-ai` Enhancements)

## 1) Conversation IDs for AI.CHAT$
Update `AI.CHAT$` to support an OPTIONAL `conversation_id$` argument.

- If `conversation_id$` is provided and non-empty:
  - Prefer the OpenAI **Conversations API + Responses API** to maintain state server-side.
  - This provides persistence across sessions/devices without re-sending full message history.
- If `conversation_id$` is omitted/empty:
  - Keep existing behavior (stateless) unless `previous_response_id` is provided (see below).

### Signature / Backward compatibility
Keep existing call sites working.

Implement overloading in the Basil binding layer so these all work:

1) `AI.CHAT$(prompt$)`  (existing)
2) `AI.CHAT$(prompt$, conversation_id$)`
3) `AI.CHAT$(prompt$, conversation_id$, options_json$)`  (recommended)
4) (Optional convenience) Allow `conversation_id` to also be specified inside `options_json$` for forwards compatibility.

### Options JSON (existing + new)
Parse `options_json$` as JSON (serde_json) and support at least:
- `model` (string)
- `instructions` (string) developer/system message
- `temperature` (number, optional if supported by current code)
- `max_output_tokens` (number)
- `store` (boolean) – pass through to Responses API when applicable
- NEW: `previous_response_id` (string) – for chaining without conversation objects
- NEW: `metadata` (object<string,string>) – optional pass-through if convenient

IMPORTANT RULE:
- If `conversation_id$` is provided, DO NOT send `previous_response_id` to OpenAI (the Responses API treats them as mutually exclusive). In this case:
  - If user provided both, throw a Basil exception with a clear message.

## 2) New Basil functions for conversation management
Add these functions to the `AI` object:

- `AI.CONVERSATION.CREATE$()`: returns a new `conv_...` conversation id
  - Default behavior: create an OpenAI Conversation via POST /conversations
  - Also record it in a local registry (see below) so LIST works.
  - Optional (nice-to-have): allow `AI.CONVERSATION.CREATE$(options_json$)` with `{ "backend": "openai" | "local" }`.
    - If omitted, default to `"openai"` when OPENAI_API_KEY is set, else fallback to `"local"`.

- `AI.CONVERSATION.DELETE(id$)`: cleanup
  - Remove from local registry.
  - If it’s an OpenAI conversation (`id$` looks like `conv_...` and backend is openai):
    - Call DELETE /conversations/{conversation_id}
    - NOTE: OpenAI delete does not delete items. Add a “best-effort deep cleanup”:
      - GET /conversations/{id}/items (paginate with `after` if `has_more`)
      - DELETE /conversations/{id}/items/{item_id} for each
      - Then DELETE /conversations/{id}
    - If deep cleanup fails partway, still attempt the conversation delete, and raise a warning/error message that some items may remain.

- `AI.CONVERSATION.LIST$()`: returns a JSON list of active conversation IDs for the current user
  - Implement by reading the local registry file (see below).
  - Return format (simple): a JSON array of strings, e.g. `["conv_abc", "conv_def"]`

## 3) Turn-chaining via previous_response_id
Support `previous_response_id` inside `AI.CHAT$` options:
- If present (and `conversation_id$` is empty):
  - Call the Responses API with `previous_response_id` and the new user input.
- After each `AI.CHAT$` call (any mode), store metadata for follow-up calls:
  - Add `AI.LAST_RESPONSE_ID$()` -> returns last response id (e.g., `resp_...`)
  - Add `AI.LAST_CONVERSATION_ID$()` -> returns last conversation id used (or empty string)
  - Add `AI.LAST_META$()` -> returns JSON with at least `{ "response_id": "...", "conversation_id": "...", "model": "..."}`
These are essential so Basil programs can easily chain.

# Implementation Details

## A) Locate existing obj-ai code
- Find the `obj-ai` crate (likely under `crates/obj-ai` or `basil-objects` or similar).
- Identify where AI functions are registered/bound (AI.CHAT$ is already there).
- Follow existing patterns for:
  - HTTP calls
  - async execution / tokio runtime usage
  - error mapping into Basil exceptions
  - feature flags and workspace conventions (prefer rustls; no openssl)

## B) OpenAI API integration (Responses + Conversations)
Implement minimal endpoints with reqwest (rustls):
- `POST https://api.openai.com/v1/conversations`  -> create conversation (returns `{ id: "conv_..." }`)
- `DELETE https://api.openai.com/v1/conversations/{conversation_id}`
- `GET https://api.openai.com/v1/conversations/{conversation_id}/items?limit=100&order=desc&after=...`
- `DELETE https://api.openai.com/v1/conversations/{conversation_id}/items/{item_id}`
- `POST https://api.openai.com/v1/responses`
  - Stateless mode:
    - body includes `model`, `input` (string or message array), optional `instructions`, optional `store`
  - Chaining mode:
    - same as stateless plus `previous_response_id`
  - Conversation mode:
    - same as stateless plus `conversation: "conv_..."`

Response parsing:
- Prefer `output_text` if present; otherwise extract from `output` items.

Auth:
- Use `OPENAI_API_KEY` (existing approach).
- Optional: support `OPENAI_BASE_URL` for testing.

## C) Local registry for LIST and durability across sessions
Create a small registry stored in the OS config dir:
- Use the existing Basil config dir conventions if present; otherwise use `directories::ProjectDirs`.
- Path suggestion:
  - `<config_dir>/basil/ai/conversations.json`

Registry schema (keep simple and forward-compatible):
```json
{
  "version": 1,
  "conversations": [
    {
      "id": "conv_...",
      "backend": "openai",
      "created_at": "RFC3339",
      "last_used_at": "RFC3339"
    }
  ]
}
```

Requirements:

* Atomic writes (write temp + rename).
* Basic file lock to avoid corruption on concurrent processes.
* Update `last_used_at` on each `AI.CHAT$` call that references a conversation.

## D) “Local backend” (optional but requested)

Implement a local backend that persists message history to disk and replays it:

* Conversation IDs can still be `conv_local_<uuid>` (starts with `conv_`).
* Store per conversation file:

  * `<config_dir>/basil/ai/conversations/<id>.json`
  * Contains an array of message items `{ role, content }`
* On `AI.CHAT$` with local conversation:

  * Load history, append new user message, call Responses API with `input: history`, then append assistant output, save.

This provides persistence across sessions; “across devices” is achievable only if the user syncs that folder (fine to document).

# Developer UX / Docs

## 1) Update docs

* Update the Basil docs page for `obj-ai` (where AI.CHAT$ is documented) to include:

  * conversation usage
  * previous_response_id usage
  * new LAST_* helper functions
  * privacy note: conversation mode stores state server-side until deleted (so prefer local/previous_response_id for sensitive data)

## 2) Add examples

Create example(s) under `/examples/ai/`:

### Example 1: conversation mode

* Create conv
* Ask it to remember something
* Ask it later in same conv
* Print last ids
* Delete conv

### Example 2: previous_response_id chaining

* Call AI.CHAT$ once
* Get AI.LAST_RESPONSE_ID$()
* Call AI.CHAT$ again with options containing previous_response_id

# Testing

## Unit tests

* Registry read/write + atomic update
* ID parsing / backend resolution
* Options JSON parsing and validation (cannot mix conversation_id and previous_response_id)

## Integration tests (gated)

* If OPENAI_API_KEY is set, run a tiny test:

  * create conv, chat twice, delete conv
* Otherwise, skip with a clear message.

# Acceptance Criteria

* Existing `AI.CHAT$` usage continues to work unchanged.
* `AI.CONVERSATION.CREATE$`, `DELETE`, `LIST$` available and documented.
* `AI.LAST_RESPONSE_ID$`, `AI.LAST_CONVERSATION_ID$`, `AI.LAST_META$` implemented and reliable.
* Conversation mode works across separate Basil runs (persistence).
* previous_response_id chaining works when conversation_id is not provided.
* Clean error messages and Basil exceptions for invalid usage.


**[1]: https://developers.openai.com/api/docs/guides/conversation-state/ "Conversation state | OpenAI API"
[2]: https://developers.openai.com/api/docs/assistants/migration/?utm_source=chatgpt.com "Assistants migration guide | OpenAI API"**
