This is a Prompt to scaffold the built-in dev server. 

I kept it explicit, file-scoped, and runnable with sensible placeholders you can wire to the real VM/compiler later.

---

Title: Scaffold “basil-serve” dev web server + basil_web crate (Axum), with CGI adapter and template runner

Goal:
Create a developer-only HTTP server for Basil, usable as either a standalone binary `basil-serve` or a `basilc serve` subcommand. It serves static files, runs `.basil/.bas` scripts with CGI-style env + headers, auto-rebuilds bytecode (`.basilx`) when stale, and streams templated HTML containing `<?basil … ?>` blocks. Keep all links/redirects relative for subfolder hosting.

Assumptions:

* Repo has `basilc` (VM) and `bcc` (compiler). If not linkable as libs yet, stub thin process calls; we’ll replace with direct lib calls later.
* Rust 1.81+, Tokio, Axum 0.7+, tower-http, anyhow, clap.
* OS: Linux/macOS dev target. Windows support nice-to-have.
* This is a dev server (no TLS). Default bind 127.0.0.1.

Deliverables:

1. New crate `crates/basil_web/` (library)
2. New binary `bin/basil-serve/` (standalone CLI)
3. Optional subcommand `basilc serve …` reusing the same library
4. Minimal docs and examples
5. Basic tests

Project changes:

A) Workspace & dependencies

* Update root Cargo.toml workspace members to include:

    * `crates/basil_web`
    * `bin/basil-serve`

* `crates/basil_web/Cargo.toml`:

    * deps: axum, hyper, tokio, tokio-util, clap (derive for types only), tower, tower-http, mime_guess, percent-encoding, bytes, time, anyhow, thiserror, camino, sha2, base64 (for ETags), regex (for template scanning), notify (for --watch, optional)
    * features: `process-runner` (default) to shell out to `bcc` / `basilc` if VM/compiler libs not linked yet.

* `bin/basil-serve/Cargo.toml`:

    * deps: basil_web = { path = "../../crates/basil_web" }, clap, anyhow, tracing, tracing-subscriber

B) CLI spec (both binaries should support)

* Flags:

    * `--root <dir>` (required; e.g., ./examples/basilbasic.com)
    * `--host <ip>` default 127.0.0.1
    * `--port <u16>` default 8000
    * `--upload-limit <bytes>` default 10MB
    * `--script-timeout <secs>` default 10
    * `--watch` (optional) clear caches on file change
    * `--no-etag` disable ETags
    * `--index <name>` default index.html
    * `--bytecode-dir <dir>` optional separate cache dir (default adjacent to source)
    * `--log <level>` info|debug
* Env override allowed (BASIL_SERVE_*)

C) basil_web crate layout (files)

crates/basil_web/src/lib.rs
crates/basil_web/src/config.rs
crates/basil_web/src/server.rs
crates/basil_web/src/handlers.rs
crates/basil_web/src/static_files.rs
crates/basil_web/src/script.rs
crates/basil_web/src/template.rs
crates/basil_web/src/cgi.rs
crates/basil_web/src/compile.rs
crates/basil_web/src/util.rs
crates/basil_web/README.md

Implementations:

1. config.rs

* Define Config { root: Utf8PathBuf, host: String, port: u16, upload_limit: usize, script_timeout: Duration, watch: bool, etag: bool, index: String, bytecode_dir: Option<Utf8PathBuf>, log: Level }
* Provide `Config::from_env_and_args(args: impl IntoIterator<Item=String>) -> anyhow::Result<Self>`

2. server.rs

* `pub async fn serve(cfg: Config) -> anyhow::Result<()>`
* Build axum Router: fallback to `handlers::entry`
* If cfg.watch, start a notify::RecommendedWatcher on cfg.root and invalidate template/script caches on change
* Tracing init if not already

3. handlers.rs

* `pub async fn entry(State(app): State<AppState>, req: Request<Body>) -> Response`
* Resolve path via `static_files::resolve_path`.
* Routing:

    * If file is dir, append cfg.index.
    * If extension in { .basil, .bas } → `script::run_script`
    * If extension .html and `template::file_contains_basil` → `template::render_html_with_basil`
    * Else → `static_files::serve_file`
* HEAD support mirrors GET but with zero-length body when static
* Common error handling → 4xx/5xx HTML snippets

4. static_files.rs

* `resolve_path(root, uri_path) -> Path`
* Prevent traversal, normalize, deny symlink escape
* `serve_file` sets Content-Type via mime_guess, ETag (sha256 of mtime+size), `If-None-Match`/`If-Modified-Since`
* Byte-range not required (nice-to-have)
* Cache-control: dev-friendly no-store by default; allow `--etag` to add weak ETags for caching

5. cgi.rs

* Build CGI-like env map from axum Request:

    * REQUEST_METHOD, PATH_INFO, QUERY_STRING, CONTENT_TYPE, CONTENT_LENGTH, REMOTE_ADDR, HTTP_* headers, COOKIE
* Provide body readers:

    * For `application/x-www-form-urlencoded` and `multipart/form-data` (write to temp files under `.upload/`).
* Output handling:

    * Parse script output “header section” until blank line (`\r?\n\r?\n`)
    * Supported headers: Status, Content-Type, Set-Cookie, Location, Cache-Control; pass-through others
    * If `Location:` present and no `Status:` set → set `302 Found`
    * After header break, remaining bytes are body (streamable)
* Abort if client disconnects: add a CancellationToken and check during VM streaming

6. compile.rs

* `compile_if_stale(source: &Path, bytecode: &Path, bytecode_dir: Option<&Path>) -> anyhow::Result<()>`

    * Create bytecode path (adjacent or in bytecode_dir)
    * If bytecode missing or `mtime(source) > mtime(bytecode)` then compile
    * Two backends:

        * feature `process-runner`: shell out to `bcc` or fallback to `basilc --bytecode` with timeouts; capture stderr
        * feature `lib-runner` (future): call into `basil_compiler::compile_to_bytecode`
* Return a `CompileOutcome { changed: bool, stderr_tail: Option<String>, version: Option<String> }`

7. script.rs

* `pub async fn run_script(app: &AppState, req: Request<Body>, path: Path) -> Response`
* Steps:

    1. Compute bytecode path: `path.with_extension("basilx")` or in cfg.bytecode_dir mirrored structure
    2. `compile_if_stale`
    3. Run VM with CGI env

        * process-runner: `basilc --run <bytecode>`, send POST body via stdin; inherit env from cgi::make_env
        * lib-runner (future): `basil_vm::run(bytecode, env, io)`
    4. Capture stdout as a stream; parse headers; map to HTTP response; stream body to client
    5. Enforce timeout (tokio::time::timeout) and kill process if exceeded

8. template.rs

* `pub async fn file_contains_basil(path: &Path) -> bool` (quick scan for `<?basil`)
* `pub async fn render_html_with_basil(app, req, path) -> Response`

    * Stream the .html file
    * Simple state machine: copy bytes until `<?basil`, collect until `?>`, evaluate that code block:

        * Compile/eval block:

            * Option A: Run a tiny ephemeral script: write block to temp file `__inline_<hash>.basil`, compile to `.basilx` under `.basilcache/inline/`, run; capture stdout; inject into stream
            * Option B (future): in-memory compile/eval via VM API
    * Continue streaming remaining bytes
    * Errors: emit a visible `<pre class="error">` fragment with stderr tail
* Keep relative URLs untouched

9. util.rs

* Small helpers: etag hashing, mtime reads, percent-decoding path segments, mime sniff, stderr tail function, safe filename

10. lib.rs

* `pub struct AppState { pub cfg: Config, pub etag_enabled: bool, pub bytecode_root: Utf8PathBuf, pub cancellation: CancellationToken }`
* `pub use server::serve;`
* Re-exports of key types

D) bin/basil-serve/src/main.rs

* Parse CLI with clap, build Config, call basil_web::serve(cfg)
* Warn if host != 127.0.0.1 (“dev server; do not use in production”)
* Print listening address

E) Add `basilc serve` subcommand (optional now; nice-to-have)

* In basilc’s main.rs, add `serve` that forwards to basil_web::serve(cfg)

F) Security & behavior checklist (enforce now)

* Deny traversal and symlink escape
* Default bind 127.0.0.1; print WARN on 0.0.0.0
* Per-request upload temp dir; size limit
* Per-request timeout; kill child process on timeout
* No directory listing by default
* Relative redirects/links (no leading “/” required by server)
* Log access lines and script stderr tail to a rotating file under `logs/` (optional)

G) Tests (basic)

* Unit: path resolution, header parsing, template tokenizer
* Integration (tokio::test):

    * Serves static index.html (200)
    * Runs a trivial hello.basil producing `Content-Type: text/plain` + body
    * Redirect test: script prints `Location: user_home.basil` (maps to 302)
    * Template: page with two `<?basil print "X" ?>` blocks renders “X” twice
    * Recompile: touch source → next request recompiles

H) Example content

* Under `examples/basilbasic.com/`, add:

    * `hello.basil` simple script that prints headers + “Hello”
    * `templated.html` containing two basil blocks
    * README.md with quickstart

Skeleton code (snippets to implement):

crates/basil_web/src/server.rs

```rust
use axum::{Router, extract::State, routing::get};
use tokio::net::TcpListener;
use crate::{config::Config, handlers, AppState};
use std::sync::Arc;

pub async fn serve(cfg: Config) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new(cfg.clone()));
    let app = Router::new()
        .fallback(get(handlers::entry))
        .with_state(state.clone());
    let addr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("basil-serve listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
```

crates/basil_web/src/handlers.rs

```rust
use axum::{http::{Request, StatusCode}, response::Response, extract::State};
use bytes::Bytes;
use std::sync::Arc;
use crate::{AppState, static_files, template, script};

pub async fn entry(State(app): State<Arc<AppState>>, req: Request<axum::body::Body>) -> Response {
    match static_files::dispatch(&app, req).await {
        Ok(resp) => resp,
        Err(err) => {
            let msg = format!("Internal Server Error: {:#}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
```

crates/basil_web/src/static_files.rs (core idea)

```rust
pub async fn dispatch(app: &AppState, req: Request<Body>) -> anyhow::Result<Response> {
    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let uri_path = percent_decode_str(parts.uri.path()).decode_utf8_lossy();
    let path = resolve_path(&app.cfg.root, &uri_path)?;
    if path.is_dir() {
        let idx = path.join(&app.cfg.index);
        return serve_file(&app.cfg, parts, idx).await;
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "basil" | "bas" => return crate::script::run_script(app, Request::from_parts(parts, Body::from(body)), path).await,
        "html" if crate::template::file_contains_basil(&path).await? =>
            return crate::template::render_html_with_basil(app, Request::from_parts(parts, Body::from(body)), path).await,
        _ => return serve_file(&app.cfg, parts, path).await,
    }
}
```

crates/basil_web/src/cgi.rs (header parse sketch)

```rust
pub struct ScriptOutput { pub status: Option<StatusCode>, pub headers: HeaderMap, pub body: Bytes }

pub fn split_headers_and_body(mut raw: Vec<u8>) -> ScriptOutput {
    let sep = raw.windows(4).position(|w| w == b"\r\n\r\n")
        .or_else(|| raw.windows(2).position(|w| w == b"\n\n"));
    let (head, body) = if let Some(i) = sep { raw.split_at(i + 4) } else { (&raw[..], &raw[..0]) };
    let mut status = None;
    let mut headers = HeaderMap::new();
    for line in head.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line).trim().to_string();
        if line.is_empty() { continue; }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            if k.eq_ignore_ascii_case("Status") { status = parse_status(v); continue; }
            headers.append(HeaderName::from_bytes(k.as_bytes()).unwrap_or(header::INVALID_HEADER_NAME), HeaderValue::from_str(v).unwrap_or(HeaderValue::from_static("")));
        }
    }
    ScriptOutput { status, headers, body: Bytes::copy_from_slice(body) }
}
```

Run instructions:

* Build: `cargo build -p basil-serve`
* Serve: `target/debug/basil-serve --root ./examples/basilbasic.com --port 8000`
* Or: `basilc serve --root ./examples/basilbasic.com`

Acceptance criteria (Phase A):

* `GET /index.html` serves static 200 with correct MIME.
* `GET /hello.basil` compiles if needed then runs, honoring CGI headers (Content-Type).
* `GET /templated.html` renders basil blocks inline.
* Touching a `.basil` then reloading recompiles.
* `Location:` without `Status:` yields 302.
* Binding to `0.0.0.0` prints a dev-mode warning.

Please implement all files as above with clean, production-grade Rust (errors via anyhow/thiserror), tight path sanitization, and clear TODOs where we’ll swap the process-runner to direct VM/compiler APIs.
