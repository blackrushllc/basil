# basil_web

Dev-only Axum server that serves static files, runs Basil .basil/.bas scripts with a CGI-style adapter, recompiles bytecode when stale, and renders HTML templates containing `<?basil ... ?>` inline blocks.

Status: scaffolded. Uses external `bcc` and `basilc` processes by default (feature `process-runner`). Future `lib-runner` will call into basil compiler/VM crates directly.

Quickstart

- Build: cargo build -p basil-serve
- Run:   target/debug/basil-serve --root ./examples/basilbasic.com

CLI flags

--root <dir>                 Required
--host <ip>                  Default 127.0.0.1
--port <u16>                 Default 8000
--upload-limit <bytes>       Default 10MB
--script-timeout <secs>      Default 10
--watch                      Optional
--no-etag                    Disable ETags
--index <name>               Default index.html
--bytecode-dir <dir>         Optional separate cache dir
--log <level>                info|debug|trace|warn|error

Environment overrides prefixed with BASIL_SERVE_ are supported (e.g., BASIL_SERVE_ROOT).

Security notes

- Path traversal and symlink escape are denied.
- This is a developer server. Do not expose publicly.
