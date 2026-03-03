I read the researcher’s TeleEMC report and the Basil object docs you attached, and below is a **Phase 2 wiring plan** that’s detailed enough for Junie to implement: `/api/query` → Basil CGI route → SQL generation → guardrails → execution → results back to the desktop windows. I’ll also recommend targeted upgrades to `obj-ai` (including the specific Phase 3 tool-binding pieces that matter most for this dashboard).

Key constraints from the report:

* The `patients` table is a **single big denormalized table** and is the primary source of truth.
* There are **soft deletes** (`deleted_at IS NULL` must be applied).
* There are **highly sensitive credential fields** (e.g., `ecw_username/password`, `gmail_password`, etc.) in `chiropractors` and `attorney_masters` that must never be exposed.
* Researcher recommends **structured output/function calling** for SQL generation (we’ll do “structured JSON” now; tool calling is the best upgrade).

---

## 1) Phase 2 goal

Implement a backend “Query Service” used by the UI windows:

* `POST /api/query` → returns an **Artifact**:

    * `kind`: table | chart | summary | report
    * `title`
    * `result` payload
    * `sqlPreview` (safe)
    * `diagnostics` (timing, row counts, cache)

* `POST /api/export` + `GET /api/export/status` → async export jobs (CSV/TXT now; PDF can be placeholder until server-side PDF is added)

This matches the UI’s “window artifacts” model you’re building.

---

## 2) Backend file layout (Basil CGI)

Create these Basil CGI files (paths assume your existing Basil web folder conventions; adjust if your repo uses `/cgi/`):

```
/cgi/api/query.basil
/cgi/api/export.basil
/cgi/api/export_status.basil

/cgi/lib/api_base.basil            ' auth, json parsing, responses
/cgi/lib/query_service.basil       ' orchestration: prompt → sql → run → format
/cgi/lib/sql_guardrails.basil      ' validation + rewriting (limit, soft delete)
/cgi/lib/schema_context.basil      ' schema dictionaries + sensitive lists
/cgi/lib/result_formatters.basil   ' table/chart/summary/report format
/cgi/lib/audit_log.basil           ' append-only log rows
/cgi/lib/export_jobs.basil         ' job store + worker-ish polling approach

/docs/teleemc_schema_context.md    ' curated schema context for AI prompts
/docs/teleemc_sensitive_fields.md  ' denylist + masking rules
```

You’ll use Basil’s SQL connectors for MySQL/Postgres (`DB_MYSQL`/`DB_POSTGRES`) with pooled connections and timeouts.
If you choose to use ORM for some internal queries/logging/saved artifacts, the ORM is available too.

Basil language and `obj-ai` calling patterns (including `AI.KNOWLEDGE$` to load schema prompt context) are documented in the lexicon.
The current `obj-ai` mod capabilities and deterministic test mode will be very helpful for local dev.

---

## 3) API contracts (what Basil must implement)

### 3.1 `POST /api/query`

**Request JSON**

```json
{
  "prompt": "no-shows last 30 days by chiropractor",
  "kindHint": "table|chart|summary|report",
  "context": {
    "dateRangeDays": 30,
    "reseller": "All",
    "status": "All"
  },
  "limit": 200,
  "offset": 0,
  "includePII": false,
  "debug": false
}
```

**Response JSON**

```json
{
  "ok": true,
  "artifact": {
    "id": "art_20260218_abcdef",
    "title": "No-shows by chiropractor (last 30 days)",
    "kind": "table",
    "result": { "kind":"table", "columns":[...], "rows":[...], "rowCount": 57 },
    "sqlPreview": "SELECT ... LIMIT 200",
    "diagnostics": { "tookMs": 182, "cached": false, "rowCount": 57 }
  }
}
```

**Error response**

```json
{ "ok": false, "error": "Explainable message for UI", "details": { ...optional... } }
```

### 3.2 `POST /api/export`

Request:

```json
{
  "prompt": "...",
  "context": { ... },
  "exportType": "csv|txt|pdf",
  "includePII": false
}
```

Response:

```json
{
  "ok": true,
  "job": {
    "jobId": "exp_123",
    "status": "queued",
    "progress": 0.0,
    "downloadUrl": null,
    "message": "Queued"
  }
}
```

### 3.3 `GET /api/export/status?jobId=exp_123`

Response:

```json
{
  "ok": true,
  "job": {
    "jobId": "exp_123",
    "status": "running|done|error",
    "progress": 0.45,
    "downloadUrl": "/downloads/exp_123.csv",
    "message": "..."
  }
}
```

---

## 4) Query pipeline (end-to-end)

### Step A — Auth + parse

In `api_base.basil`:

* Validate session token (for now, your hardcoded login token)
* Parse JSON request body
* Normalize context (`dateRangeDays`, reseller/status filters)
* Normalize limits (cap `limit` to e.g. 500; default 200)

### Step B — Build AI prompt context (schema + rules)

Create a **curated schema context** file (not full DDL dumps). The researcher’s report gives the core tables and key fields, which is a great starting point.

In `schema_context.basil`, provide:

* Table summaries: `patients`, `users`, `chiropractors`, `attorney_masters`
* Relationship hints: e.g. `patients.pip_atty → attorney_masters.id`, `patients.chiroid → chiropractors.id`
* “Soft delete rule” and “must not expose credential fields”
* Common query patterns (the report includes a lot of them—use those as few-shot examples)

Also define:

* **Sensitive field denylist** (credentials + other PII)

    * Credentials: `password`, `remember_token`, `ecw_username`, `ecw_password`, `gmail_username`, `gmail_password`, `apple_id`, `apple_password` etc.
    * PII: `dob`, `claim_no`, `policy_no`, full address, phone, email (unless includePII=true)
* **Allowed tables**: start with those four + any additional “slim db” tables you choose to add later.

### Step C — Decide query kind (table/chart/summary/report)

Do this with a deterministic heuristic first, and optionally ask AI to classify.

Heuristic examples:

* contains “trend”, “over time”, “per week” → `chart`
* contains “how many”, “total”, “sum”, “count” → `summary` (or single-row table)
* contains “list”, “show me”, “patients with” → `table`
* contains “report” → `report`

Store `kind` in the Artifact response; it drives the UI renderer.

### Step D — Generate structured “SQL Plan” (JSON) using AI

Because `obj-ai` Phase 1/2 doesn’t include tool calling yet, we’ll use **strict JSON output**.

Ask the model to output exactly:

```json
{
  "title": "...",
  "kind": "table|chart|summary|report",
  "sql": "SELECT ...",
  "params": ["..."],          // optional
  "columns": [
    {"key":"...", "label":"...", "type":"number|string|date"}
  ],
  "chart": { ...optional... }, // if kind=chart
  "notes": "1-2 sentence explanation"
}
```

**Important:** even though the report says “parameterized queries,” GPT sometimes returns literals. We can support both:

* Prefer placeholders + params.
* If literals appear, we still run through guardrails and refuse anything risky.

You can also cache “SQL Plan” results by `(prompt + context + includePII)`.

### Step E — Guardrails & rewriting (non-negotiable)

Implement `sql_guardrails.basil` that:

1. **Read-only enforcement**
   Reject if SQL contains dangerous keywords (the report lists the exact family: `DROP`, `DELETE`, `UPDATE`, `INSERT`, `ALTER`, `CREATE`, `TRUNCATE`, `GRANT`, `REVOKE`).

2. **Single statement only**

* Reject if multiple statements detected (e.g., `;` used mid-query)
* Reject `UNION` if you want stricter early behavior (optional)

3. **Table allowlist**

* Only allow `patients`, `users`, `chiropractors`, `attorney_masters` initially
* Reject `information_schema`, `mysql.*`, etc.

4. **Sensitive column denylist**

* If SQL selects any credential fields, reject hard (never allow). The report explicitly calls out credentials stored in `chiropractors` and `attorney_masters`.
* If includePII=false, also block DOB, claim/policy numbers, email/phone/address.

5. **Soft delete**

* If query touches soft-delete tables (patients, attorney_masters), ensure `deleted_at IS NULL` is present; if missing, **inject it** safely.

    * The report stresses `deleted_at` in patients and attorney_masters.

6. **Limit enforcement**

* If missing LIMIT, append `LIMIT <cap>`
* If present but too high, clamp to max.

7. **Timeout**

* Set connector `CommandTimeoutMs%` (e.g. 30s) and fail gracefully.

8. **Result filtering (double layer)**
   Even if the SQL passed, strip sensitive fields from returned rows before sending to UI.

### Step F — Execute safely using DB connector

Use `DB_MYSQL` (report database is MySQL/RDS “slim”).
Connector usage & timeouts are documented.

Execution plan:

* Run with parameters if present (preferred)
* Get JSON rows via `Query$`
* Convert to a typed response (columns + rows)
* Optionally run a separate `COUNT(*)` query for total rows (later; not required for MVP)

### Step G — Format result to Artifact

In `result_formatters.basil`:

* Table: columns + rows
* Chart: translate grouped table to `{labels, series}`
* Summary: either

    * server-generated summary for single aggregates
    * or AI summary **over a small safe slice** (aggregates only, or first N rows with PII masked)
* Report: block array (summary + chart + table)

### Step H — Audit log (must-have)

Write prompt + sql hash + rowCount + tookMs + user + timestamp to an audit table or append-only log file.

The report recommends audit logging and shows how to filter sensitive fields.

---

## 5) Concrete Basil implementation notes (pseudo-code)

### 5.1 `/cgi/api/query.basil` skeleton

```basic
#USE JSON, AI, SQL_MYSQL   ' names depend on your actual #USE tokens
#INCLUDE "lib/api_base.basil"
#INCLUDE "lib/query_service.basil"

DIM req$ = API.ReadJsonBody$()
CALL API.RequireAuth()

DIM resp$ = QueryService.HandleQuery$(req$)
API.JsonOk resp$
```

### 5.2 `QueryService.HandleQuery$` (core orchestrator)

* Parse request
* Build schema context string (from file + constants)
* Generate SQL plan (AI.CHAT$)
* Validate/repair JSON
* Guardrails validate+rewrite SQL
* Execute query
* Format Artifact JSON response
* Audit log

### 5.3 Generating schema context for AI

You already have `AI.KNOWLEDGE$` in your Basil lexicon; use it to load `docs/teleemc_schema_context.md` and keep prompts consistent.

---

## 6) Guardrails: exact denylist starter (from report)

Start with the report’s explicit sensitive fields and expand:

**Credential denylist (hard reject always)**

* `password`, `remember_token`
* `ecw_username`, `ecw_password`
* `gmail_username`, `gmail_password`
* `apple_id`, `apple_password`
  These are explicitly called out as critical risks in the report.

**PII denylist (reject unless includePII=true)**

* `dob`, `email`, `phone*`, `address*`, `claim_no`, `policy_no` (patients table includes these)

---

## 7) Export implementation (Phase 2)

Implement export by reusing the exact same `/api/query` pipeline (same SQL plan, same guardrails). Do **not** allow export to bypass rules.

* CSV:

    * If table result: render header + rows to CSV
* TXT:

    * prompt + explanation + summary text
* PDF:

    * For now return a placeholder “PDF not implemented yet” file or HTML snapshot; later add a server-side HTML→PDF renderer

Store exports in:

* `/downloads/exp_123.csv` (or a temp folder)
* job state in a tiny DB table or JSON files in `.basil/exports/`

---

## 8) Recommendations to improve `obj-ai` for this project

Your existing `obj-ai` is a great Phase 1/2 base: chat, stream, embeddings, moderation, caching, deterministic test mode.
For this dashboard, the following upgrades will pay off immediately:

### 8.1 Add “strict JSON” helpers (Phase 2.5 quality-of-life)

1. `AI.CHAT_JSON$(prompt$, schema$[, opts$]) -> string`

* Wraps `AI.CHAT$` with a strict system prompt: “output JSON only”
* Validates JSON; if invalid, auto-runs **one repair pass**
* Returns canonical JSON string or `""` + `AI.LAST_ERROR$`

2. `AI.JSON_REPAIR$(bad_json$, schema$[, opts$]) -> string`

* Minimal “fix malformed JSON” helper
* This will dramatically reduce “model output broke the parser” failures in `/api/query`.

### 8.2 Add basic cost/usage introspection

For an owner-facing analytics app, it’s helpful to show:

* `AI.LAST_TOKENS_IN%`, `AI.LAST_TOKENS_OUT%`
* `AI.LAST_COST$` (approx)
* request id / cached boolean

This makes it easy to add “cost guardrails” per user/session.

### 8.3 The Phase 3 tool bindings that matter most here

Even if you “stop short” of general tool calling, these *specific tools* make your SQL pipeline much safer and more reliable:

**Tool: `generate_sql_plan`**

* Input: `{prompt, context, schema_version, includePII}`
* Output: `{sql, params, title, kind, explanation, columns}`
* This replaces “free-form JSON” with structured tool output.

**Tool: `validate_sql`**

* Input: `{sql}`
* Output: `{ok, reasons[], rewrittenSql?, appliedRules[]}`
* Lets the model participate in self-correction *without* being able to execute anything.

**Tool: `execute_readonly_sql` (optional; you can keep execution purely server-side)**

* If you implement it, it must hard-enforce:

    * allowlist tables
    * denylist columns
    * max rows
    * timeouts
    * no PII unless allowed
* But you don’t need this tool if Basil already executes SQL; it’s “nice” not required.

**Tool: `schema_lookup`**

* Input: `{table}` or `{table, column}`
* Output: metadata (safe, curated)
* This prevents the model from hallucinating fields.

The TeleEMC report explicitly points to function calling/structured output as an ideal way to do SQL generation safely.
Your Phase 3 plan we discussed earlier fits perfectly here—just keep the tool registry tight and the defaults locked down.

---

## 9) Junie implementation checklist (ordered for success)

1. **Backend scaffolding**

* Add the `/cgi/api/*.basil` files + shared libs.
* Hardcode auth token check (temporary) to match UI prototype.

2. **DB connector setup**

* Add `.teleemc-db.toml` or env-based DSN
* Use read-only DB user (SELECT only) as recommended.
* Set `CommandTimeoutMs%` and pooling defaults.

3. **Schema context files**

* Create `docs/teleemc_schema_context.md` and `docs/teleemc_sensitive_fields.md`
* Include relationship keys and “deleted_at” rules.

4. **SQL plan generation**

* Implement “structured JSON” plan via `AI.CHAT$` now.
* Add robust JSON parse/repair path (temporary in service if obj-ai doesn’t have helpers yet).

5. **Guardrails engine**

* Implement keyword blocking + allowlist + denylist + LIMIT + soft-delete injection
* Add unit tests for common bad SQL strings

6. **Result formatters**

* Table -> Artifact
* Chart -> Artifact (simple mapping)
* Summary -> Artifact (avoid PII; send aggregates)

7. **Export jobs**

* Implement job store + polling endpoint + file generation

8. **Audit log**

* Add database table or append-only file

---
