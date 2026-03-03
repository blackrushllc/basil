Junie Prompt: Phase 2 Wiring (TeleEMC Dashboard) — /api/query + /api/export + Guardrails

Goal
Implement “Phase 2 wiring” for the TeleEMC dashboard backend:
- Map POST /api/query -> Basil CGI route that:
  1) authenticates a session token,
  2) turns natural language into SAFE read-only MySQL SQL via obj-ai (AI.CHAT$),
  3) validates SQL with guardrails (no DML/DDL, no sensitive columns, no SELECT *, enforce LIMIT, enforce soft deletes),
  4) executes SQL using DB_MYSQL (Query$ / QueryTable$),
  5) returns JSON shaped for the UI windows/artifacts model.
- Map POST /api/export -> Basil CGI route that:
  1) authenticates,
  2) takes artifact_id from a previous /api/query,
  3) exports as csv/text now (pdf stub ok),
  4) returns appropriate Content-Type / download headers.

Important constraints (must implement)
A) Never allow mutation statements or multiple statements. Only SELECT/WITH. Block keywords like INSERT/UPDATE/DELETE/DROP/ALTER/CREATE/TRUNCATE/GRANT/REVOKE, INTO OUTFILE, LOAD_FILE, INFORMATION_SCHEMA access, etc.
B) Never expose credentials/sensitive fields. Must block at least:
ecw_password, ecw_username, gmail_password, gmail_username, apple_password, apple_id, remember_token, any column containing “password”, “passwd”, “secret”, “token”, “api_key”, “key”, “credential”.
C) Soft deletes: always include:
- patients.deleted_at IS NULL
- attorney_masters.deleted_at IS NULL
  (and any other *deleted_at* tables you discover)
  D) Enforce row limits (default 200, max 1000) and query timeout (~30s) on DB connector.
  E) The LLM must ONLY generate a plan (SQL + params + explanation). NEVER include actual DB row data in the prompt. DB results stay local.

Repo integration notes
- Follow the existing /api/website CGI patterns for reading request body, env vars, and emitting headers.
- Use Basil modules:
  - obj-ai (AI.CHAT$, AI.MODERATE%)
  - obj-sql-mysql (DB_MYSQL)
  - obj-json if available; otherwise implement minimal JSON parsing for our known shapes.
  - obj-csv optional; QueryTable$ is acceptable for CSV.
- Use deterministic offline behavior when running “basilc test …” (AI functions return deterministic values); add smoke tests that run in test mode.

Deliverables: create/modify these files (exactly)
1) web/api/teleemc-dashboard/cgi/api/query.basil
2) web/api/teleemc-dashboard/cgi/api/export.basil
3) web/api/teleemc-dashboard/cgi/lib/teleemc_config.basil
4) web/api/teleemc-dashboard/cgi/lib/teleemc_auth.basil
5) web/api/teleemc-dashboard/cgi/lib/teleemc_cgi.basil
6) web/api/teleemc-dashboard/cgi/lib/teleemc_ai_sql.basil
7) web/api/teleemc-dashboard/cgi/lib/teleemc_sql_guard.basil
8) web/api/teleemc-dashboard/docs/TELEEMC_SCHEMA_QUICKREF.md
9) web/api/teleemc-dashboard/docs/PHASE2_WIRING.md
10) examples/teleemc/01_smoketest_api_plan.basil
11) examples/teleemc/02_smoketest_endpoints.md  (curl/http examples, no secrets)

-----------------------------------------------------------------------
File 8: web/api/teleemc-dashboard/docs/TELEEMC_SCHEMA_QUICKREF.md
Create a compact schema doc (do NOT try to list 200 columns yet; we’ll expand later). Must include at least:

- users
  - PK id; unique email
  - relationship hints: users links to patients via fields like user_id, owner_id, atty_id, chiro_id, ortho_id (confirm from schema later)
- patients (soft delete: deleted_at)
  - PK id
  - relationship hints:
    - pip_atty -> attorney_masters.id
    - chiroid -> chiropractors.id
    - homevisit_providerid -> providers.id (if providers table exists)
  - “critical fields” list: balance, amount_billed, payment1-4, date1-4, status
- chiropractors
  - PK id
  - status (1 = active)
  - sensitive fields: ecw_password, ecw_username, apple_password, gmail_password (explicitly warn: NEVER SELECT)
- attorney_masters (soft delete: deleted_at)
  - PK id
  - status (1 = active)
  - sensitive fields: ecw_password, ecw_username (explicitly warn: NEVER SELECT)
  - note multiple contact points (primary attorney, co-counsel, paralegals)

Also add a short “Allowed query patterns” section:
- Always specify explicit column list (no SELECT *).
- Use JOINs with ON.
- For “counts” use COUNT(*).
- For date ranges use CURDATE(), DATE_SUB, DATEDIFF, etc.
- Always include deleted_at IS NULL where applicable.

-----------------------------------------------------------------------
File 3: web/api/teleemc-dashboard/cgi/lib/teleemc_config.basil
Provide config helpers. Implement these functions (stubs + real logic):

#USE (whatever module directive Basil uses), keep consistent with repo patterns.

FUNC EnvOr$(key$, fallback$)
' returns env var if set else fallback
END FUNCTION

FUNC TeleemcDsn$()
' Prefer TELEEMC_DSN env var (mysql://user:pass@host:3306/slim?ssl-mode=REQUIRED)
' Else return "mysql://teleemc_readonly:CHANGE_ME@localhost:3306/slim?ssl-mode=DISABLED" as dev placeholder
END FUNCTION

FUNC TeleemcRootCertPath$()
' Optional: TELEEMC_RDS_CA_PATH; empty if none
END FUNCTION

FUNC TeleemcMaxRows%()
' default 200, cap 1000; env TELEEMC_MAX_ROWS optional
END FUNCTION

FUNC TeleemcApiToken$()
' env TELEEMC_API_TOKEN; if empty, use a hardcoded dev token "dev-teleemc-token"
END FUNCTION

FUNC TeleemcSchemaText$()
' Read schema quickref file text. Prefer AI.KNOWLEDGE$(path$) if available, else read file via FOPEN/FREADLINE.
' Path: web/api/teleemc-dashboard/docs/TELEEMC_SCHEMA_QUICKREF.md
END FUNCTION

FUNC TeleemcAiModel$()
' optional env TELEEMC_AI_MODEL else use obj-ai default model
END FUNCTION

-----------------------------------------------------------------------
File 4: web/api/teleemc-dashboard/cgi/lib/teleemc_auth.basil
Implement simple auth (Phase 2 = hard gate, not fancy):
- Accept header Authorization: Bearer <token> OR cookie teleemc_token=<token>.
- Compare to TeleemcApiToken$().

Implement:

FUNC GetHeader$(name$)
' read CGI env HTTP_<NAME> style; search existing CGI helper patterns in repo
END FUNCTION

FUNC GetCookie$(name$)
' parse HTTP_COOKIE if present
END FUNCTION

FUNC RequireAuth%()
' returns 1 if authed else 0
END FUNCTION

SUB EmitAuthError()
' 401 JSON: { ok:false, error:"unauthorized" }
END SUB

-----------------------------------------------------------------------
File 5: web/api/teleemc-dashboard/cgi/lib/teleemc_cgi.basil
Central CGI helpers used by both endpoints.

Implement:
FUNC ReadRequestBody$()
' Read stdin content based on CONTENT_LENGTH (CGI). Follow existing patterns in /api/website.
END FUNCTION

FUNC ContentType$()
' env CONTENT_TYPE
END FUNCTION

SUB SendJson(status%, json$)
' prints Status:, Content-Type: application/json, no-cache headers
' then prints json$
END SUB

SUB SendText(status%, contentType$, text$)
' generic
END SUB

SUB SendDownload(status%, contentType$, filename$, bytes$)
' add Content-Disposition: attachment; filename="..."
END SUB

FUNC JsonGetString$(json$, key$)
' Minimal JSON getter for top-level string keys if obj-json APIs are unclear.
' If obj-json exists, replace with real parsing.
END FUNCTION

FUNC JsonGetInt%(json$, key%, default%)
' same idea
END FUNCTION

-----------------------------------------------------------------------
File 7: web/api/teleemc-dashboard/cgi/lib/teleemc_sql_guard.basil
SQL guardrails: strict and boring.

Implement:
FUNC NormalizeSql$(sql$)
' trim; collapse whitespace; remove trailing semicolon
END FUNCTION

FUNC IsSingleStatement%(sql$)
' reject if contains ';' after normalization or contains '--'/'/*' comments
END FUNCTION

FUNC IsReadOnlySql%(sql$)
' allow only starting with SELECT or WITH
' reject if contains forbidden keywords (case-insensitive):
' INSERT UPDATE DELETE DROP ALTER CREATE TRUNCATE GRANT REVOKE REPLACE CALL EXECUTE PREPARE DEALLOCATE
' also reject "INTO OUTFILE", "LOAD_FILE", "INFORMATION_SCHEMA", "MYSQL." etc
END FUNCTION

FUNC ContainsSensitiveCols%(sql$)
' reject if sql contains any banned column tokens:
' ecw_password ecw_username gmail_password gmail_username apple_password apple_id remember_token
' plus any substring match of "password" "passwd" "secret" "token" "api_key" "credential"
END FUNCTION

FUNC RejectSelectStar%(sql$)
' reject if matches "SELECT *" or "SELECT  *"
END FUNCTION

FUNC EnforceSoftDeletes$(sql$)
' If sql references patients (FROM patients or JOIN patients), ensure predicate "patients.deleted_at IS NULL" exists.
' If references attorney_masters, ensure "attorney_masters.deleted_at IS NULL" exists.
' If missing, inject into WHERE safely:
'   - if WHERE exists, add "AND <predicate>"
'   - else add "WHERE <predicate>"
' Must preserve existing ORDER BY / LIMIT clauses (inject before them).
END FUNCTION

FUNC EnforceLimit$(sql$, maxRows%)
' If LIMIT already present, keep but cap to maxRows% if numeric constant is greater.
' If no LIMIT, append " LIMIT <maxRows%>"
END FUNCTION

FUNC ValidateSqlOrError$(sql$)
' Return "" if ok else JSON error string describing why blocked (for UI display).
END FUNCTION

-----------------------------------------------------------------------
File 6: web/api/teleemc-dashboard/cgi/lib/teleemc_ai_sql.basil
Convert natural language -> SQL plan via obj-ai.
We are NOT implementing true OpenAI function-calling yet; instead we require the model to output strict JSON.

JSON plan schema (top-level):
{
"sql": "SELECT ... ? ...",
"params": ["...","..."],         // array of strings/numbers; keep simple
"mode": "table" | "scalar" | "summary",
"title": "short label",
"explanation": "1-3 sentences",
"columns": ["col1","col2", ...]  // optional; helps UI
}

Implement:
FUNC BuildSystemPrompt$()
' Build strict system prompt:
' - role: “You are a SQL query generator for TeleEMC MySQL.”
' - Include schema text from TeleemcSchemaText$()
' - Rules:
'   - output JSON only, no markdown, no commentary
'   - MySQL dialect; placeholders must be ? and params must be separate array
'   - no DML/DDL; no multi statements; no SELECT *; do not reference sensitive columns
'   - handle soft deletes: deleted_at IS NULL
'   - default to LIMIT <= TeleemcMaxRows%() if not specified by user
' - Remind: return columns explicit
END FUNCTION

FUNC MakeSqlPlanJson$(userPrompt$)
' 1) Optionally run AI.MODERATE% on userPrompt$ (if flagged, return JSON error plan)
' 2) Call AI.CHAT$(userPrompt$, opts$) with opts containing:
'    - system: BuildSystemPrompt$()
'    - temperature: 0.1
'    - max_tokens: ~700
' 3) Return raw model output (expected JSON)
END FUNCTION

FUNC ParseSqlPlanOrError$(planJson$)
' Validate that required keys exist; if not, return a JSON error response payload for endpoint.
' If obj-json exists, parse properly.
' If not, implement minimal extraction for keys sql/mode/title/explanation and a simple params parser
' (params can be empty array often; support that).
END FUNCTION

-----------------------------------------------------------------------
File 1: web/api/teleemc-dashboard/cgi/api/query.basil
POST /api/query
Request JSON (front end):
{
"prompt": "how many patients are 90 days past due?",
"max_rows": 200,          // optional
"format": "json"          // optional
}

Response JSON (server):
{
"ok": true,
"artifact_id": "A2026...random",
"prompt": "...",
"sql": "...",
"params": [...],
"mode": "table|scalar|summary",
"title": "...",
"explanation": "...",
"rows": [ {..}, {..} ],   // actual DB results parsed to JSON array if possible
"rows_json": "[...]"      // always include raw JSON string from DB.Query$ as fallback
}

Implementation steps:
1) RequireAuth%(), else EmitAuthError()
2) ReadRequestBody$()
3) Extract prompt:
  - If Content-Type contains json: prompt = JsonGetString$(body$, "prompt")
  - Else fallback: read from QUERY_STRING param "prompt="
4) Call MakeSqlPlanJson$(prompt$) -> planJson$
5) ParseSqlPlanOrError$(planJson$) -> a structure or extracted fields:
   sql$, params[], mode$, title$, explanation$
6) Guardrails:
  - NormalizeSql$, ValidateSqlOrError$
  - EnforceSoftDeletes$, EnforceLimit$
7) Execute on DB:
  - DIM db@ AS DB_MYSQL(TeleemcDsn$())
  - if TeleemcRootCertPath$() not empty: db@.RootCertPath$ = that
  - set db@.CommandTimeoutMs% = 30000 (or env)
  - rowsJson$ = db@.Query$(sql$, paramsArray)
  - OPTIONAL: if obj-json available, parse rowsJson$ to real JSON array; else just return rows_json
8) Persist artifact:
  - Create folder .basil/teleemc/artifacts/
  - Save <artifact_id>.json containing: prompt, sql, params, plan meta, rows_json, created_at
9) Return JSON via SendJson(200,...)

Also add explicit error shapes:
- ok:false, error:"bad_request" | "blocked" | "sql_error" | "ai_error"
- include “details” for UI display

-----------------------------------------------------------------------
File 2: web/api/teleemc-dashboard/cgi/api/export.basil
POST /api/export
Request JSON:
{
"artifact_id": "A...",
"format": "csv" | "txt" | "pdf"
}

Behavior:
- auth required
- Load artifact file from .basil/teleemc/artifacts/<id>.json
- For csv:
  - Prefer re-run query using db@.QueryTable$(sql$, params) if available (fast CSV-like lines)
  - Else: if rows_json exists, convert JSON array to CSV (simple)
  - Return download “teleemc_<artifact_id>.csv” with content type text/csv
- For txt:
  - Return explanation + sql + rows_json
- For pdf:
  - OK to stub: return 501 with message “PDF export not implemented yet” OR return simple text/pdf placeholder
- Must NOT accept raw SQL in export request; only artifact_id.

-----------------------------------------------------------------------
File 9: web/api/teleemc-dashboard/docs/PHASE2_WIRING.md
Write a clear internal doc:
- Environment vars:
  - OPENAI_API_KEY (or .basil-ai.toml api_key = env:OPENAI_API_KEY)
  - TELEEMC_DSN
  - TELEEMC_RDS_CA_PATH (optional)
  - TELEEMC_API_TOKEN
  - TELEEMC_MAX_ROWS
- How to create read-only DB user (SQL snippet; SELECT only), SSL note, timeout note.
- Apache/Nginx routing example:
  - /api/query -> cgi/api/query.basil
  - /api/export -> cgi/api/export.basil
- Security notes:
  - LLM sees ONLY schema + user prompt, not row data
  - blocked columns list
  - audit logging + artifact storage location
- How to run smoke tests in test mode.

-----------------------------------------------------------------------
File 10: examples/teleemc/01_smoketest_api_plan.basil
A test-mode script that validates core wiring pieces without any network:
- Calls BuildSystemPrompt$() and prints first 20 lines
- Calls MakeSqlPlanJson$("How many patients are 90 days past due?") and prints planJson$
- Ensures SQL guard functions run:
  - IsReadOnlySql%, RejectSelectStar%, ContainsSensitiveCols%
  - EnforceSoftDeletes$ and EnforceLimit$
- Do NOT call real DB by default; if env TELEEMC_RUN_DB=1 then run one harmless SELECT 1.

-----------------------------------------------------------------------
File 11: examples/teleemc/02_smoketest_endpoints.md
Include ready-to-copy curl examples:
- curl -X POST /api/query with Authorization bearer token
- curl -X POST /api/export with artifact_id

-----------------------------------------------------------------------
Implementation tips (do these while coding)
- Prefer DB.Query$ for JSON result output; it returns a JSON array of row objects (good for UI).
- If JSON parsing libs are present, parse and return rows as structured JSON; otherwise return rows_json string and let front-end render from it.
- Keep error handling tight using TRY/CATCH around AI + SQL calls; return ok:false with meaningful error.

Phase 3 hooks (ONLY stubs/comments, no implementation yet)
In teleemc_ai_sql.basil add a commented section describing a future tool-binding approach (function calling) where the model can:
- get_schema()
- validate_sql(sql)
- run_sql_preview(sql, params, limit)
  But DO NOT implement the tool-calling runtime in this phase.

Done criteria
- /api/query works end-to-end against mock (test mode) and optionally real DB when env vars set.
- /api/export works for csv + txt based on artifact_id.
- Guardrails demonstrably block:
  - SELECT *
  - any sensitive column
  - any non-read-only statement
  - missing soft delete predicate for patients / attorney_masters (it should be injected or blocked)
- Docs and smoke tests included.

Proceed to implement everything in one go, with clean, readable Basil code and minimal dependencies.
