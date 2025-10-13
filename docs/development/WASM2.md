# Running BASIL in the browser with WASM

## Approach

1. **Keep the VM in Rust, target WebAssembly (WASM).**

   * Toolchain: `wasm32-unknown-unknown` target, **wasm-bindgen**, **wasm-pack**.
   * Optional ergonomics: **console_error_panic_hook** (nicer stack traces), **wee_alloc** (smaller builds).

2. **Abstract host I/O once, implement it twice (native & web).**

   * Define a `Host` trait in Rust (print, read line, time, random, env vars, optional FS).
   * Provide:

      * `NativeHost` (current CLI/CGI behavior).
      * `WebHost` (bridges to JS for `PRINT/PRINTLN`, `INPUT/INKEY`, time, randomness, etc.).
   * This keeps the VM identical and portable.

3. **Expose a minimal JS-facing API via wasm-bindgen.**

   * `BasilVm.new(host_callbacks) -> BasilVmHandle`
   * `load_bytecode(Uint8Array)`
   * `run()` (blocking to completion), or `step()`/`run_until_waiting_for_input()` for interactive UIs
   * `provide_input(string)` to satisfy `INPUT/INKEY`
   * Optional: `reset()`, `get_globals()`, `set_global(name, value)`

4. **Load `.basilx` directly in the browser.**

   * `fetch("program.basilx") → ArrayBuffer → Uint8Array → vm.load_bytecode(...)`
   * No WASI/Emscripten needed if you don’t need a POSIX-like FS. If you *do* want files, use a simple virtual FS you define and expose read/write calls to JS (IndexedDB or OPFS behind the scenes).

5. **UI layer in JS (simple or fancy).**

   * Minimal: a `<textarea>` or `<div>` for output and a single-line input box hooked to `provide_input`.
   * Nicer: **xterm.js** for a terminal-like experience; a Web Worker to keep the UI responsive.

6. **Performance & threading.**

   * Single-threaded WASM is fine to start.
   * If you later want WASM threads, you’ll need COOP/COEP headers and SharedArrayBuffer; not required for v1.

7. **Security model.**

   * In the browser, the VM can’t do arbitrary I/O unless you expose it. This is good. Keep host calls whitelisted.

# Difficulty (scope-only, not time)

* **Low–Medium:** wasm target + bindgen + minimal I/O bridge.
* **Medium:** polished terminal UI, async INPUT, pause/resume stepping, virtual FS.
* **Medium–High:** full WASI-like sandbox, persistent storage, plugin system.

# What tools you’ll need (besides RustRover & AI)

* **Rust**: stable toolchain with `wasm32-wasi` target (for WASI-like sandbox)
* **wasm-bindgen-cli** and **wasm-pack**
* **Node/NPM** (to package & demo)
* (Optional) **Vite**/**Parcel**/**Webpack** for a quick demo app
* (Optional) **xterm.js** for terminal UI

# Skeletons Junie can start from

## Rust (WASM-facing API)

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
extern "C" {
    // JS callbacks supplied by the host page
    #[wasm_bindgen(js_namespace = BasilHost)]
    fn js_println(s: &str);
    #[wasm_bindgen(js_namespace = BasilHost)]
    fn js_need_input();
    #[wasm_bindgen(js_namespace = BasilHost)]
    fn js_rand_u32() -> u32;
    #[wasm_bindgen(js_namespace = BasilHost)]
    fn js_now_ms() -> f64;
}

pub trait Host {
    fn println(&mut self, s: &str);
    fn need_input(&mut self);            // signal the UI to prompt
    fn rand_u32(&mut self) -> u32;
    fn now_ms(&mut self) -> f64;
    fn take_stdin_line(&mut self) -> Option<String>; // provided by UI
}

pub struct WebHost {
    pending_input: Option<String>,
}

impl WebHost {
    pub fn new() -> Self { Self { pending_input: None } }
    pub fn provide_input(&mut self, line: String) { self.pending_input = Some(line); }
}

impl Host for WebHost {
    fn println(&mut self, s: &str) { js_println(s); }
    fn need_input(&mut self) { js_need_input(); }
    fn rand_u32(&mut self) -> u32 { js_rand_u32() }
    fn now_ms(&mut self) -> f64 { js_now_ms() }
    fn take_stdin_line(&mut self) -> Option<String> { self.pending_input.take() }
}

#[wasm_bindgen]
pub struct BasilVmHandle {
    vm: basil_vm::Vm<WebHost>, // your VM generic over Host
}

#[wasm_bindgen]
impl BasilVmHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BasilVmHandle {
        console_error_panic_hook::set_once();
        BasilVmHandle { vm: basil_vm::Vm::new(WebHost::new()) }
    }

    #[wasm_bindgen]
    pub fn load_bytecode(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.vm.load_bytecode(bytes).map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    /// Run until completion or until VM requests input
    #[wasm_bindgen]
    pub fn run_slice(&mut self) -> RunState {
        match self.vm.run_until_block() {
            basil_vm::BlockReason::Completed => RunState::Completed,
            basil_vm::BlockReason::NeedsInput => RunState::NeedsInput,
            basil_vm::BlockReason::Yielded => RunState::Yielded,
        }
    }

    #[wasm_bindgen]
    pub fn provide_input(&mut self, line: String) {
        self.vm.host_mut().provide_input(line);
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub enum RunState {
    Completed,
    NeedsInput,
    Yielded,
}
```

> Your existing VM likely runs “to completion.” The `run_until_block()` pattern is ideal for browsers: you render output, return to the event loop, then resume after input.

## JS (host glue + simple UI)

```html
<script type="module">
import init, { BasilVmHandle } from './pkg/basil_wasm.js';

const out = document.getElementById('out');
const input = document.getElementById('in');
const runBtn = document.getElementById('run');

window.BasilHost = {
  js_println: (s) => { out.value += s + "\n"; out.scrollTop = out.scrollHeight; },
  js_need_input: () => { input.disabled = false; input.focus(); },
  js_rand_u32: () => Math.floor(Math.random() * 0xFFFFFFFF),
  js_now_ms: () => performance.now()
};

await init(); // loads .wasm
const vm = new BasilVmHandle();

runBtn.onclick = async () => {
  out.value = "";
  input.disabled = true;
  const resp = await fetch('examples/hello.basilx');
  const buffer = new Uint8Array(await resp.arrayBuffer());
  await vm.load_bytecode(buffer);

  // pump VM until it asks for input or completes
  let state = vm.run_slice();
  if (state === 'NeedsInput') input.disabled = false;
  else if (state === 'Completed') console.log('done');
};

input.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    const line = input.value;
    input.value = '';
    input.disabled = true;
    vm.provide_input(line);
    const state = vm.run_slice();
    if (state === 'NeedsInput') input.disabled = false;
  }
});
</script>
<textarea id="out" rows="16" cols="80" readonly></textarea><br/>
<input id="in" placeholder="type and hit Enter" disabled />
<button id="run">Run</button>
```

## Build & package

* Add target & tools:

  ```
  rustup target add wasm32-wasi
  cargo install wasm-bindgen-cli wasm-pack
  ```
* In your `basil-wasm` crate:

  ```
  wasm-pack build --target web
  ```
* Serve with any static server (Vite, `python -m http.server`, etc.). Ensure `.wasm` is served with `application/wasm`.

# Deployment options

* **Static hosting:** GitHub Pages, Cloudflare Pages, Netlify, S3+CloudFront.
* **Distribution:** publish an **NPM package** (`@blackrush/basil-wasm`) so people can `import { BasilVmHandle }` directly.
* **CDN**: jsDelivr/UNPKG for demos (`type="module"` import).
* **Web Component** (optional): wrap the VM into a `<basil-runner>` custom element that loads a `.basilx` URL and renders a terminal automatically.

# Nice-to-haves after v1

* **Web Worker** runtime to keep UI super snappy for long runs.
* **xterm.js** terminal with ANSI, copy/paste, and keyboard handling.
* **Virtual FS** backed by IndexedDB/OPFS for `OPEN/INPUT#/PRINT#` style file I/O (when you add it).
* **Step debugger** hooks (`on_breakpoint`, `inspect_stack`, `locals()`) and a little React devpane.
* **Source → bytecode mapping** later for breakpoints in `.basil` source.

---


