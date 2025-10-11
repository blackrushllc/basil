# JUNIE TASK — Add a native compiler **“bcc”** (Basil → Rust transpile)

**Title:** Introduce `bcc` compiler (Basil → Rust → native), sharing the frontend with `basilc`

**Context:**
Repo: `blackrushllc/basil` (development branch). The existing **interpreter/VM** binary is **`basilc`**. Add a second
binary **`bcc`** that compiles Basil source to a tiny Rust crate and invokes Cargo to build a native executable.
**Key principle:** `bcc` stays lean; **features are enabled in the generated Cargo project**, not by compiling `bcc`
with “obj-all”. Do **early feature validation** so users don’t hit long Cargo errors.

Today, `basilc` is the **interpreter/VM**. We want a **second binary**
called **`bcc`** that transpiles Basil source to a tiny Rust crate and invokes `cargo` to build a native executable. The
interpreter must **not** pull compiler code, and both should **share** the same lexer/parser/AST.

## Goals

1. Create **`bcc`** (compiler driver) in this workspace without changing `basilc` behavior.
   * Create a small **AOT compiler driver** `bcc` in this workspace (no breaking changes to `basilc`).
2. Factor or reuse the **frontend** (lexer/parser/AST/spans + minimal semantic checks) from existing code in a shared
   crate.
    *   **Factor** the existing front-end (lexer/parser/AST + spans) into a shared crate so both `basilc` and `bcc` use it.
3. Implement a **Rust backend emitter** (IR → `src/main.rs` + `Cargo.toml`) and a builder that shells out to
   `cargo build`.
4. Generated projects depend on published crates (**`libbasilrt`** + `basil-obj-*`) so end-users don’t need this repo to
   AOT-compile.
   * Publishable runtime path: generated projects depend on `libbasilrt` and relevant `obj-*` crates so end-users don’t need this repo to compile.  

 

## Workspace layout (incremental)

```
/crates
  basil-frontend   # NEW or refactor: lexer, parser, AST, spans, basic semantic checks (shared)
  basil-ir         # NEW: tiny IR + passes (const-fold, DCE)
  backend-rs       # NEW: IR → Rust emitter (writes Cargo.toml + main.rs)
  libbasilrt       # NEW (or carved from current runtime): public runtime API used by VM & compiled code
  basil-objects    # existing feature crates (expose Rust APIs via libbasilrt::features)
 /bins
  basilc           # existing interpreter/VM (uses basil-frontend + current runtime)
  bcc              # NEW compiler driver (uses basil-frontend + basil-ir + backend-rs)
```

## `bcc` CLI

```
bcc aot <input.basil>
  [-o <outdir>] [--name <prog>]
  [--features @auto|@all|obj-audio,obj-midi,...]
  [--target <rust-triple>] [--opt 0|1|2|3] [--lto off|thin|fat]
  [--emit-project <dir>] [--keep-build] [--quiet]
```

* **Default**: `--features @auto` plus a curated **stable default set** (e.g., audio,midi,daw,term).
* `@all`: include **all stable** features.
* Explicit list: comma/space-separated `obj-*` names.
* `--emit-project`: generate the Cargo crate but don’t build (air-gapped / inspection).

## Features handling (recommended flow)

1. **Auto-detect** from source: parse `#USE` directives and external symbol references (`AUDIO_*`, `MIDI_*`, `DAW_*`,
   `TERM_*`…).
2. **Merge**: effective = `(@auto + curated default set) ∪ (CLI list or @all override)`.
3. **Early validation** (before Cargo): if code references a symbol whose feature isn’t in the effective set, **error**:

   > `AUDIO_RECORD% requires feature 'obj-audio'. Add '#USE AUDIO' or run: bcc aot app.basil --features obj-audio`
4. Generated **Cargo.toml** enables `libbasilrt` features and adds direct deps on the needed `basil-obj-*` crates (
   version-pinned).

## backend-rs (emitter) requirements

* Emit **simple, monomorphic, ownership-only** Rust (no lifetimes/borrows/generics).
* Control flow: `while`/`if`; values: `i64`, `bool`, `rt::Str`, `ObjHandle`.
* All I/O via `libbasilrt::{print, println, input_line}`; object calls via `libbasilrt::features::*`.
* Add source markers as comments `// @basil: file:line:col` for diagnostics.
* Support `--name` for package/binary name.
* `Cargo.toml` template (pin exact versions, set `lto="thin"`, `codegen-units=1`).
* Respect `--target`, `--opt`, `--lto` by passing through to `cargo build`.

## basil-ir (minimal)

* Types: `Int`, `Bool`, `Str`, `ObjHandle`.
* Ops: arithmetic, comparisons, boolean ops, string concat, `Print/PrintLn`, `Call`, `Return`.
* CFG: basic blocks with `Jump/CondJump`.
* Passes: const-fold, DCE, trivial copy-prop.

## libbasilrt (runtime crate)

* Public API used by both VM and compiled output:

  ```rust
  pub struct Str(/* Arc<str> or String */);
  pub enum Val { Int(i64), Bool(bool), Str(Str), Obj(ObjHandle) }
  pub type RtResult<T> = Result<T, RtError>;
  pub fn print(v:&Val)->RtResult<()>;  pub fn println(v:&Val)->RtResult<()>;
  pub fn input_line(prompt:&Str)->Str;
  pub mod features { /* gated modules: audio, midi, daw, term, ... */ }
  ```
* Gate `features::*` modules behind Cargo features; keep APIs monomorphic for fast builds.

## Build & cache

* Project directory: `.basil/targets/<hash>/` where hash = (input content + effective features + target + versions).
* Reuse cache when unchanged; allow `--keep-build` to leave the temp crate on disk.

## CI & tests

* Keep all existing VM tests.
* New jobs:

    * Build `bcc`.
    * `bcc aot examples/hello.basil` → run compiled exe → assert VM parity.
    * Snapshot tests for tiny programs’ emitted `main.rs`.
    * Optional cross-matrix for `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` (allow-fail initially).

## Acceptance

* `basilc` behavior unchanged.
* `bcc aot` compiles at least hello-world + a feature-using example with `--features @auto`.
* Helpful early errors for missing features; no reliance on slow Cargo failures.
* Generated crates pin versions and set `lto="thin"`; binaries run on host without needing this repo.

---

Please reference **`docs/AOT_COMPILER.md`** and the initial **`Cargo.toml` templates** `bcc` should emit (with
pinned crate versions and `lto = "thin"`)

# Basil AOT Compiler (`bcc`) — Basil → Rust → Native EXE

This document explains how the **`bcc`** compiler turns a Basil source file into a
stand-alone executable by emitting a tiny Rust crate and invoking `cargo build`.

- Interpreter (VM): **`basilc`** — run `.basil` scripts directly.
- AOT Compiler: **`bcc`** — transpile `.basil` → Rust → native EXE.

> Key principle: `bcc` stays lean. **Features are enabled in the generated Cargo project**, not by compiling `bcc` with “obj-all”. We do **early feature validation** so users don’t hit long Cargo errors.

---

## Requirements (for AOT)

- Rust toolchain via **rustup** (stable).
    - Windows (MSVC): install “Visual Studio Build Tools” C++ workload.
    - macOS: Xcode command line tools.
    - Linux: typically nothing extra; for static builds, add the `musl` target.
- Internet access for the first build (Cargo will fetch runtime crates), unless you use `--emit-project` and vendor deps.

End-users **do not** need this repository to compile Basil programs; `bcc` generates a small Cargo project that depends on published crates (`libbasilrt` and `basil-obj-*`).

---

## Quickstart

```bash
# Compile a Basil program to a native exe
bcc aot examples/hello.basil

# Run the native result (path is printed by bcc)
./target-out/hello         # or hello.exe on Windows
````

Common flags:

```bash
bcc aot app.basil \
  --name app \
  --features @auto \
  --target x86_64-unknown-linux-gnu \
  --opt 3 --lto thin
```

---

## Features handling

`bcc` determines which Feature Objects to include by:

1. **Auto-detecting** from source (`#USE AUDIO, MIDI` and symbol scans like `AUDIO_*`).
2. **Merging** with your CLI request:

    * `--features @auto` (default): auto + a curated stable baseline (audio, midi, daw, term).
    * `--features @all`: include all **stable** features.
    * `--features obj-audio,obj-midi,...`: explicit set (comma/space separated).

**Early validation:** If code references a symbol that requires a feature not in the effective set, `bcc` fails fast, e.g.:

```
error: AUDIO_RECORD% requires feature 'obj-audio'
help: Add '#USE AUDIO' or run with: --features obj-audio
```

---

## CLI Reference

```text
bcc aot <input.basil>
  [-o <outdir>]            # where to place the final exe (default: ./target-out)
  [--name <prog>]          # package/binary name for the generated crate
  [--features @auto|@all|obj-audio,obj-midi,...]
  [--target <triple>]      # e.g., x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc
  [--opt 0|1|2|3]          # Rust opt-level (default: 3)
  [--lto off|thin|fat]     # link-time optimization (default: thin)
  [--emit-project <dir>]   # emit the generated Cargo crate without building
  [--keep-build]           # keep the temp build directory for inspection
  [--quiet]
```

---

## What `bcc` generates

```
generated/
  Cargo.toml           # pinned versions, features enabled
  src/
    main.rs            # simple, monomorphic Rust calling libbasilrt
.basil/targets/<hash>/ # cache dir keyed by source+features+target+versions
```

* **`Cargo.toml`** enables `libbasilrt` features and adds deps on the required `basil-obj-*` crates.
* **`src/main.rs`** contains safe, ownership-only Rust (no borrows/generics) and calls:

    * `libbasilrt::{print, println, input_line, ...}`
    * `libbasilrt::features::{audio, midi, daw, term, ...}` when used.
* Each emitted statement is annotated with comments like `// @basil: file:line:col` for diagnostics.

---

## Reproducible builds

* `Cargo.toml` **pins exact versions** of `libbasilrt` and `basil-obj-*`.
* Release profile defaults: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`.

---

## Cross-compiling

```bash
rustup target add x86_64-unknown-linux-musl
bcc aot app.basil --target x86_64-unknown-linux-musl
```

* Windows GNU/MSVC, macOS, and Linux targets are supported as allowed by your local toolchain.
* For fully static Linux binaries, prefer the `musl` target.

---

## Troubleshooting

* **“Missing feature” error:** Add `#USE FOO` in Basil or pass `--features obj-foo`.
* **Cargo can’t find a crate:** Ensure you’re online for the first build, or use `--emit-project` and vendored deps.
* **MSVC link errors on Windows:** Install “Build Tools for Visual Studio” (C++).
* **Slow rebuilds:** `bcc` caches generated projects under `.basil/targets/<hash>/`. Reuse is automatic if nothing changed.

---

## VM parity & tests

* The same front-end (lexer/parser/AST) is shared by `basilc` and `bcc`.
* CI compiles `examples/hello.basil` via `bcc` and compares output with `basilc run`.

---

# `bcc` Cargo.toml template(s)

### 1) Standard template (default)

```toml
[package]
name = "basil_prog_{{NAME_OR_HASH}}"
version = "0.1.0"
edition = "2021"

[dependencies]
libbasilrt = { version = "=0.1.0", features = [ {{FEATURE_LIST}} ] }
# The following lines are **conditionally included** based on effective features:
# basil-obj-audio = "=0.1.0"
# basil-obj-midi  = "=0.1.0"
# basil-obj-daw   = "=0.1.0"
# basil-obj-term  = "=0.1.0"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
```

* `{{FEATURE_LIST}}` becomes something like `"audio", "midi", "daw", "term"`.
* Only the selected `basil-obj-*` lines are emitted (with exact pinned versions).

### 2) (Optional) Static Linux / musl variant

If you want a template that assumes musl (you can also just pass `--target x86_64-unknown-linux-musl` from `bcc` and keep the template unchanged):

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
```

(You generally don’t need extra config; the **target triple** controls static linking on musl.)

---

src/main.rs — emission template stub

Generate this verbatim, then substitute the {{}} placeholders. Keep the code monomorphic and avoid borrows/generics in emitted nodes.

```rust

// AUTOGENERATED by bcc — DO NOT EDIT
// Source: {{SOURCE_BASIL_PATH}}
//
// Notes:
// - Emitted Rust is intentionally simple: ownership-only, no borrows/generics.
// - All I/O & features go through libbasilrt to match VM semantics.
// - Each statement line carries a @basil comment for diagnostics.

use libbasilrt as rt;

// --- Literal helpers (inlined) ---
#[inline(always)]
fn lit_str(s: &'static str) -> rt::Str { rt::Str::from_static(s) }

#[inline(always)]
fn lit_int(i: i64) -> rt::Val { rt::Val::Int(i) }

#[inline(always)]
fn lit_bool(b: bool) -> rt::Val { rt::Val::Bool(b) }

#[inline(always)]
fn lit_val_str(s: &'static str) -> rt::Val { rt::Val::Str(rt::Str::from_static(s)) }

// --- Program entry (generated from IR) ---
fn basil_main() -> rt::RtResult<()> {
    // @basil: {{FILE}}:{{LINE}}:{{COL}}
    // (Example of a simple println)
    // rt::println(&lit_val_str("Hello from Basil"))?;

    // {{BEGIN_EMITTED_BODY}}
    {{EMITTED_BODY}}
    // {{END_EMITTED_BODY}}

    Ok(())
}

// --- Optional: simple top-level error surfacing ---
fn main() {
    if let Err(e) = basil_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}


```



Notes for the emitter

* Replace {{SOURCE_BASIL_PATH}}, and per-statement // @basil: file:line:col.
* Splice the generated Rust statements into {{EMITTED_BODY}}.
* When calling features, always go via rt::features::X::fn(...) so Cargo features govern availability.
* For strings, prefer lit_val_str("...") and for dynamic strings return rt::Str from runtime helpers (e.g., rt::concat(a, b)).
* Keep temp variables let mut tN: rt::Val / let mut iN: i64 style—predictable and easy for rustc to optimize.



Optional: tiny statement snippets (ready-to-emit)

These are “lego bricks” Junie can map from our IR:



```rust
// PRINT <val>;
rt::print(&VAL)?;                        // @basil: {{f:l:c}}

// PRINTLN <val>;
rt::println(&VAL)?;                      // @basil: {{f:l:c}}

// IF cond THEN ...
if COND_BOOL {
    // ...
}                                        // @basil: {{f:l:c}}

// WHILE ...
while COND_BOOL {
    // body ...
}                                        // @basil: {{f:l:c}}

// FOR i = a TO b STEP s
{
    let mut i: i64 = A;
    let end_: i64 = B;
    let step_: i64 = S;
    while if step_ >= 0 { i <= end_ } else { i >= end_ } {
        // body ...
        i = i.saturating_add(step_);
    }
}                                        // @basil: {{f:l:c}}

// STRING CONCAT
let s_ab: rt::Str = rt::concat(&A_STR, &B_STR);    // @basil: {{f:l:c}}

// FEATURE CALL (example)
let rc: i32 = rt::features::audio::record(/* args */)?;

```


## One-page PR checklist for bcc (AOT compiler)

Scaffolding & structure
* New binary bcc added; basilc unchanged.
* Crates created/refactored: basil-frontend (parser/AST/spans), basil-ir (minimal IR), backend-rs (emitter), libbasilrt (runtime API).
* Workspace builds on stable Rust (rust-toolchain pinned to stable or 1.xx).
 
CLI & UX

* bcc aot <file> works; defaults: --features @auto, --opt 3, --lto thin.
* Flags implemented: --name, --features @auto|@all|obj-*, --target, --opt, --lto, --emit-project, --keep-build, --quiet.
* Helpful --help text with examples.

Features & validation

* Auto-detect #USE + symbol scan; merge with CLI set and curated stable defaults.
* Early error on missing feature: clear message + suggested fix.
* Mapping table: Basil symbols → feature names → crates & libbasilrt feature flags.

Emitter quality

* Emits generated/src/main.rs exactly like the stub above (ownership-only, monomorphic).
* Adds // @basil: file:line:col comments to each emitted statement.
* Cargo.toml is templated with pinned crate versions and:

```toml

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"


```


Runtime API

 libbasilrt exposes: Str, Val, RtResult, print, println, input_line, concat.
 libbasilrt::features::{audio,midi,daw,term,...} behind Cargo features, with thin, monomorphic functions.

Build & caching
 
* Cache dir .basil/targets/<hash>/ (hash over source+features+target+versions).
* --emit-project writes a reproducible crate to disk without building.

Tests & CI

* Unit tests for feature detection & validation.
* Snapshot tests for tiny emitted main.rs (hello/loops/if/strings).
* Golden parity: basilc run X output == compiled exe output for the same program.
* CI matrix builds bcc, compiles examples/hello.basil on at least Linux; optional musl and Windows.

Docs

* docs/AOT_COMPILER.md added and linked from README.
* README brief: distinguish basilc (VM) vs bcc (AOT), with Quickstart.

Polish

* Friendly error messages; no panics on user input.
* No accidental dependency on nightly.
* Version pinning for published crates (=x.y.z) to keep builds reproducible.

## PR Checklist (keep the PR tight)
 
* New bcc binary; basilc unchanged.
* Crates added/refactored: basil-frontend, basil-ir, backend-rs, libbasilrt.
* CLI implemented with --features @auto|@all|obj-* and early validation.
* Emission uses the stub above; Cargo.toml versions pinned; release profile has lto="thin".
* Cache at .basil/targets/<hash>/.
* Tests: feature detection, snapshot main.rs, VM parity for hello-world.
* Docs added and README updated.