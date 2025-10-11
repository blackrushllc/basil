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

# End of document