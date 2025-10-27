Here’s a ready-to-paste **Junie Prompt** for Phase 2 — swap the dev server from “process-runner” to a **native library runner** while preserving the same CLI and behavior. It adds a `lib-runner` feature with trait-based adapters so we can keep both paths (spawn vs. in-proc) behind feature flags.

---

# Title

Phase 2: Add `lib-runner` to basil_web (native VM/compiler integration), keep `process-runner` as fallback

# Goal

Enable the dev server to call the **Basil compiler** and **VM** directly as Rust libraries (no child processes). Keep both backends available:

* `--features lib-runner` → use Basil crates directly
* `--features process-runner` (default for now) → spawn `bcc` / `basilc`

All existing behavior should remain: compile-if-stale, run `.basil/.bas`, render `<?basil … ?>` blocks, relative redirects, timeouts, upload limits, etc.

# Assumptions

* A Basil compiler crate (or soon-to-exist) exposing:
  `compile_to_bytecode(source: &Path, bytecode: &Path) -> Result<CompilerReport>`
* A Basil VM crate exposing:
  `run_bytecode(bytecode: &Path, env: &Env, io: &mut VmIo) -> Result<VmExit>`
  and/or a direct `run_source` for small in-memory snippets
* If exact APIs differ, create **thin adapters** in `basil_web` and keep stable traits there.

# Deliverables

1. Feature flags + traits in `basil_web`:

    * `CompileService` + `RunService` traits
    * `impl` for `lib-runner`
    * Existing `process-runner` rewritten as another `impl` of those traits
2. Swap `compile.rs` and `script.rs` to depend on the traits (not on process calls)
3. Add **in-memory template block eval** (no temp files) when `lib-runner` enabled
4. Tests to validate both backends produce identical HTTP results on the same inputs
5. Docs notes in `crates/basil_web/README.md`

# Edits

## A) Cargo features

**crates/basil_web/Cargo.toml**

* Add:

```toml
[features]
default = ["process-runner"]
process-runner = []
lib-runner = []  # enables in-proc compiler + VM path

[dependencies]
# existing deps...
anyhow = "1"
thiserror = "1"
bytes = "1"
# Add conditional deps only if you already have Basil crates; otherwise leave as path placeholders:
basil_compiler = { path = "../../crates/basil_compiler", optional = true }
basil_vm       = { path = "../../crates/basil_vm",       optional = true }

[target.'cfg(any())'.dependencies]
# Tie them to the lib-runner feature:
basil_compiler = { path = "../../crates/basil_compiler", optional = true, package = "basil-compiler", features = [], default-features = true }
basil_vm       = { path = "../../crates/basil_vm",       optional = true, package = "basil-vm",       features = [], default-features = true }

[features]
lib-runner = ["basil_compiler", "basil_vm"]
```

> If crate names differ, adjust the `package =` lines accordingly.

## B) Introduce stable service traits

**crates/basil_web/src/engine.rs** (new)

```rust
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct CompileOutcome {
    pub changed: bool,
    pub version: Option<String>,
    pub stderr_tail: Option<String>,
    pub bytecode_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Env {
    pub vars: HashMap<String, String>,  // CGI-like env
    pub stdin: Option<Vec<u8>>,         // request body
    pub timeout: Duration,
}

#[derive(Default)]
pub struct VmIo {
    pub stdout: Vec<u8>,  // script writes here
}

pub trait CompileService: Send + Sync + 'static {
    fn compile_to_bytecode(&self, source: &Path, bytecode: &Path) -> Result<CompileOutcome>;
}

pub trait RunService: Send + Sync + 'static {
    fn run_bytecode(&self, bytecode: &Path, env: &Env) -> Result<VmIo>;
    /// Optional for template blocks (in-memory source)
    fn run_source_block(&self, source_snippet: &str, env: &Env) -> Result<VmIo> {
        let _ = (source_snippet, env);
        anyhow::bail!("run_source_block not supported in this backend");
    }
}
```

## C) Implement `process-runner` as trait impls

**crates/basil_web/src/engine_process.rs** (new; move current spawn logic here)

* Implement `CompileService` by spawning `bcc`/`basilc --bytecode`
* Implement `RunService` by spawning `basilc` to run the `.basilx`
* `run_source_block` can write to a temp `__inline_<hash>.basil` and reuse compile+run for parity

```rust
#[cfg(feature = "process-runner")]
pub struct ProcCompiler { /* cfg, paths, timeouts */ }

#[cfg(feature = "process-runner")]
impl CompileService for ProcCompiler { /* ... */ }

#[cfg(feature = "process-runner")]
pub struct ProcRunner { /* cfg, timeouts */ }

#[cfg(feature = "process-runner")]
impl RunService for ProcRunner { /* ... */ }
```

## D) Implement `lib-runner` as trait impls

**crates/basil_web/src/engine_lib.rs** (new)

* Use the Basil crates directly
* No child processes; use in-memory IO buffers
* Provide `run_source_block` that compiles/evals snippet in memory (or via an internal “snippet” API if exposed by VM)

```rust
#[cfg(feature = "lib-runner")]
pub struct LibCompiler {
    // any config or handles you need
}
#[cfg(feature = "lib-runner")]
impl CompileService for LibCompiler {
    fn compile_to_bytecode(&self, source: &Path, bytecode: &Path) -> Result<CompileOutcome> {
        // call basil_compiler APIs
        // write .basilx; fill CompileOutcome
        unimplemented!()
    }
}

#[cfg(feature = "lib-runner")]
pub struct LibRunner { /* cfg: timeouts, limits, etc. */ }

#[cfg(feature = "lib-runner")]
impl RunService for LibRunner {
    fn run_bytecode(&self, bytecode: &Path, env: &Env) -> Result<VmIo> {
        // map env.vars into VM env; feed env.stdin
        // collect stdout into VmIo
        unimplemented!()
    }
    fn run_source_block(&self, source_snippet: &str, env: &Env) -> Result<VmIo> {
        // compile + run snippet entirely in-memory if VM supports it;
        // fallback: compile to temp bytecode in a cache and run.
        unimplemented!()
    }
}
```

## E) Wire selection in `lib.rs`

**crates/basil_web/src/lib.rs**

* Add a constructor that selects the backend based on features

```rust
#[cfg(feature = "process-runner")]
pub type DefaultCompiler = crate::engine_process::ProcCompiler;
#[cfg(feature = "process-runner")]
pub type DefaultRunner   = crate::engine_process::ProcRunner;

#[cfg(all(not(feature = "process-runner")), feature = "lib-runner"))]
pub type DefaultCompiler = crate::engine_lib::LibCompiler;
#[cfg(all(not(feature = "process-runner")), feature = "lib-runner"))]
pub type DefaultRunner   = crate::engine_lib::LibRunner;
```

* Expose `make_services(cfg: &Config) -> (impl CompileService, impl RunService)`

## F) Swap `compile.rs` and `script.rs` to use traits

**crates/basil_web/src/compile.rs**

* Replace direct spawn calls with:

```rust
pub async fn compile_if_stale<C: CompileService>(
    compiler: &C,
    source: &Path,
    bytecode: &Path
) -> Result<CompileOutcome> {
    let stale = /* mtime compare or hash compare */;
    if stale { compiler.compile_to_bytecode(source, bytecode) } else {
        Ok(CompileOutcome{ changed:false, version:None, stderr_tail:None, bytecode_path: bytecode.to_path_buf() })
    }
}
```

**crates/basil_web/src/script.rs**

* Accept a `&dyn RunService` to run `.basilx`, parse headers/body as before.
* Respect timeout: wrap execution in `tokio::time::timeout`.

## G) In-memory template blocks (lib-runner)

**crates/basil_web/src/template.rs**

* When `lib-runner` active, prefer `runner.run_source_block(code, &env)` for each `<?basil … ?>` block instead of writing temp files.
* Keep fallback path for `process-runner` (write temp source, compile_if_stale, run).

## H) App wiring

**crates/basil_web/src/server.rs**

* Initialize services once and keep in `AppState`:

```rust
pub struct AppState {
  pub cfg: Config,
  pub compiler: Arc<dyn CompileService>,
  pub runner: Arc<dyn RunService>,
  // ...
}
```

* Update handlers to call through `app.compiler` / `app.runner`.

## I) Tests

Add tests to ensure parity:

1. **Hello script**: `.basil` returns same headers/body under both features.
2. **Redirect**: printing `Location:` without `Status:` yields 302 in both.
3. **Template**: page with two `<?basil print "X" ?>` renders “XX”.
4. **Recompile**: touch source; next request recompiles (observe `changed == true`).
5. **Timeout**: long script → 504 or 500 with clear error (same code path).

Provide two cargo invocations in CI (or local makefile):

```
cargo test -p basil_web --features process-runner
cargo test -p basil_web --no-default-features --features lib-runner
```

## J) Docs

Update `crates/basil_web/README.md`:

* Explain both backends, default features, how to switch:

```
# process-runner (default)
cargo run -p basil-serve -- --root ./examples/basilbasic.com

# lib-runner
cargo run -p basil-serve --no-default-features --features lib-runner -- --root ./examples/basilbasic.com
```

* Note dev-only scope; warn on 0.0.0.0 binding.

# Acceptance Criteria

* Building with `lib-runner` runs scripts without spawning external processes.
* `<?basil … ?>` blocks execute in memory (no temp files) when `lib-runner` is enabled.
* HTTP behavior and outputs are functionally identical across both modes on the supplied tests.
* Clear errors when Basil crates are missing while `lib-runner` is requested.

# Nice-to-haves (optional if time allows)

* A tiny **capabilities endpoint** (`/_basil/capabilities`) that shows which runner is active.
* Bytecode cache path configurable to a dedicated `.basilcache/` root for cleaner trees.
* Memory/CPU limits surfaceable through runner config (future VM API).

---

If any Basil crate APIs aren’t ready yet, implement the `lib-runner` methods with `todo!()` plus `#[cfg(feature = "lib-runner")]` compile-time errors explaining which exports are needed (function signatures listed above).
