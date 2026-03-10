### Summary of EXIT vs. STOP Implementation in Basil

```FYI when testing you should always using EXIT instead of STOP.  Exit terminates the program and returns to the command line, whereas STOP suspends execution of the program but does not terminate.  The purpose of STOP is to leave the program in memory to process events or other triggers that we may add on later for making GUI or Daemon programs```

The Basil language distinguishes between `EXIT` and `STOP` to support different program lifecycles, particularly for command-line tools versus event-driven or GUI applications.

#### 1. EXIT Statement
- **Behavior**: `EXIT [code]` immediately terminates the entire process and returns control to the operating system with the specified (or default `0`) numeric exit code.
- **Implementation**:
  - The compiler translates `EXIT` into a call to **Builtin 61** with one argument (the exit code).
  - The VM (in `basilcore\vm\src\lib.rs`) handles this by calling `std::process::exit(code)`.
- **Usage**: Use `EXIT` when the program's task is complete and you want to close the terminal or return to the shell.

#### 2. STOP Statement
- **Behavior**: `STOP` suspends the execution of the Basil program's instruction stream but does **not** terminate the process. The program remains in memory, preserving its current state (variables, stacks, open handles).
- **Implementation**:
  - The compiler translates `STOP` into the **`Op::Stop`** opcode.
  - The VM sets its `suspended` flag to `true` and returns from the execution loop.
  - In the CLI runner (`basilc`), reaching a suspended state causes the runner to enter a long-running sleep loop (`std::thread::sleep`), keeping the process alive.
  - In the embedded environment (`basil-embed`), the suspended VM is stored in the current `Session`, allowing it to be resumed later to process events or triggers.
- **Usage**: Use `STOP` for GUI applications, background daemons, or programs that need to wait for asynchronous events after the main execution flow has reached a logical pause point.

### Verification of Current Codebase
- The current implementation in `basilcore\vm\src\lib.rs` and `basilc\src\main.rs` aligns perfectly with your instructions.
- All test scripts (e.g., `test_unwind.basil`, `test_unwind2.basil`) have been verified to use `EXIT` at the end to ensure clean termination when run from the command line.
- Programs without an explicit `EXIT` or `STOP` at the end will terminate naturally when they reach the end of the script, which is the preferred behavior for standard CLI scripts.
