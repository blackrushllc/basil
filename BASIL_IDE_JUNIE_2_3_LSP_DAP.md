### This doc conatains **TWO (2)

** prompts for Junie with no blockquote indentation. First builds the LSP server, second builds the DAP server, both
consuming the Debug API and Compiler Service you just asked her to add.

# Junie Prompt: Build `basil-lsp` (Language Server)

Task: Create a Rust Language Server for the Basil language so any LSP-capable editor (VS Code, JetBrains LSP, Vim,
Geany) gets diagnostics, hovers, completions, go-to-definition, and document symbols. The server must consume the new
Compiler Service (`analyze_source`) and expose a fast, robust LSP using `tower-lsp`.

## Goals

* New crate: `crates/basil-lsp/`
* Binary: `basil-lsp` (invoked by editors)
* Provide: diagnostics, hover, completion, definition, document symbols, formatting stub (no-op), and configuration
  reload
* Zero editor lockups; all heavy work off the main thread
* Works cross-platform (Windows/macOS/Linux)

## Dependencies

* `tower-lsp = "0.20"` (or latest)
* Reuse Basil core crates for lexer/parser and `analyze_source`
* For config: read a simple optional `basil-lsp.toml` in project root (e.g., keyword casing preferences, include paths)

## Protocol Surface (MVP)

* initialize/initialized/shutdown/exit
* textDocument/didOpen|didChange|didSave: re-run `analyze_source`
* textDocument/publishDiagnostics: map `CompilerDiagnostics` to LSP diagnostics
* textDocument/hover: show brief symbol/kind/type if resolvable; otherwise nearest token info
* textDocument/completion: return keywords + visible symbols (from last analysis)
* textDocument/definition: jump to symbol definition if available from symbol table
* textDocument/documentSymbol: top-level functions/subs/labels/consts
* workspace/didChangeConfiguration: re-read `basil-lsp.toml`

## Implementation Notes

* Structure:

    * `crates/basil-lsp/src/main.rs` – launches server
    * `crates/basil-lsp/src/server.rs` – LSP handlers
    * `crates/basil-lsp/src/analysis.rs` – thin wrapper around `analyze_source`
    * `crates/basil-lsp/src/symbols.rs` – convert Basil symbol info → LSP symbols
    * `crates/basil-lsp/src/completion.rs` – keyword + symbol completion
* Keep an in-memory per-document cache: last text, last `CompilerDiagnostics`, last symbol table
* Provide a `--stdio` mode (default) and `--tcp 127.0.0.1:9465` for debugging the server
* Map Basil severities to LSP: Error, Warning, Information

## Deliverables

* Source files above
* `Cargo.toml` with bin target `basil-lsp`
* Integration tests (can be lightweight) that feed a small Basil file and assert diagnostics
* README: how to point VS Code at this server using a minimal extension (see “Editor Integration” below)
* Example `basil-lsp.toml`

## Editor Integration (VS Code minimal extension)

Create a folder `editors/vscode-basil/` with:

* `package.json` (activation on `language:basil`)
* `extension.js` that spawns `basil-lsp` with `stdio`
* `syntaxes/basil.tmLanguage.json` or Monarch grammar placeholder (ok to include a starter keyword list)
* Optional: contributes configuration schema for `basil-lsp.toml`

## Commands to Run

* Build: `cargo build -p basil-lsp`
* Run (stdio): `basil-lsp`
* Run (tcp): `basil-lsp --tcp 127.0.0.1:9465`

## Acceptance Criteria

* Opening a `.basil` file in VS Code with the sample extension shows live diagnostics from `analyze_source`
* Hover over identifiers shows symbol kind (e.g., Function, Const) and location if known
* Typing a keyword triggers completion
* Document outline lists top-level items
* No panics on rapid edits or empty files

# Junie Prompt: Build `basil-dap` (Debug Adapter)

Task: Create a Rust Debug Adapter Protocol server that bridges the Basil VM Debug API to standard DAP. Any DAP-capable
editor can set/clear breakpoints, run/continue, step in/over/out, and inspect stack/scopes/variables. Read-only
expression evaluation is supported.

## Goals

* New crate: `crates/basil-dap/`
* Binary: `basil-dap`
* Transport: stdio by default; optional TCP for debugging the adapter
* Launch modes:

    * `launch`: run a Basil program (path provided by client)
    * `attach`: attach to an already-running Basil VM (optional; can be a stub if you don’t expose external attach yet)

## Dependencies

* Use `tokio` + `serde`/`serde_json`
* Optional: `dap` or write a small message loop (request/response/event) per DAP spec
* Depend on Basil VM with the new Debugger API:

    * set/clear breakpoints
    * pause/resume
    * step_in/step_over/step_out
    * get_call_stack/get_scopes/variables
    * evaluate(expr)

## Message Map (MVP)

Requests to implement:

* initialize → capabilities: supportsConfigurationDoneRequest, supportsEvaluateForHovers
* launch (program, args?, cwd?, stopOnEntry?) → start Basil VM in debug mode; emit “stopped” if stopOnEntry
* setBreakpoints (by file:lines) → reflect to Debugger; return verified breakpoints
* configurationDone → ready to run if stopOnEntry
* threads → return a single thread (id 1) for the VM
* stackTrace → map VM `Frame` list
* scopes → return e.g. “Locals”, “Globals”
* variables → expand scope variables into DAP Variables (name, value, type, variablesReference=0)
* continue → resume VM
* next/stepIn/stepOut → stepping
* pause → pause VM
* evaluate (context=“repl”|“hover”) → call Debugger.evaluate (read-only)

Events to emit:

* initialized
* stopped (reason: breakpoint/step/entry)
* continued
* output (category: stdout/stderr) when VM prints output
* terminated/exited (exitCode)

## Implementation Notes

* Structure:

    * `crates/basil-dap/src/main.rs` – DAP loop (stdio + optional `--tcp`)
    * `crates/basil-dap/src/adapter.rs` – request routing, state machine
    * `crates/basil-dap/src/convert.rs` – VM ↔︎ DAP type mapping
    * `crates/basil-dap/src/vm_host.rs` – wrapper that launches Basil VM in debug mode and wires events
* Maintain a small registry for breakpoints per file
* Keep a stable thread id (1) unless Basil introduces true multi-threading
* Ensure that when VM reports `DebugEvent::Stopped(Breakpoint)`, the adapter emits a DAP `stopped` event with
  `allThreadsStopped: true`

## Launch JSON (example for VS Code)

Provide an example in `editors/vscode-basil/.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "basil",
      "request": "launch",
      "name": "Basil: Launch Program",
      "program": "${file}",
      "cwd": "${workspaceFolder}",
      "stopOnEntry": false
    }
  ]
}
```

Also include `debuggers` contribution in `package.json` to register `"type": "basil"` and point to the `basil-dap`
executable (stdio).

## Commands to Run

* Build: `cargo build -p basil-dap`
* Run (stdio): `basil-dap`
* Run (tcp): `basil-dap --tcp 127.0.0.1:9485`

## Deliverables

* Source files above
* `Cargo.toml` with bin target `basil-dap`
* README with usage and example `launch.json`
* A tiny `examples/hello.basil` for debugging
* Smoke tests that simulate `initialize → setBreakpoints → launch → continue` and assert a `stopped` event

## Acceptance Criteria

* Can set a breakpoint in the current file, hit it, inspect Locals/Globals
* Can step over/in/out and see stack/line update
* `evaluate` returns stringified value for simple expressions in current scope
* Clean termination and no orphaned VM processes on stop

# Monorepo Wiring (both servers)

* Add workspace members in root `Cargo.toml`:

    * `crates/basil-lsp`
    * `crates/basil-dap`
* Ensure both compile on stable Rust, no nightly features
* CI job (optional) to build both on Windows/macOS/Linux

# Nice-to-haves (optional if time remains)

* In `basil-lsp`, add basic semantic tokens (keywords, numbers, strings, comments) once the Monarch grammar stabilizes
* In `basil-dap`, support `disassemble` (no-op or simple listing) and `setExceptionBreakpoints` (ignored for now)
* Add a unified `editors/vscode-basil` command palette: Basil: Run, Basil: Debug, Basil: Build (calling your `bcc`
  pipeline)
