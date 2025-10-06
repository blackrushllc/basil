# 🧩 Proposal: CLASS Support for Basil

* Author:** Erik Olson
* Target Component:** Basil Compiler / VM / Parser
* Feature Type:** Language Extension
* Purpose:** Introduce lightweight, file-based class objects in Basil for structured modular programming without complexity of full OOP systems.

---

## 1. Overview

This proposal introduces a **`CLASS`** construct to the Basil language.
It replaces legacy mechanisms such as `INCLUDE` or `CHAIN` by providing a modern, modular, and encapsulated way to organize code while preserving the simplicity of BASIC.

A **Class** is defined in its own `.basil` source file and can be precompiled to `.basilx` bytecode using the existing compiler infrastructure. Class files contain **variables** and **functions**, both of which are **public** and accessible from main programs through an instantiated object reference.

Classes are not standalone executable programs; attempting to `RUN` a class file directly will show an error message indicating that it must be used within a main program.

---

## 2. Basic Structure

Each class file is a self-contained `.basil` script with **top-level variable initialization** and **function declarations**.

Example: `myclass.basil`

```basil
REM Example class file

DIM UserNames$(100)
LET Description$ = "Default description"

FUNC AddUser(name$)
  FOR i% = 0 TO UBOUND(UserNames$)
    IF UserNames$(i%) = "" THEN
      UserNames$(i%) = name$
      RETURN
    ENDIF
  NEXT
END FUNC

FUNC CountMyUsers%()
  count% = 0
  FOR i% = 0 TO UBOUND(UserNames$)
    IF UserNames$(i%) <> "" THEN count% = count% + 1
  NEXT
  RETURN count%
END FUNC
```

---

## 3. Usage in Main Program

Classes are instantiated using a new keyword:

```basil
DIM user@ AS CLASS("myclass.basil")
```

If `myclass.basil` is not found, the runtime checks for a precompiled version `myclass.basilx` before throwing an error.

All variables and functions within the class are then accessible via the instance handle (`user@`):

```basil
user@.Description$ = "These are my favorite users"

user@.AddUser("Erik")
user@.AddUser("Junie")
user@.AddUser("ChatGPT")

FOR EACH name$ IN user@.UserNames$()
  PRINTLN name$
NEXT

PRINTLN "Count:", user@.CountMyUsers%()
```

---

## 4. Key Design Rules

| Aspect              | Rule                                                                                                                 |
| ------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **File Location**   | One class per file. Must be `.basil` or `.basilx`.                                                                   |
| **Public Members**  | All variables and functions are public by default.                                                                   |
| **Private Members** | Not supported (future possibility).                                                                                  |
| **Inheritance**     | Not supported. No subclassing or overrides.                                                                          |
| **Constructors**    | No explicit constructor. Top-level statements are executed upon instantiation.                                       |
| **Destructors**     | Not yet supported (future possibility).                                                                              |
| **Name Isolation**  | Class member names do not conflict with global or other class symbols.                                               |
| **Type Reference**  | Always stored in an `@` variable reference.                                                                          |
| **Reassignment**    | Assigning a new class to an existing `@` variable destroys the previous instance.                                    |
| **Execution**       | Running a class file directly produces a compile/runtime error: “This is a Class and must be used within a program.” |

---

## 5. Compiler and VM Integration

### 5.1 Parsing

* The parser must recognize `CLASS(filename$)` as a special object instantiation expression.
* During compile, it marks this as a **ClassReference Node** pointing to a `.basil` or `.basilx` source.
* All member references `obj@.name` are resolved dynamically or via indexed lookup from a compiled class table.

### 5.2 Runtime

* When `CLASS(filename$)` is evaluated:

  1. If a `.basilx` file exists and is newer than the `.basil` source, load it directly.
  2. If only a `.basil` source exists, compile and cache it as `.basilx`.
  3. Execute top-level code to initialize public variables.
  4. Store variable and function tables in a new instance.
* Member resolution follows the pattern:

  * `obj@.VarName` → read/write from instance memory.
  * `obj@.FuncName(...)` → invoke function within that class instance context.

### 5.3 Bytecode Model

* New opcodes may be introduced for:

  * `NEW_CLASS` → loads class definition.
  * `GET_MEMBER` / `SET_MEMBER` → variable access.
  * `CALL_MEMBER` → function call within class scope.
  * `DESTROY_INSTANCE` → clean up references when reassigned.

---

## 6. Error Handling

| Condition                      | Behavior                                                |
| ------------------------------ | ------------------------------------------------------- |
| Missing `.basil` and `.basilx` | `Runtime Error: Class file not found.`                  |
| Attempt to `RUN` a class       | `Runtime Error: Cannot execute a class directly.`       |
| Accessing unknown member       | `Runtime Error: Unknown property or function in class.` |
| Circular class references      | Optional future safeguard. Initially unhandled.         |

---

## 7. Optional Future Enhancements (in the spirit of BASIC)

These can be noted for later milestones:

* **`.toString()` and `.toJSON()`** built-in methods to output variables in declared order.
* **`TYPE` keyword** for lightweight struct-like definitions.
* **Fixed-length strings** for preallocated memory usage (useful for embedded environments).
* **Class-level constants** via `CONST` declarations.
* **`IMPORT CLASS`** directive for compile-time preloading.
* **`FOR EACH VAR IN class@`** enumeration of public members.

---

## 8. Suggested Additional Attributes

These stay faithful to BASIC simplicity:

1. **Default Property Access**

  * Allow reading/writing of a default variable without dot notation.
  * Example:

    ```basil
    PRINTLN user@
    ```

    could implicitly print `user@.description$` if marked as `DEFAULT`.

2. **SAVE/LOAD**

  * Built-in `SAVE class@ TO filename$` and `LOAD class@ FROM filename$` for quick serialization.

3. **CLONE**

  * Built-in function `CLONE(class@)` to duplicate a class instance with all variable values.

4. **INTROSPECTION**

  * Provide optional functions:
    `CLASSVARS(class@)` → list of variable names
    `CLASSFUNCS(class@)` → list of function names

---

## 9. Summary

This feature gives Basil a **modular, modern structure** without losing its BASIC heritage. It allows programs to grow in sophistication, reuse logic, and even distribute precompiled `.basilx` class libraries while maintaining readability and simple syntax.

This document can serve as a **prompt basis for Junie** to:

* Extend Basil’s parser and bytecode compiler.
* Implement runtime object tables.
* Introduce `CLASS()` instantiation semantics.
* Add error handling and optional debug introspection.


