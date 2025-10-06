### Overview
You can target WebAssembly in two different ways, depending on where you want to run Basil:

- WASI (wasm32-wasi): best for server-side/CLI-like environments (Wasmtime, Wasmer, Node’s WASI, browsers with WASI polyfills). You compile the existing Rust binary (the `basilc` CLI) to a `.wasm` module.
- Browser (wasm32-unknown-unknown): best for running Basil in the browser UI. This typically means building a small `cdylib` wrapper with `wasm-bindgen` that exposes functions such as `compile` and `run` from the core crates (`basil-lexer`, `basil-parser`, `basil-compiler`, `basil-vm`).

Below are end-to-end instructions for both approaches.

---

### Option A: Build the existing CLI for WASI (wasm32-wasi)
This compiles the `basilc` binary to WebAssembly for use with WASI runtimes.

1) Install the target
- `rustup target add wasm32-wasi`

2) Build
- From the workspace root:
  - `cargo build --release --target wasm32-wasi -p basilc`
- Output: `target\wasm32-wasi\release\basilc.wasm`

3) Run with a WASI runtime
- With Wasmtime:
  - `wasmtime run --dir . target/wasm32-wasi/release/basilc.wasm -- --help`
  - To run a script in the repo:
    - `wasmtime run --dir . target/wasm32-wasi/release/basilc.wasm -- examples/input.basil`
- With Wasmer:
  - `wasmer run --dir=. target/wasm32-wasi/release/basilc.wasm -- --help`

4) Passing environment variables (for CGI usage)
- The CLI contains CGI dispatch code that uses environment variables (`REQUEST_METHOD`, `QUERY_STRING`, etc.). You can provide them to the WASI runtime:
  - `wasmtime run --dir . --env REQUEST_METHOD=GET --env DOCUMENT_ROOT=. --env SCRIPT_FILENAME=examples/cgi.basil target/wasm32-wasi/release/basilc.wasm`

5) Notes and limitations under WASI
- File access: Use `--dir` (Wasmtime) or `--mapdir/--dir` (Wasmer) to grant filesystem access 
  (`basilc` reads `.basil` files and may write `.basilx` bytecode files).
- Process spawning: `std::process::Command` is used in `basilc`. Most WASI runtimes do not support arbitrary process spawning; paths that rely on spawning sub-processes won’t work in pure WASI.
- Interactive input: Functions like `INPUT$` depend on a console/TTY. Under WASI, stdin is available, but interactive raw-mode behavior is limited; design for line-oriented stdin.

If your goal is "run Basil programs on the server/edge in a sandbox", this is the simplest route.

---

### Option B: Build for the Browser (wasm32-unknown-unknown)
This requires a small wrapper crate that exposes an ergonomic JS API using `wasm-bindgen`. The current `basilc` crate is a binary and isn’t directly suitable for the browser. Instead, reuse the core crates (`basil-lexer`, `basil-parser`, `basil-compiler`, `basil-vm`) from a new `wasm` crate.

1) Create a new crate in the workspace (example structure)
- `basil-wasm\Cargo.toml`:
  ```toml
  [package]
  name = "basil-wasm"
  version = "0.1.0"
  edition = "2021"

  [lib]
  crate-type = ["cdylib", "rlib"]

  [dependencies]
  wasm-bindgen = "0.2"
  basil-lexer    = { path = "../basilcore/lexer" }
  basil-parser   = { path = "../basilcore/parser" }
  basil-compiler = { path = "../basilcore/compiler" }
  basil-vm       = { path = "../basilcore/vm" }
  basil-bytecode = { path = "../basilcore/bytecode" }
  basil-common   = { path = "../basilcore/common" }
  ```

- `basil-wasm\src\lib.rs`:
  ```rust
  use wasm_bindgen::prelude::*;
  use basil_lexer::Lexer;
  use basil_parser::parse;
  use basil_compiler::compile;
  use basil_vm::VM;

  #[wasm_bindgen]
  pub fn compile_and_run(source: &str) -> Result<String, JsValue> {
      // lex, parse, compile
      let mut lexer = Lexer::new(source);
      let ast = parse(&mut lexer).map_err(|e| JsValue::from_str(&format!("parse error: {}", e)))?;
      let program = compile(&ast).map_err(|e| JsValue::from_str(&format!("compile error: {}", e)))?;

      // run on the VM, capture stdout
      let mut vm = VM::new();
      let mut out = Vec::new();
      vm.set_output(Box::new(&mut out));
      vm.run(&program).map_err(|e| JsValue::from_str(&format!("vm error: {}", e)))?;

      Ok(String::from_utf8_lossy(&out).to_string())
  }
  ```

2) Build for the browser
- Install tools:
  - `rustup target add wasm32-unknown-unknown`
  - `cargo install wasm-bindgen-cli` or `cargo install wasm-pack`
- Build with `wasm-pack` (recommended):
  - `wasm-pack build basil-wasm --target web --release`
  - This generates `pkg/` with `basil_wasm_bg.wasm` and JS glue you can import in the browser.
- Alternatively, use `wasm-bindgen` directly:
  - `cargo build -p basil-wasm --release --target wasm32-unknown-unknown`
  - Then run `wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/basil_wasm.wasm`

3) Use from JavaScript
  ```html
  <script type="module">
  import init, { compile_and_run } from './pkg/basil_wasm.js';
  await init();
  const output = compile_and_run('PRINT "Hello from Basil!"');
  console.log(output);
  </script>
  ```

4) Notes for the browser
- Avoid any filesystem, environment variable, or process-spawn operations. Keep to pure in-memory compile+execute.
- Interactive input constructs (`INPUT$`, `INPUTC$`, etc.) need UI-mediated shims (e.g., pass a mocked input provider into `basil_vm::VM` or expose JS callbacks to provide input lines).

---

### Feature flags and objects
If you are using object features, the `basilc` crate exposes feature flags that proxy to `basil-vm`:
- Examples:
  - `cargo build --target wasm32-wasi -p basilc --features obj-bmx`
- Ensure any feature-dependent code doesn’t rely on host-only APIs when running under WASI or in the browser.

---

### Troubleshooting tips
- Linker errors for WASI: ensure the right target is installed and that you’re not pulling in non-WASI-compatible crates.
- Missing I/O permissions: pass `--dir` (Wasmtime) to mount host directories.
- Panics at runtime related to `std::process::Command`: those code paths aren’t supported under WASI. Gate them with feature flags or avoid those modes when running as WASM.
- Interactive hangs: WASI doesn’t provide raw TTY; stick to line-based input or pre-supply input.

---

### Quick cheat sheet
- Server/CLI (WASI):
  - `rustup target add wasm32-wasi`
  - `cargo build --release --target wasm32-wasi -p basilc`
  - `wasmtime run --dir . target/wasm32-wasi/release/basilc.wasm -- examples/while.basil`

- Browser:
  - Create `basil-wasm` cdylib wrapper with `wasm-bindgen`.
  - `wasm-pack build basil-wasm --target web --release`
  - Import from `pkg/` in your web app.

If you tell me which environment you’re targeting (WASI vs browser), I can tailor the exact steps and provide a minimal working wrapper (including a `Cargo.toml` and `lib.rs`) for that target.