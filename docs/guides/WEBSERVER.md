basil_web

Dev-only Axum server that serves static files, runs Basil .basil/.bas scripts with a CGI-style adapter, recompiles
bytecode when stale, and renders HTML templates containing `<?basil ... ?>` inline blocks.

Status: scaffolded. Uses the external `basilc` process by default (feature `process-runner`). Future `lib-runner` will
call into basil compiler/VM crates directly.

Quickstart

- Build: cargo build -p basil-serve
- Run:   target/debug/basil-serve --root ./examples/basilbasic.com

CLI flags

--root <dir>                 Required
--host <ip>                  Default 127.0.0.1
--port <u16>                 Default 8000
--upload-limit <bytes>       Default 10MB
--script-timeout <secs>      Default 10
--watch Optional
--no-etag Disable ETags
--index <name>               Default index.html
--bytecode-dir <dir>         Optional separate cache dir
--log <level>                info|debug|trace|warn|error

Environment overrides prefixed with BASIL_SERVE_ are supported (e.g., BASIL_SERVE_ROOT).

Security notes

- Path traversal and symlink escape are denied.
- This is a developer server. Do not expose publicly.


Logging

- By default, basil-serve initializes console logging (tracing) at the level specified by `--log` (default: `info`).
- If the `RUST_LOG` environment variable is set, it takes precedence over `--log` and can use the full `env_filter` syntax (e.g., `RUST_LOG=basil_web=debug,axum=info`).
- Server behavior:
  - 404 Not Found and 405 Method Not Allowed are logged at warn level.
  - 500 Internal Server Error (unexpected failures) is logged at error level, along with the request method and URI.
  - Script stderr (from running `.basil` files) is tailed and logged at warn level.
  - Script timeouts and spawn/run errors are logged at error level.

Examples

- `--log debug` to see detailed server activity.
- `RUST_LOG=trace` (or Windows PowerShell `$env:RUST_LOG='trace'`) to override and enable maximum verbosity across all modules.
