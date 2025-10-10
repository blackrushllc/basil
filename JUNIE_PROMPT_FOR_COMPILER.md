Totally. I took a peek at your public `development` branch—here’s the lay of the land I can see, then I’ll give you a clean “drop-in” Junie prompt to start the compiler (named **`bcc`**) inside this repo.

# What’s already in the repo (quick read)

* Top-level crates/folders include **`basilc`**, **`basilcore`**, **`basil-objects`**, plus `examples/` and a bunch of docs. The README shows the **interpreter executable is `basilc`** and how you run programs / enable object features. ([GitHub][1])
* README examples explicitly run: `target/release/basilc run examples/hello.basil` and `cargo run -q -p basilc --features ... -- run ...`, confirming `basilc` is the VM/runner today. ([GitHub][1])

That’s enough context to scaffold a compiler without touching your current runner.

---

# Junie prompt — Add a native compiler “bcc” (Basil → Rust transpile)

> **Title:** Introduce `bcc` compiler (Basil → Rust → native), sharing the frontend with `basilc`
>
> **Context:**
> Repo: `blackrushllc/basil` (development branch). Today, `basilc` is the **interpreter/VM**. We want a **second binary** called **`bcc`** that transpiles Basil source to a tiny Rust crate and invokes `cargo` to build a native executable. The interpreter must **not** pull compiler code, and both should **share** the same lexer/parser/AST.
>
> **High-level goals:**
>
> 1. Create a small **AOT compiler driver** `bcc` in this workspace (no breaking changes to `basilc`).
> 2. **Factor** the existing front-end (lexer/parser/AST + spans) into a shared crate so both `basilc` and `bcc` use it.
> 3. Implement a **Rust backend emitter** (IR → `main.rs` + `Cargo.toml`) and a builder that shells out to `cargo build`.
> 4. Publishable runtime path: generated projects depend on `libbasilrt` and relevant `obj-*` crates so end-users don’t need this repo to compile.
>
> **Workspace structure changes (incremental):**
>
> ```
> /crates
>   basil-frontend      # NEW (or extract from basilcore): lexer, parser, AST, spans, basic semantic checks
>   basil-ir            # NEW: minimal IR + tiny passes (const-fold, DCE)
>   backend-rs          # NEW: IR → Rust emitter (writes src/main.rs + Cargo.toml)
>   libbasilrt          # NEW (or carve out of basilcore): small runtime API used by both VM & compiled code
>   basil-objects       # existing feature crates; expose Rust APIs usable from libbasilrt
> /bins
>   basilc              # existing interpreter/VM (depends on basil-frontend + basilcore + libbasilrt)
>   bcc                 # NEW compiler driver (depends on basil-frontend + basil-ir + backend-rs)
> ```
>
> *If `basilcore` currently mixes front-end + VM, move only the front-end modules into `basil-frontend` and leave the VM/runtime bits where they are. Keep `basilc` building and behavior identical.*
>
> **CLI spec (bcc):**
>
> ```
> bcc aot <input.basil> [-o <outdir>] [--name <prog>] \
>   [--features obj-audio,obj-midi,obj-daw,...] \
>   [--target <rust-target-triple>] [--opt 0|1|2|3] [--lto off|thin|fat] \
>   [--emit-project <dir>] [--keep-build] [--quiet]
> ```
>
> * Default `--target` = host target from `rustc -vV`.
> * Default `--opt=3`, `--lto=thin`.
> * `--emit-project` writes the generated Cargo crate without building (for inspection/air-gapped builds).
>
> **Backend-rs emitter:**
>
> * Generate a tiny Rust binary crate per compile in a temp dir:
    >
    >   * `Cargo.toml`:
          >
          >     * `[dependencies] libbasilrt = "x.y"` (pin exact version)
>     * Include any `obj-*` crates based on `--features`, re-exposed through `libbasilrt` or imported directly.
>   * `src/main.rs`: safe, monomorphic Rust that calls only `libbasilrt` APIs; no generics/borrows in generated code.
> * Code style: emit simple `while`/`if` for loops and branches; values are POD (`i64`, `bool`) or owned strings via `libbasilrt::Str`.
> * All I/O via `libbasilrt::{print, println, input_line,...}` to unify behavior with the VM.
> * Add line markers as comments like `// @basil: file:line:col` per emitted statement.
>
> **Minimal IR (basil-ir) to support emitter:**
>
> * Types: `Int(i64)`, `Bool`, `Str`, `ObjHandle`.
> * Ops: `Add, Sub, Mul, Div, CmpEq, CmpLt, And, Or, Not`, `ConcatStr`, `Print/PrintLn`, `Call`, `Return`.
> * CFG: basic blocks + `Jump`/`CondJump`.
> * Passes: constant fold; dead-code elimination; trivial copy-prop.
>
> **libbasilrt (runtime crate):**
>
> * Public API consumed by both the VM and compiled code, e.g.:
    >
    >   ```rust
>   pub struct Str(...);                 // owned string type
>   pub enum Val { Int(i64), Bool(bool), Str(Str), Obj(ObjHandle) }
>   pub fn print(v: &Val) -> RtResult<()>;
>   pub fn println(v: &Val) -> RtResult<()>;
>   pub fn input_line(prompt: &Str) -> Str;
>   pub mod features { /* audio/midi/daw/term facades */ }
>   ```
> * Provide adapters to current object crates (audio/midi/daw/…); keep APIs monomorphic to avoid compile-time bloat.
>
> **Build & caching in bcc:**
>
> * Materialize project under `.basil/targets/<hash>/` based on (input contents + selected features + target triple).
> * If the hash exists, reuse; otherwise render fresh.
> * Spawn `cargo build --release` with the requested `--target`, `-Z` flags avoided (stable toolchain).
>
> **End-user requirements:**
>
> * For AOT: Rust toolchain via rustup (plus the usual per-platform prerequisites). No need to clone this repo; Cargo fetches `libbasilrt`/`obj-*` from crates.io.
> * For VM: the `basilc` executable only.
>
> **Testing & acceptance:**
>
> * Keep `basilc run` behavior unchanged; CI runs current script tests on VM.
> * New CI job: build `bcc`, compile `examples/hello.basil`, run the produced exe, assert output matches VM.
> * Add golden tests for codegen (snapshot `main.rs` for tiny programs).
> * Cross-target smoke: build a small program for `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` in matrix (allow failures initially).
>
> **Deliverables:**
>
> 1. New crates: `basil-frontend`, `basil-ir`, `backend-rs`, `libbasilrt` (or equivalent factoring).
> 2. New binary: `bcc` with the CLI above.
> 3. Docs: `docs/AOT_COMPILER.md` with install prereqs and usage; README update pointing out `basilc` (VM) vs `bcc` (compiler).
> 4. Examples: compile `examples/hello.basil` to a native exe via `bcc aot`.
>
> **Non-goals for v1:** debugging symbols (DWARF), advanced optimizations, Cranelift/LLVM backends—keep it simple and Rust-only first.

---

Here you go—updated and tightened to reflect the “keep `bcc` lean, set features on the **generated project**, validate early” approach.

---

# JUNIE TASK — Add a native compiler **“bcc”** (Basil → Rust transpile)

**Context:**
Repo: `blackrushllc/basil` (development branch). The existing **interpreter/VM** binary is **`basilc`**. Add a second binary **`bcc`** that compiles Basil source to a tiny Rust crate and invokes Cargo to build a native executable.
**Key principle:** `bcc` stays lean; **features are enabled in the generated Cargo project**, not by compiling `bcc` with “obj-all”. Do **early feature validation** so users don’t hit long Cargo errors.

## Goals

1. Create **`bcc`** (compiler driver) in this workspace without changing `basilc` behavior.
2. Factor or reuse the **frontend** (lexer/parser/AST/spans + minimal semantic checks) from existing code in a shared crate.
3. Implement a **Rust backend emitter** (IR → `src/main.rs` + `Cargo.toml`) and a builder that shells out to `cargo build`.
4. Generated projects depend on published crates (**`libbasilrt`** + `basil-obj-*`) so end-users don’t need this repo to AOT-compile.

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

1. **Auto-detect** from source: parse `#USE` directives and external symbol references (`AUDIO_*`, `MIDI_*`, `DAW_*`, `TERM_*`…).
2. **Merge**: effective = `(@auto + curated default set) ∪ (CLI list or @all override)`.
3. **Early validation** (before Cargo): if code references a symbol whose feature isn’t in the effective set, **error**:

   > `AUDIO_RECORD% requires feature 'obj-audio'. Add '#USE AUDIO' or run: bcc aot app.basil --features obj-audio`
4. Generated **Cargo.toml** enables `libbasilrt` features and adds direct deps on the needed `basil-obj-*` crates (version-pinned).

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

Want me to also prep **`docs/AOT_COMPILER.md`** and the initial **`Cargo.toml` templates** `bcc` should emit (with pinned crate versions and `lto = "thin"`)?
