# BasilBasic.com — Project Plan (Phases 1–3) and Junie Prompts

This plan turns the `examples/website` CGI demo into **basilbasic.com**, where users can log in and compile Basil programs from the browser. It’s split into phases with concrete acceptance criteria, architecture notes, file layout, security/hardening, and a sequence of **ready‑to‑paste Junie prompts**.

---

## Phase 1 — Single‑box upload → compile → zip → download

### Goals

* Auth users can upload a single `.basil` file.
* Server compiles to **bytecode (.basilx)** locally (using `bcc` or `basilc`), zips **source + bytecode**, and streams download.
* Clear, friendly UI and error feedback. Basic logging.

### Acceptance Criteria

1. Non‑logged‑in users are redirected to **login** before accessing upload/compile pages.
2. Upload only accepts files ending in `.basil` (case‑insensitive), size ≤ configurable limit (e.g., 256 KB by default).
3. On success, user gets a `project-<timestamp>.zip` with:

    * `<name>.basil` (exact uploaded source)
    * `<name>.basilx` (compiled bytecode)
    * `README.txt` (build info: compiler version, time, server, etc.)
4. On failure, user sees a styled error with suggestions.

### High‑Level Flow

1. **UI**: New page with a card that contains file input + Compile button.
2. **POST /upload** (CGI: `compile.basil`):

    * Validate session, validate file, create temp work dir per request.
    * Save source, invoke compiler, capture stdout/stderr + exit code.
    * If success: create zip and **send as download** with `Content-Type: application/zip`.
    * Cleanup temp dir (best‑effort) after streaming.

### Directory / Files (within `examples/basilbasic.com/`)

```
examples/basilbasic.com/
  index.basil
  login.basil
  logout.basil
  register.basil
  user_home.basil
  compile.basil          # NEW: POST handler that validates & compiles
  /views/
    home.html
    login.html
    register.html
    logged_in.html
    upload.html          # NEW: upload UI for authenticated users
  /css/site.css
  /js/site.js
```

### Config & Conventions

* **Compiler:** use `bcc` (preferred) or `basilc` with flags for bytecode output (Junie to wire).
* **Paths:** demo lives in a subfolder; use **relative** URLs/redirects, not root `/…`.
* **Temp Workdir:** `tmp/<epoch>-<rand>/` under the example folder or system temp. Remove on success/failure.
* **Limits:** `MAX_SIZE=262144` default; environment override via `BASIL_WEB_MAX_UPLOAD`.
* **Logging:** append to `logs/access.log` & `logs/error.log` (simple timestamped lines).

### Security / Hardening Checklist (Phase 1)

* Validate extension and **MIME sniff** (fallback to extension + size check).
* Strip paths from filename; drop everything except `[A-Za-z0-9._-]`.
* Create per‑request temp dir; never reuse; 0700 perms.
* Do **not** execute uploaded files; only compile them with controlled args.
* Enforce size limit; reject binary files > few KB with `.basil` suffix.
* Sanitize environment; pass only required vars to compiler.
* Return `Status: 400/413/500` with friendly HTML body on errors.

---

## Phase 2 — Cloud compile via SQS/S3 (multi‑platform)

### Goals

* The website enqueues the upload to **AWS SQS** with a `job_id`.
* A separate **compiler worker** (Rust or Basil‑driven runner) consumes SQS, builds:

    * Bytecode `.basilx`
    * Optional native binaries for targets (e.g., Windows, Debian) via cross‑compile containers or build agents.
* Results uploaded to **S3** as `job_id/…` and a manifest `job_id/manifest.json`.
* Website polls for completion; when ready, zips artifacts (or downloads a pre‑built zip from S3) and streams to user.

### Acceptance Criteria

1. Jobs show **Queued → Running → Completed/Failed**.
2. User can refresh or auto‑poll to retrieve artifacts.
3. Errors include worker stderr tail and status.

### Architecture Notes

* **Queue**: SQS standard queue; message contains `job_id`, s3 bucket, user id, original filename, compile flags.
* **Storage**: Upload source to S3 at `incoming/job_id/source.basil`.
* **Worker**: pulls message, downloads source, runs compilers, uploads artifacts to `results/job_id/…`, writes manifest.
* **Site**: polls S3 existence of `results/job_id/manifest.json` or via a lightweight status API.

### Security / Ops

* Dedicated IAM role limited to S3 paths + SQS queue.
* Server‑side encryption (S3 SSE‑S3 or SSE‑KMS).
* Short TTL cleanup (e.g., 24–72 hours) lifecycle rules.
* Rate limits per user; job quota; size limits as in Phase 1.

---

## Phase 3 — Minimal in‑browser Dev Environment

### Goals

* Authenticated workspace with projects (CRUD), code editor (Monaco), compile/test flow.
* Save files server‑side; allow quick build and download artifacts.
* Optional: syntax check (compiler service endpoint) and basic "Run in sandbox" (future).

### Acceptance Criteria

* Users can create project, edit `.basil` in browser, click **Compile**, and download.
* Recent builds list with statuses.

### Architecture Notes

* Add simple project DB tables (projects, files, builds).
* Reuse Phase 2 cloud compile when enabled; otherwise local Phase 1.

---

# Junie Prompts — Phase 1 (step‑by‑step)

> **Important:** We host under a subfolder (e.g., `/basil/website`). Use **relative** links and redirects.

### Step 1 — Create upload UI (`views/upload.html`) and nav link

**Prompt to Junie:**

Create a new file `examples/basilbasic.com/views/upload.html` with a clean card that lets a logged‑in user select a `.basil` file and submit to `compile.basil` via POST `multipart/form-data`. Include helper text with size limit and a small notice about privacy/security. Add a link to this page from `views/logged_in.html` ("Compile a Basil file"). Keep classes consistent with `site.css`.

### Step 2 — Add `compile.basil` CGI handler (upload, validate)

**Prompt to Junie:**

Add a new CGI script `examples/basilbasic.com/compile.basil` that:

* Requires logged‑in user; redirect to `login.basil` if missing cookie.
* Accepts `multipart/form-data` POST with file field name `source`.
* Enforces size limit `MAX_SIZE` default 262144 bytes; allow env override `BASIL_WEB_MAX_UPLOAD`.
* Validates filename and extension `.basil` (case‑insensitive), strips paths; allow only `[A-Za-z0-9._-]`.
* Creates a unique temp dir `tmp/<epoch>-<rand>/` with 0700 perms; saves the source as `main.basil`.
* On any validation error, responds `Status: 400 Bad Request` and prints a friendly HTML error body using existing layout helpers.
* Logs a line to `logs/access.log` with user, file, size, status.

### Step 3 — Wire local compilation to bytecode

**Prompt to Junie:**

Extend `compile.basil` to compile `main.basil` to bytecode `main.basilx` using **bcc** (preferred). If bcc isn’t available, fall back to `basilc` flags that produce bytecode. Requirements:

* Capture stdout/stderr and exit code.
* On non‑zero exit, return `Status: 422 Unprocessable Entity` and show the last 80 lines of stderr in a styled error block.
* On success, write a short `README.txt` with compiler version, timestamp, and command used.

### Step 4 — Zip and stream download

**Prompt to Junie:**

After successful compile, create `artifact.zip` containing `main.basil`, `main.basilx`, and `README.txt`. Then stream it as a download with headers:

* `Status: 200 OK`
* `Content-Type: application/zip`
* `Content-Disposition: attachment; filename="project-<epoch>.zip"`
  Ensure we don’t emit any extra HTML. After streaming, cleanup the temp dir best‑effort.

### Step 5 — UI polish + form/page wiring

**Prompt to Junie:**

* Add a "Compile a Basil file" button on `views/logged_in.html` and a breadcrumb back to Home.
* Create an errors/notices partial (or inline block) so validation messages render above the form in `upload.html`.
* Ensure redirects are **relative** (no leading `/`).
* Add a small line to the homepage `home.html` that describes the service.

### Step 6 — Logging + error files

**Prompt to Junie:**

* Create `logs/` if missing. Append request summaries to `logs/access.log`.
* On failures, append a one‑line error with timestamp and job temp path to `logs/error.log`.
* Add a simple utility in Basil to format timestamps consistently.

---

# Junie Prompts — Phase 2 (Cloud compile)

### Step 7 — Config plumbing for AWS

**Prompt to Junie:**

Introduce a config file `examples/basilbasic.com/.basil-cloud.toml` and env var support for:

* `AWS_REGION`, `SQS_QUEUE_URL`, `S3_BUCKET`
* optional `BASIL_TARGETS` (comma‑separated like `bytecode,linux-x64,windows-x64`)
  Add a small Basil helper module `cloud_config.basil` that reads env/ TOML and exposes getters.

### Step 8 — Enqueue job to SQS and upload source to S3

**Prompt to Junie:**

Add a new path in `compile.basil` behind a toggle `USE_CLOUD=1` that:

* Generates `job_id` (ULID or UUID).
* Uploads the uploaded `main.basil` to `s3://<bucket>/incoming/<job_id>/source.basil`.
* Sends SQS message with `job_id`, user id, original filename, targets, and bucket.
* Renders a status page with job id and auto‑refresh every 5s.

### Step 9 — Worker service skeleton (Rust or Basil)

**Prompt to Junie:**

Create a new folder `services/compiler-worker/` (Rust) that:

* Polls SQS for messages, downloads `source.basil` from S3.
* Runs compilers for targets from the message (`bytecode` required, others optional for now).
* Uploads artifacts to `s3://<bucket>/results/<job_id>/…` and writes `manifest.json` with fields: `job_id`, `status`, `artifacts[]`, `stderr_tail`, `started_at`, `finished_at`.
* Deletes SQS message on success/failure after upload.
  Provide a `README.md` with build/run instructions and IAM policy snippet.

### Step 10 — Status polling page and final download

**Prompt to Junie:**

Add a new page `views/job_status.html` and a CGI `job_status.basil` that:

* Accepts `job_id` query param.
* Checks S3 for `results/<job_id>/manifest.json`.
* Shows status; if completed, provide a **Download** button that fetches a pre‑built zip from S3 or zips on the fly by streaming artifacts from S3 (choose simpler option).

---

# Junie Prompts — Phase 3 (Web IDE)

### Step 11 — Project + file storage

**Prompt to Junie:**

Add basic tables for `projects(id, user, name, created_at)` and `files(id, project_id, path, content, updated_at)`. Create CRUD CGI endpoints and views for listing projects and files. Auth‑guard everything.

### Step 12 — Browser editor (Monaco) + compile button

**Prompt to Junie:**

Add an editor page using Monaco (or CodeMirror if simpler) that loads/saves a `.basil` file. Provide **Compile** button that either triggers Phase 1 local or Phase 2 cloud compile depending on config. Show build output inline and offer **Download Zip**.

### Step 13 — Recent builds + retention

**Prompt to Junie:**

Add a minimal builds table (project_id, job_id, status, created_at). Show recent builds on project page with links to artifacts (local disk or S3). Add cleanup routines and a small admin page for housekeeping.

---

## Testing Checklist (Phase 1 focus)

* Upload with valid `.basil` under size limit → success download.
* Upload with wrong extension → 400 + error.
* Upload oversized → 413 + error.
* Simulate compiler error (bad syntax) → 422 with stderr tail.
* Confirm no absolute redirects; all relative.
* Verify temp dirs removed and logs written.

## Operational Notes

* Put `bcc`/`basilc` on PATH for the CGI runtime user; test permissions.
* Consider ulimit/timeouts on compiler process.
* Add simple rate limit (per IP/user) to avoid abuse.

---

## Nice‑to‑have polish

* Show compiler version on the upload page.
* Dark/light theme toggle.
* Remember last filename (not content) in local storage.
* Accessible form labels, focus states, and keyboard flow.
