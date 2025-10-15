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

---

# Phase 1 — Step 1 & 2 Implementation (upload UI + compile handler)

Below are **drop‑in files** and minimal patches that keep the simple, same‑folder CGI layout and read HTML from `/views/`.

## 1) NEW: `views/upload.html`

```html
<!-- views/upload.html -->
<section class="wrap">
  <div class="intro">
    <h2>Compile a Basil file</h2>
    <p>Upload a single <code>.basil</code> file. We’ll compile it to bytecode and return a ZIP with your source and the <code>.basilx</code>.</p>
  </div>
  <div class="card" style="max-width:560px">
    <form action="compile.basil" method="post" enctype="multipart/form-data">
      <label for="source">Choose a .basil file</label>
      <input id="source" name="source" type="file" accept=".basil" required>
      <p class="notice" style="margin-top:.75rem">Max size: 256 KB by default. Only <code>.basil</code> files.</p>
      <div style="margin-top:1rem">
        <button class="btn" type="submit">Compile & Download ZIP</button>
        <a class="btn secondary" href="user_home.basil">Cancel</a>
      </div>
    </form>
  </div>
</section>
```

## 2) PATCH: add a link from `views/logged_in.html`

Add this button somewhere sensible (e.g., under the greeting/dashboard area):

```html
<a class="btn" href="compile.basil">Compile a Basil file</a>
```

> Note: we link directly to `compile.basil` so GET shows the upload form; POST handles the compile.

## 3) NEW: `compile.basil` (CGI handler)

This script serves the upload form on **GET** and handles upload/compile/zip/download on **POST**. It keeps **relative redirects** and assumes a few helper functions noted below. If any helper doesn’t exist yet (e.g., `FILE_EXISTS()`), Junie can wire it using Basil’s FS/CGI APIs.

```basil
' compile.basil — upload → compile to .basilx → zip → stream download
#CGI_NO_HEADER

' -----------------
' CONFIG DEFAULTS
' -----------------
LET MAX_SIZE% = 262144           ' 256 KB default (env override BASIL_WEB_MAX_UPLOAD)
LET TMP_ROOT$ = "tmp"            ' per-request subdir will be created inside here
LET ZIP_NAME$ = "project"       ' base name for the download zip

' read env override if present
IF LEN(ENV$("BASIL_WEB_MAX_UPLOAD")) > 0 THEN LET MAX_SIZE% = VAL(ENV$("BASIL_WEB_MAX_UPLOAD"))

' -----------------
' AUTH GUARD
' -----------------
IF LEN(get_cookie$("user")) == 0 THEN BEGIN
  PRINT "Status: 302 Found
"
  PRINT "Location: login.basil

"
  EXIT 0
END

' -----------------
' DISPATCH BY METHOD
' -----------------
IF UCASE$(CGI_REQUEST_METHOD$()) == "POST" THEN
  GOTO :handle_post
ELSE
  GOTO :show_form
END

:show_form
  send_header_ok_html()
  layout_start("Upload & Compile")
  ' TODO: if you want to surface a flash message, print it here
  PRINT READFILE$("views/upload.html")
  layout_end()
  EXIT 0

:handle_post
  ' 1) VALIDATE UPLOAD
  DIM err$
  LET field$ = "source"
  IF NOT CGI_HAS_FILE%(field$) THEN LET err$ = "No file uploaded."

  IF LEN(err$) == 0 THEN BEGIN
    LET fname$ = CGI_FILE_NAME$(field$)        ' original name user provided
    LET size%  = CGI_FILE_SIZE%(field$)

    IF size% <= 0 THEN LET err$ = "Empty upload."
    IF size% > MAX_SIZE% THEN LET err$ = "File too large (limit " + STR$(MAX_SIZE%) + " bytes)."

    ' enforce .basil extension, case-insensitive
    IF LEN(err$) == 0 THEN BEGIN
      LET lower$ = LCASE$(fname$)
      IF RIGHT$(lower$, 6) <> ".basil" THEN LET err$ = "Only .basil files are allowed."
    END
  END

  IF LEN(err$) > 0 THEN BEGIN
    ' 400 Bad Request
    PRINT "Status: 400 Bad Request
"
    PRINT "Content-Type: text/html; charset=utf-8

"
    layout_start("Upload error")
    PRINT "<div class=\"error\">"; HTML$(err$); "</div>"
    PRINT READFILE$("views/upload.html")
    layout_end()
    EXIT 0
  END

  ' 2) PREP TEMP WORKDIR
  LET req_id$ = gen_req_id$()
  LET workdir$ = make_workdir$(TMP_ROOT$, req_id$)      ' e.g., tmp/17300-ABCD12/
  IF LEN(workdir$) == 0 THEN BEGIN
    GOTO :fatal_500
  END

  ' sanitize basename, but we’ll save as main.basil internally
  LET base$ = safe_basename$(fname$)
  LET src_path$ = workdir$ + "main.basil"
  LET out_path$ = workdir$ + "main.basilx"
  LET readme$   = workdir$ + "README.txt"
  LET zip_path$ = workdir$ + "artifact.zip"

  ' 3) SAVE UPLOADED FILE TO workdir/main.basil
  IF CGI_SAVE_FILE%(field$, src_path$) == 0 THEN BEGIN
    LET err$ = "Failed to save uploaded file."
    GOTO :respond_500
  END

  ' 4) COMPILE TO BYTECODE (prefer bcc, fallback to basilc)
  DIM comp
  comp = run_compile%(workdir$, src_path$, out_path$, readme$)
  IF comp <> 0 OR NOT FILE_EXISTS(out_path$) THEN BEGIN
    ' 422 + show stderr tail from readme or a separate stderr file
    PRINT "Status: 422 Unprocessable Entity
"
    PRINT "Content-Type: text/html; charset=utf-8

"
    layout_start("Compile failed")
    PRINT "<div class=\"error\"><strong>Compilation failed.</strong><br>"
    PRINT "Please check your Basil syntax and try again.</div>"
    PRINT "<pre class=\"notice\">"; HTML$(tail_file$(readme$, 2000)); "</pre>"
    PRINT READFILE$("views/upload.html")
    layout_end()
    cleanup_dir(workdir$)
    EXIT 0
  END

  ' 5) CREATE ZIP: main.basil, main.basilx, README.txt
  IF create_zip%(zip_path$, src_path$, out_path$, readme$) == 0 THEN BEGIN
    LET err$ = "Failed to create ZIP."
    GOTO :respond_500
  END

  ' 6) STREAM DOWNLOAD
  LET dl_name$ = ZIP_NAME$ + "-" + STR$(EPOCH%()) + ".zip"
  PRINT "Status: 200 OK
"
  PRINT "Content-Type: application/zip
"
  PRINT "Content-Disposition: attachment; filename=\""; dl_name$; "\"

"
  SEND_FILE(zip_path$)

  cleanup_dir(workdir$)
  EXIT 0

:fatal_500
  LET err$ = "Server error (unable to prepare temp directory)."

:respond_500
  PRINT "Status: 500 Internal Server Error
"
  PRINT "Content-Type: text/html; charset=utf-8

"
  layout_start("Server error")
  PRINT "<div class=\"error\">"; HTML$(err$); "</div>"
  PRINT READFILE$("views/upload.html")
  layout_end()
  EXIT 0

' -----------------
' HELPER FUNCTIONS (assumptions allowed; Junie can wire real impls)
' -----------------
FUNCTION gen_req_id$()
  RETURN STR$(EPOCH%()) + "-" + RANDOMSTR$(6)
END

FUNCTION make_workdir$(root$, id$)
  LET path$ = root$ + "/" + id$ + "/"
  IF NOT FILE_EXISTS(root$) THEN MKDIR(root$)
  IF MKDIRS(path$) == 0 THEN RETURN ""
  RETURN path$
END

FUNCTION safe_basename$(name$)
  DIM i%: i% = LASTINDEXOF(name$, "/")
  IF i% > 0 THEN LET name$ = MID$(name$, i%+1)
  i% = LASTINDEXOF(name$, "\")
  IF i% > 0 THEN LET name$ = MID$(name$, i%+1)
  ' allow only safe chars
  RETURN REGEX_REPLACE$(name$, "[^A-Za-z0-9._-]", "_")
END

FUNCTION run_compile%(workdir$, src$, out$, readme$)
  ' Try bcc first
  DIM cmd$
  cmd$ = "bcc bc " + QUOTE$(src$) + " -o " + QUOTE$(out$)
  DIM rc%: rc% = SYS_EXEC%(cmd$, workdir$, readme$)
  IF rc% == 0 THEN RETURN 0
  ' Fallback: basilc with a bytecode flag (placeholder)
  cmd$ = "basilc --bytecode " + QUOTE$(src$) + " -o " + QUOTE$(out$)
  rc% = SYS_EXEC%(cmd$, workdir$, readme$)
  RETURN rc%
END

FUNCTION create_zip%(zip$, file1$, file2$, file3$)
  ' If Basil ships a ZIP helper, use it. Otherwise shell out.
  IF ZIP_CREATE%(zip$, file1$, file2$, file3$) == 1 THEN RETURN 1
  DIM cmd$
  cmd$ = "zip -q -j " + QUOTE$(zip$) + " " + QUOTE$(file1$) + " " + QUOTE$(file2$) + " " + QUOTE$(file3$)
  RETURN (SYS_EXEC%(cmd$, "", "") == 0)
END

FUNCTION tail_file$(path$, limit%)
  IF NOT FILE_EXISTS(path$) THEN RETURN ""
  DIM s$: s$ = READFILE$(path$)
  IF LEN(s$) <= limit% THEN RETURN s$
  RETURN RIGHT$(s$, limit%)
END

SUB cleanup_dir(path$)
  ' Best-effort remove temp dir; Junie can implement a recursive delete helper
  ' e.g., DIR_DELETE_RECURSIVE(path$)
END
```

### Notes on helpers & assumptions

* `CGI_HAS_FILE%`, `CGI_FILE_NAME$`, `CGI_FILE_SIZE%`, `CGI_SAVE_FILE%` represent reasonable CGI upload primitives. If names differ in Basil, Junie can map them.
* `SYS_EXEC%(cmd$, cwd$, tee_to$)` should run a process with optional working dir and append stdout/stderr to `tee_to$`. If not available, split into `SYS_EXEC%` and `APPENDFILE$()` calls.
* `FILE_EXISTS(path$)`, `MKDIRS(path$)`, `SEND_FILE(path$)`, `ZIP_CREATE%()` are proposed helpers; Junie can supply equivalents or small wrappers.
* We keep **relative** links and redirects to support hosting in a subfolder.

## 4) (Optional) Tiny CSS tweak for better error/notice blocks

`site.css` already includes `.error` and `.notice`. If you want a subtle border:

```css
/* optional */
.notice{border:1px solid #dbeafe}
.error{border-left:4px solid #b00020;padding-left:.8rem}
```

---

## Smoke tests to run now

1. Visit `compile.basil` while logged out → 302 to `login.basil`.
2. Logged in → upload form renders (GET).
3. Upload non-.basil file → 400 with error.
4. Upload >256KB → 413 (if you want this stricter status) or 400 with message.
5. Upload syntactically broken Basil → 422 with stderr tail.
6. Valid file → browser downloads `project-<epoch>.zip` containing `main.basil`, `main.basilx`, `README.txt`.

```
```
Added:

* `views/upload.html` (clean upload form)
* `compile.basil` (GET shows form; POST validates → compiles → zips → streams download)
* A tiny patch to `views/logged_in.html` (button link)
* Helper stubs + assumptions clearly marked so Junie can wire the actual Basil APIs

Want me to also drop in a minimal `SYS_EXEC%`/`FILE_EXISTS`/`SEND_FILE` shim file (e.g., `web_utils.basil`) to make it runnable immediately, or do you prefer Junie to connect those to Basil’s real primitives first?
