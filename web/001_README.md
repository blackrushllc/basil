# 1) Architecture at a glance

* **Backend (Basil/Rust):** Basil web app serving REST/JSON endpoints + server-rendered pages where handy. Use a
  relational DB (Postgres) for the 30k+ rows and related tables. Read-replica for analytics.
* **Frontend (HTML5/CSS/JS):** vanilla components + a lightweight charting lib (e.g. Chart.js) and a small fetch helper.
  No framework required.
* **File storage:** documents/forms to object storage (e.g., S3-compatible) with signed URLs.
* **AI layer:** text-to-SQL + summaries over **read-only** data with strict guardrails, using OpenAI via your `obj-ai`
  feature (or a self-hosted model later).
* **Security:** auth (session or JWT), RBAC, audit logs, per-row scoping (e.g., by reseller/firm), PHI/PII handling.

# 2) Data: from flat file → relational model

* **Ingest once, then automate:** Write an import script that:

    * Validates rows, trims/normalizes enums (“No Show”, “Missing Info”, …), dedupes people.
    * Splits to tables: `customers`, `appointments`, `insurances`, `chiropractors`, `doctors`, `attorneys`, `employees`,
      `resellers`, `carriers`, `documents`, `messages`, `flags`, `waivers`, `logs`.
    * Adds indexes you’ll actually use (date, firm, status, last_updated).
* **Keys & history:** stable customer_id; soft deletes; `updated_at`; optional event log for status changes.
* **Search:** GIN trigram or full-text for names/claims; composite indexes for dashboards.

# 3) Backend services (Basil)

* **Core REST endpoints** (JSON):

    * `/api/customers?filters…&page…`
    * `/api/appointments?date_range…&status…`
    * `/api/analytics/kpis?from…&to…` (counts, sums, rates)
    * `/api/export/csv?...` (async job → signed URL)
    * `/api/docs/signed-url?id=…` (time-boxed access)
* **Auth & RBAC:** admin, analyst, intake, attorney, reseller. Enforce row-level constraints in SQL and again in code.
* **Auditing:** record who ran which query/export; attach AI prompts used.
* **Performance:** pagination (cursor or keyset), response caching of common KPIs, background workers for heavy exports.

# 4) The AI/query experience (safe & useful)

* **Two UX modes**

    1. **Guided builder:** filters + columns + sort → instant table/chart. (No LLM needed.)
    2. **Ask in English:** “Show no-shows in Jan with missing paperwork by chiropractor, bar chart.”
* **Text-to-SQL pipeline**

    * **Intent parse → SQL draft → safety checks → approve/execute (read-only).**
    * Safety checks: allow-list tables/columns, **deny list PII** columns for free-text, mandatory `LIMIT` (with
      user-controlled “show more”), timeouts, and EXPLAIN plan sanity checks for big scans.
    * Show the generated SQL (read-only viewer) so the owner can trust results.
* **Summaries/explanations:** Model turns resultsets into bullet summaries; never send raw PII unless user opted in.
* **Caching:** hash(prompt+filters) → cache key; invalidate on data updates.

# 5) Frontend (plain, fast, printable)

* **Pages**

    * Home: KPIs (total evals, EMC approval rate, no-show rate, AR/referral owed, trendlines).
    * Explore: table view with saved filters, column picker, quick exports.
    * Ask AI: prompt box → “SQL preview + answer + chart/table.”
    * Records: customer detail drawer (profile, appointments, documents, flags).
* **Charts & tables**

    * Tables: virtualized scrolling, sticky header, CSV/Excel export.
    * Charts: line (trends), bar (grouped counts), pie (rare), stacked bar (status by firm).
* **Accessibility & print:** semantic HTML, keyboard nav, print styles for exports.

# 6) Security, privacy, compliance

* **Data class:** Even if not treatment data, treat as sensitive. Minimal exposure to LLM:

    * Prefer **aggregates** / masked fields in prompts.
    * De-identify where possible; never send SSN, DOB, claim numbers unless explicitly requested with elevated role.
* **Network:** server-side calls only; no keys in browser; per-request allow-listing for AI features.
* **Logging:** redact PII in logs; store prompt→SQL mapping and row counts only.
* **Backups/DR:** encrypted at rest; tested restore.

# 7) Exports and documents

* **Async exports:** queue job, progress indicator, email + signed URL on completion.
* **Documents:** upload → virus scan → store; tag to customer/case; render thumbnails; access via time-limited links.

# 8) Phased delivery plan

**Phase 0 – Foundations (1–2 weeks)**

* Postgres schema + indices; import 30k rows; S3 bucket; Basil project skeleton; auth + RBAC; seed data.

**Phase 1 – Owner productivity (2–3 weeks)**

* Explore page (filters, saved views); KPIs dashboard; exports; documents + signed URLs.

**Phase 2 – AI assist (2–3 weeks)**

* Wire `obj-ai` (chat + streaming). Add **text-to-SQL** with allow-listed schema, SQL preview, strict guards.
  Summaries + chart suggestions. Caching.

**Phase 3 – Safety + polish (1–2 weeks)**

* Row-level security rules; audit trails; rate limits; timeout guards; error UX; print/export polish.

**Phase 4 – Nice-to-haves**

* Scheduled reports, saved questions, anomaly alerts (“spike in no-shows”), attorney/chiro portals.

# 9) Minimal tech choices

* DB: PostgreSQL (RLS optional).
* Storage: S3-compatible (minio/dev, AWS in prod).
* Charts: Chart.js.
* Styling: utility CSS (e.g., Tailwind) or a tiny design system.
* AI: OpenAI via `obj-ai`; swap later if needed.

# 10) “Day-1” acceptance tests (define now)

* Import completes with 0 fatal errors; referential integrity holds.
* Explore page returns filtered rows <500ms for indexed queries.
* Dashboard KPIs match hand-checked SQL.
* AI “Show no-shows last 30 days grouped by chiropractor”:

    * Shows SQL preview with `LIMIT`.
    * Returns correct table + chart.
    * No PII leaked in the prompt payload.
* Export produces a downloadable CSV within a minute for 30k rows.

If this plan feels right, I’ll turn it into concrete tasks (schema sketch, API routes, AI guardrails, and the first UI
wireframe) and we can start building.
