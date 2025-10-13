### 🟢 **Junie Prompt: Basil Debug API + Compiler Service**

**Prompt for Junie:**

🚀 **Task:** Extend the Basil VM and Compiler with developer-tooling APIs to support external IDE integration (LSP + DAP).

### 1️⃣ Debug API for Basil VM

Implement a new `basil::debug` module (or feature-gated “debugger” submodule inside the VM) that exposes a clean, thread-safe Rust API for runtime control and inspection.

#### Requirements:

* **Breakpoints**
  
     * `set_breakpoint(file: String, line: usize) -> Result<()>`
  * `clear_breakpoint(file: String, line: usize) -> Result<()>`
  * Maintain internal `HashMap<String, HashSet<usize>>` keyed by filename → lines.
  * When interpreter executes a line that matches a breakpoint, pause and emit a “Stopped(Breakpoint)” event.
* **Execution control**
  
     * `pause()`, `resume()`, `step_in()`, `step_over()`, `step_out()`.
  * Maintain internal “paused” flag and simple call-stack tracking.
* **Inspection**
  
     * `get_call_stack() -> Vec<Frame>` where `Frame { function: String, file: String, line: usize }`.
  * `get_scopes() -> Vec<Scope>` where `Scope { name: String, vars: Vec<Variable> }`.
  * `Variable { name: String, value: String, type_name: String }`
* **Expression evaluation**
  
     * `evaluate(expr: &str) -> Result<String>`: use the Basil expression parser to evaluate in the current scope (read-only, no mutation).
* **Events**
  
     * Define a simple `DebugEvent` enum (`Started`, `Stopped(Breakpoint)`, `Continued`, `Exited`, `Output(String)`) with an async sender channel that external clients can subscribe to.
  * Ensure thread-safety using `Arc<Mutex<…>>` or `RwLock`.
* **Integration**
  
  * Hook breakpoints and stepping into the VM’s main `Interpreter` execution loop so stepping and pausing work seamlessly.
  * Wrap the debugger in a struct:
    
   ```rust
    pub struct Debugger {
        pub breakpoints: HashMap<String, HashSet<usize>>,
        pub state: Arc<Mutex<DebugState>>,
    }
    ```
  * Add debug hooks in the interpreter’s line-execution function, e.g. `check_breakpoint(file, line)` before executing.

### 2️⃣ Compiler Service Endpoint

Add a new **compiler service layer** (for future use by Basil-LSP) that exposes compiler diagnostics and symbol metadata.

#### Requirements:

* Expose function:
   ```rust
   pub fn analyze_source(source: &str, filename: &str) -> CompilerDiagnostics
   ```
* `CompilerDiagnostics` should contain:
  * `errors: Vec<Diagnostic>` where `Diagnostic { message: String, line: usize, column: usize, severity: DiagnosticSeverity }`
    *`symbols: Vec<SymbolInfo>` where `SymbolInfo { name: String, kind: SymbolKind, line: usize, col: usize }`
* Plug into existing Basil lexer/parser to collect:
    * Tokenization or syntax errors.
    * Undefined identifiers, duplicate symbols, etc.
* Return structured JSON via CLI flag `--analyze <file>` for external tools (i.e. `basilc --analyze examples/test.basil`).
* Optional: add a `--json` output mode for easy consumption by IDEs and the upcoming LSP.

### 3️⃣ Deliverables

* New `src/debug/` folder or module with Rust implementation.
* Integration points in VM core loop.
* New compiler service file: `src/compiler/service.rs`.
* CLI hook in `basilc/main.rs` for `--analyze`.
* Thorough inline doc comments explaining how IDEs can use these APIs later.

### 4️⃣ Final Stretch

* Add a minimal test under `tests/debug_api.rs` simulating breakpoint hit and variable inspection.
* Return structured JSON events for `--debug` mode so external DAP clients can attach easily.

✨ **Goal:** After this change, Basil should support external tools (like VS Code or our future Tauri IDE) to (a) inspect parse errors and symbols without running, and (b) attach to a running VM for stepping and breakpoints.
