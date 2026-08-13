# AtomicBASIC

## Purpose

AtomicBASIC is a restricted Basil-related language profile intended for interactive AtomicFlix games. It is designed to be compiled into a lightweight JSON-based Intermediate Representation (IR) that can be executed by virtual machines on various platforms, including Roku and web browsers.

## Current Phase

Basil currently defines the contracts and validation logic for AtomicBASIC:

- **Manifest contract**: Metadata for identifying and loading games.
- **Game IR contract**: A set of 13 operations for game logic and UI.
- **VM operation names**: Standardized names for all instructions.
- **VM state names**: Execution states for the virtual machine.
- **Runtime-error structure**: Standardized error reporting.
- **Validation**: Structural and logical validation of Game IR.
- **Future target placeholder**: `basilc --target atomic` stub.

**Basil does not yet compile AtomicBASIC source.** The current implementation focused on establishing the shared bridge between the compiler and the runtimes.

## Architecture

```text
AtomicBASIC Source
        |
        v
Basil Compiler               <-- future
        |
        v
AtomicFlix Game IR (JSON)
       / \
      /   \
 Roku VM  Browser VM
```

## Initial Operations

The following 13 operations are supported in AtomicBASIC IR v1:

1.  `LABEL`: Defines a jump target.
2.  `LAYOUT`: Sets the screen layout (e.g., `fullscreen_text`).
3.  `IMAGE`: Displays an image asset.
4.  `TITLE`: Sets the scene title.
5.  `TEXT`: Displays descriptive text.
6.  `CHOICE`: Defines a menu option with a jump target.
7.  `WAITCHOICE`: Pauses execution until a choice is made.
8.  `GOTO`: Unconditional jump to a label.
9.  `END`: Terminates program execution.
10. `SET`: Assigns a value to a variable.
11. `ADD`: Adds a numeric value to a variable.
12. `IF_EQ`: Conditional jump if a variable equals a value.
13. `WAITKEY`: Pauses execution until a key is pressed.

## Indexing Convention

- **Instruction indexes**: Zero-based.
- **Source line numbers**: One-based.

## Versioning

- **Manifest format version**: 1
- **Game IR version**: 1
- **Runtime version**: 1
