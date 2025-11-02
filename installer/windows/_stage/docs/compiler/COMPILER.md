Short version: you’ve got four viable paths to “real EXEs” for Basil. They differ mainly in how much runtime you keep and how aggressive you want to be about performance and portability.

# The 4 approaches

1. Transpile → C (or Rust) → use a normal toolchain

* **What it is:** Compile Basil to readable C (or Rust), then call `cc`/`clang`/`zig cc` (or `rustc`) to get a native exe.
* **Pros:** Easiest to stand up; great portability; you can piggyback mature optimizers (Clang/LLVM or Rust MIR/LLVM). Debugging via generated source is approachable.
* **Cons:** Harder to do fine-grained, Basil-aware optimizations; error messages reference generated code unless you add good source maps.

2. Basil → LLVM IR (Inkwell) → native

* **What it is:** Build a real compiler frontend and lower to LLVM IR using the `inkwell` crate, then emit object files/exes.
* **Pros:** Best peak performance; huge optimization toolbox; easy cross-targeting (x64, ARM, wasm, etc.).
* **Cons:** Heavier lift; more moving parts (data layout, calling conventions, GC strategy) to get right.

3. Basil → Cranelift (Wasmtime/Cranelift) → native obj

* **What it is:** Lower to Cranelift IR and ask it to produce machine code/obj you can link.
* **Pros:** Faster to implement than LLVM; great for a “fast, correct first” compiler; pleasant IR.
* **Cons:** Fewer aggressive optimizations than LLVM; cross-target story is improving but not as broad.

4. Basil → WASM (WASI) → native (optional)

* **What it is:** Target WASI (WebAssembly System Interface). Run as a `.wasm` with `wasmtime`, or ahead-of-time compile WASM to native with tools like Wasmtime AOT.
* **Pros:** Very portable; sandboxing; you can ship small runners.
* **Cons:** Interop with native OS features and your feature objects is trickier; native perf depends on AOT.

---

# What a native Basil compiler must define

## 1) Frontend (shared with your VM, ideally)

* **Lexer/Parser**: You’ve already converged on a concrete grammar (“Kitchen Sink”). Keep this in its own crate so VM and compiler share it.
* **AST**: Typed (or “type-annotatable”) nodes that preserve source spans for diagnostics.
* **Semantic passes**: name resolution, constant folding, control-flow lowering, simple type checks (at least “is this numeric vs string vs object handle?”).

## 2) IR design (your compiler’s “truth”)

You need an internal IR that’s easy to optimize and easy to lower to your chosen backend. Minimal blocks + SSA-ish temporaries is enough.

Recommended shape:

* **Instructions**: `AddI`, `CmpEq`, `Jump`, `CondJump`, `Call`, `Return`, `LoadStr`, `Phi` (if SSA), etc.
* **Values**: `i64` (your `%`), fat `String` (ptr+len), `ObjHandle` (`@`), and `bool`.
* **Control flow**: basic blocks with explicit terminators.
* **Metadata**: source span per instruction for great error messages and (eventually) debug info.

## 3) Runtime (the “libbasilrt”)

Even with AOT, you’ll keep a small runtime:

* **Memory/string**: UTF-8 strings (RefCount + copy-on-write or immutable + arena).
* **Collections** (if Basil exposes them): arrays/dicts—decide now whether they’re value types or heap handles.
* **I/O, files, time, CLI args**.
* **Feature Objects ABI**: a stable FFI surface your compiled Basil can call (see below).
* **Error model**: functions return status + error string, or you use panics internally and map to Basil errors at FFI boundaries.

## 4) GC / ownership model

You’ve got three pragmatic options:

* **Refcount (Arc/Rc)** for strings/arrays/objects + cycle-free discipline. Simple, deterministic finalization; good fit for Rust.
* **Tracing GC** (Boehm or a small mark-sweep you own) if you need cycles and don’t want to burden users. More work in Rust.
* **Hybrid**: refcount for common types; arenas for short-lived comp units; occasional cycle breaker for known structures.

For Phase 1 native, **Refcount + arenas** is a sweet spot.

## 5) Calls & the ABI story (for Feature Objects)

Define a C-ABI surface that both the VM and AOT compiler can call:

```c
// libbasilrt.h
typedef struct { const char* ptr; uint32_t len; } BasilStr;
typedef int32_t BasilI32;
typedef void*   BasilObj;   // opaque handle

typedef struct {
  BasilI32 (*audio_open_in)(BasilObj* out_dev, BasilStr name, BasilStr* err);
  BasilI32 (*audio_record)(BasilObj dev, BasilStr* err);
  // ...
} BasilFeature_Audio;

typedef struct {
  BasilFeature_Audio audio;
  // other features exposed here
} BasilFeatures;

// Provided by runtime on program start:
const BasilFeatures* basil_get_features(void);
```

Your generated code links against `libbasilrt` (static or dynamic). Each feature (obj-audio, obj-midi, obj-daw, future obj-term) registers its vtable during startup.

## 6) Exceptions & errors

Basil doesn’t need C++ exceptions. Prefer:

* **Status codes** + `ERR$` string per thread/program, or
* **Result<T, E>** at IR level lowered to status/err for FFI calls.

## 7) Linking & packaging

* **Static runtime** for “single-file exe” feel.
* **Dynamic feature packs** (optional): e.g., `basil-obj-audio.dll/.so` that you dlopen on start if present.
* **Assets**: choose conventions now (embed with `--embed` like Go’s `-ldflags -X`, or ship beside the exe).
* **Cross-compile**: with Cargo `--target` + `zig cc` or `cross`.

---

# Practical roadmaps

## Roadmap A (fastest to usable): Transpile → C

1. **Re-use your current parser/AST.**
2. **Lower AST → Basil IR** (blocks + simple SSA).
3. **IR → C generator.** One C function per Basil function; a small runtime header (`libbasilrt.h`) for strings/arrays/ffi.
4. **Call out** to runtime & feature vtables for I/O, audio, midi, term.
5. **Compile & link** via `clang`/`zig cc`, emit `.exe`/ELF.
6. Add **source maps**: emit `#line` directives so C compiler errors map back to `.basil` lines.

*Result:* you ship native exes quickly and get decent performance through Clang/LLVM optimizations.

## Roadmap B (longer-term, highest ceiling): LLVM via Inkwell

1. AST → IR (your own) → **LLVM IR** with Inkwell.
2. Implement integer/string/handle types; call external runtime functions for strings, arrays, features.
3. Use **LLVM passes** for DCE, GVN, mem2reg, inlining.
4. Emit **DWARF** debug info mapped to Basil lines for step-debugging.
5. Add **JIT mode** later for REPL if desired (nice bonus).

*Result:* best performance and cross-targeting, at the cost of more upfront work.

## Roadmap C (Cranelift first)

Similar to LLVM plan but faster to bring up; great for “compile-run” loops and still produce native objs you link.

---

# Language details to decide now (so the backend isn’t chaos later)

* **Numeric type `%`:** pick a canonical width (e.g., `i64`); define overflow behavior (wrap vs trap).
* **String ops:** define concat, slicing cost model (copy vs view); interning policy.
* **Arrays:** 0-based (you already use this); bounds checks (on by default; allow `--release-no-bounds` later).
* **Functions:** calling convention (C-ABI for FFI; internal fastcall is fine).
* **Modules:** how do `#USE` and imports map to link units? (one obj per file vs per module).
* **Runtime init:** deterministic order for feature registration + global `DIM` initializers.

---

# Dev-ergonomics & DX

* **Errors/diagnostics:** keep your current, friendly VM errors. Reuse spans and colorized diagnostics for compiler errors.
* **Build tool:** a `basilc aot main.basil --target x86_64-unknown-linux-gnu --static` that drops `main` exe.
* **Caching:** hash the Basil inputs + feature list; incremental recompile per module.
* **Testing:** run your current Basil test corpus both in VM and AOT mode to guarantee semantics match.
* **Source maps:** for the transpile-to-C route, `#line` directives; for LLVM, real DWARF.

---

# Recommendation (what I’d do)

* **Phase 1:** Transpile-to-C backend + static `libbasilrt` in Rust (exposed as `cdylib/staticlib`). You’ll ship working native exes quickly and validate the runtime ABI for feature objects.
* **Phase 2:** Stand up an LLVM backend in parallel (Inkwell). Keep both backends behind flags (`--backend=c`, `--backend=llvm`).
* **Phase 3:** Optimize hot paths (string ops, array loops), add DWARF, and a `-O` story. Consider Cranelift as a “dev mode” fast compiler if you want super-snappy compile-run cycles.

# What that looks like:

Short answer: yes—ship **two executables** in one Rust workspace, **share the frontend** (lexer/parser/AST), and let users who want AOT builds install *only* the Rust toolchain. They won’t need your Basil repo if you publish the runtime as crates.

Here’s the clean setup.

# 1) Binaries you ship

* **`basil` (the interpreter/VM)**
  Single self-contained exe. You can build variants with feature flags (e.g., `--features obj-audio,obj-midi,obj-daw,obj-term`) or one “kitchen sink” build that includes everything.
* **`basilc` (the compiler/transpiler driver)**
  A small CLI that:

    1. parses Basil → IR (reuses the shared frontend),
    2. emits a tiny Rust crate to a temp dir (main.rs + Cargo.toml), and
    3. runs `cargo build --release` to produce a native exe.

These live in the same **Cargo workspace**, so code reuse is maximal and the interpreter doesn’t carry compiler code.

```
/crates
  basil-frontend     # lexer, parser, AST, spans  (shared)
  basil-ir           # simple IR + passes         (shared)
  libbasilrt         # runtime crate              (shared, published)
  obj-audio ...      # feature crates             (shared, published)
  backend-rs         # IR → Rust emitter          (compiler-only)
  backend-c (opt)    # keep optional              (compiler-only)
/bins
  basil              # VM binary
  basilc             # AOT/transpile driver
```

# 2) Code reuse & bloat

* The **interpreter** links: `basil-frontend` + VM + `libbasilrt` + chosen `obj-*` features.
  It **does not** depend on `backend-rs` or other compiler bits.
* The **compiler** links: `basil-frontend` + `basil-ir` + `backend-rs` (and spawns `cargo`).
  It **does not** pull in the VM.
  Result: small, focused binaries; no unnecessary bloat.

# 3) What end-users need (by scenario)

## A) They only want to run Basil scripts (the VM)

* **They need:** just your `basil` executable.
* **No Rust toolchain required.**
* You can ship multiple platform builds (Windows, macOS, Linux; possibly a static `musl` Linux build for maximal portability).

## B) They want to compile to native EXEs (AOT via transpile→Rust)

* **They need:**

    * **Rust toolchain** from rustup (this installs `rustc` + `cargo`).
    * Platform prerequisites the Rust target expects:

        * Windows (MSVC): “Visual Studio Build Tools” (the C++ build tools) – standard Rust guidance.
        * Windows (GNU): MinGW toolchain if using the `*-pc-windows-gnu` target.
        * macOS: Xcode Command Line Tools.
        * Linux: usually nothing extra; for fully static, add the `x86_64-unknown-linux-musl` target.
    * **Your `basilc` binary** (the compiler driver).
* **They do NOT need your Basil repo.**
  `basilc` can generate a tiny temp Cargo project that depends on **published crates**:

    * `libbasilrt` (your runtime)
    * any `obj-*` feature crates
      Cargo will fetch those from crates.io once, cache them, and build the user’s program into a native exe.

> Offline option: ship `basilc --emit-project` to dump the generated Cargo project, or vendor the runtime crates in a zip; `basilc` can point Cargo to a local path so no network is needed.

# 4) Typical user flow (AOT)

```
basilc aot hello.basil \
  --features obj-audio,obj-midi \
  --target x86_64-pc-windows-msvc \
  --opt 3 --lto thin
# => produces hello.exe in ./target-out/
```

Under the hood, `basilc` writes:

* `generated/Cargo.toml` with deps: `libbasilrt = "x.y"`, `obj-audio = "x.y"`, …
* `generated/src/main.rs` (your emitted Rust)
* runs `cargo build --release --target …`
* copies the artifact to `target-out/`

# 5) Cross-compiling notes

* Users can add targets with `rustup target add <triple>`.
* Some targets need a linker (e.g., MinGW for `*-gnu`, `musl` toolchain for fully static Linux).
* You can also publish prebuilt interpreter binaries for all OSes, and let AOT users build locally with their toolchain.

# 6) Packaging choices (nice-to-have)

* **Publish crates:** `libbasilrt` and each `obj-*` on crates.io. Keep them monomorphic (no heavy generics) to speed compiles.
* **Lock versions:** have `basilc` pin exact versions it knows (avoid surprise breakage).
* **Emit-only mode:** `basilc aot main.basil --emit-project ./out-crate` for users who want to edit then `cargo build` manually.
* **Static builds:** offer `--target x86_64-unknown-linux-musl` for “drop anywhere” Linux EXEs.

# 7) TL;DR answers to your questions

* **Separate exe for the compiler?** Yes. Ship `basil` (VM) and `basilc` (AOT). The VM stays lean and doesn’t embed compiler code.
* **Same project? Reuse parser?** Yes. One workspace; both binaries reuse the same `basil-frontend` (lexer/parser/AST) and other shared crates.
* **What must AOT users install?** At minimum: the **Rust toolchain** (via rustup) + target prerequisites. They don’t need your Basil repo if you publish `libbasilrt` and the `obj-*` crates—`basilc` will generate/compile a tiny Rust project that depends on those.

If you want, I can draft:

* the `libbasilrt` public API surface (types + key fns),
* the `basilc` CLI spec,
* and the exact `Cargo.toml` template `basilc` should emit.
