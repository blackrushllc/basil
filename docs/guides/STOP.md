### STOP

Suspends program execution immediately without unloading the program or clearing any state.

Syntax:

- STOP

Behavior:

- When STOP is executed, the VM halts further execution and remains resident in memory.
- All user-defined variables, arrays, objects, functions, and classes remain allocated and accessible.
- Execution can later continue from the instruction immediately after STOP.

Modes:

- CLI mode (basilc cli):
  - If a running program hits STOP, you are returned to the Basil prompt.
  - The program stays loaded; you can inspect variables (e.g., :vars) or evaluate expressions.
  - Type RESUME to continue running from after STOP.

- RUN mode (basilc run file.basil):
  - If STOP is encountered, the process becomes suspended (no prompt) and stays alive until externally terminated.

- TEST mode (basilc test file.basil ...):
  - STOP behaves like EXIT and terminates the process immediately.

Notes:

- STOP may be used anywhere: top-level, inside loops, IF blocks, or functions.
- In this build, inspection in CLI uses a snapshot of globals so you can view state; RESUME continues using the VM’s internal state.

Example:

```
PRINT "Before stop"
X = 123
STOP
PRINT "This line runs after resume"
```

CLI session:

```
> RUN "examples/stop.basil"
Before stop
Program suspended.
> PRINT X
123
> RESUME
This line runs after resume
>
```