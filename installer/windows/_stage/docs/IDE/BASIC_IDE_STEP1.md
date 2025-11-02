BASIC_IDE_STEP1: Debug API + Compiler Service (MVP)

Date: 2025-10-13

Overview
- Added a thread-safe Debug API to the Basil VM that supports breakpoints, stepping, pause/resume, call stack/scopes inspection, and event streaming.
- Added a Compiler Service entry point analyze_source that produces diagnostics and symbol metadata for IDEs.
- Wired both features into the basilc CLI via:
  - --analyze <file> [--json]
  - --debug <file>
- Added an integration test for the Debug API and a PowerShell script to run the test and demo these features.

What’s implemented
1) Debug API (basilcore/vm)
- New module: basilcore/vm/src/debug.rs
  - Debugger: Arc-managed, thread-safe; maintains breakpoints and state internally via Mutex.
  - Breakpoints: per-file HashMap<String, HashSet<usize>>; normalized, case-insensitive on Windows.
  - Execution control: pause(), resume(), step_in(), step_over(), step_out().
  - Inspection types: FrameInfo, Scope, Variable (VM exposes get_call_stack/get_scopes for globals now; can be extended later).
  - Events: DebugEvent enum with a simple mpsc subscription (Started, StoppedBreakpoint, Continued, Exited, Output).
- VM integration (basilcore/vm/src/lib.rs)
  - Emits Started/Exited and Output events.
  - Hooks SetLine to check breakpoints and step
targets via debugger.check_pause_point(); VM waits until resumed.
  - Public methods to attach debugger: set_debugger / with_debugger.

2) Compiler Service (basilcore/compiler)
- New module: basilcore/compiler/src/service.rs
  - analyze_source(&str, &str) -> CompilerDiagnostics
  - Returns structured diagnostics and symbol info (serializable with serde).
  - Current symbol collection: functions, LET bindings, labels (line/col placeholders for now).

3) CLI integration (basilc)
- New flags:
  - --analyze <file> [--json]: runs analyze_source and prints JSON (or text summary).
  - --debug <file>: runs the VM with a Debugger attached and prints JSON events (Started, Output, Exited, etc.).
- Help text updated to include new flags.

4) Tests and scripts
- Test: basilcore/vm/tests/debug_api.rs
  - Builds a small program (SetLine 1; PRINT "Hello"; HALT), attaches a Debugger, sets a breakpoint at line 1, runs the VM, and asserts that Started, Output, and Exited events are observed. (The test resumes execution when the breakpoint is hit.)
- Example script: examples/debug_demo.basil with a single PRINT.
- PowerShell runner: scripts/run_basic_ide_step1_test.ps1
  - Builds the workspace, runs the Debug API test, demonstrates the analyzer, and shows JSON debug events from running --debug.

How to run
- Prerequisites: Rust toolchain, PowerShell, and a Windows shell (paths use backslashes where applicable).

Option A: Single PowerShell script
1) From the repository root, run:
   scripts\run_basic_ide_step1_test.ps1
   - Add -NoBuild to skip the build step.

Option B: Manual steps
1) Run the Debug API test:
   cargo test -p basil-vm --test debug_api -- --nocapture
2) Show compiler analysis for the example file:
   cargo run -p basilc -- --analyze examples\debug_demo.basil --json
3) Run the VM in debug mode and see JSON events:
   cargo run -p basilc -- --debug examples\debug_demo.basil

Notes for IDE/DAP/LSP Integrators
- Subscribe to Debugger events via subscribe() to receive Started/Stopped/Continued/Exited/Output as they occur. Current basilc --debug prints these events as JSON lines for quick prototyping with a DAP client.
- Use the analyze_source function from basilcore/compiler for fast diagnostics, symbol tables, and editor features (hover/completion/definitions will evolve from this).
- VM currently exposes get_call_stack and get_scopes for basic inspection; locals and nested scopes are a future extension.

Files changed/added (high level)
- basilcore/vm/src/debug.rs (new)
- basilcore/vm/src/lib.rs (integrated debugger hooks)
- basilcore/compiler/src/service.rs (new)
- basilc/src/main.rs (CLI flags and debug/analyze commands)
- basilcore/vm/tests/debug_api.rs (new test)
- examples/debug_demo.basil (example program)
- scripts/run_basic_ide_step1_test.ps1 (runner script)

Limitations / Next steps
- Step-in/over/out semantics are basic and rely on frame depth; richer stepping and full call stacks are planned.
- Diagnostics currently lack precise line/column info from the parser; future parser enhancements can feed detailed locations.
- Scope inspection is globals-only; locals and object members inspection to be added.
