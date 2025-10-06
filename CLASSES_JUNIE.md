### 🧠 Prompt for Junie — Implement CLASS Feature in Basil

> **Project:** Basil Core
> **Feature:** CLASS – File-based lightweight object system
> **Modules:** basil-parser, basil-compiler, basil-vm, basil-bytecode
> **Purpose:** Rather than using `INCLUDE` or `CHAIN`, develop structured, modular, BASIC-friendly class support

---

**Junie, please implement a new language feature called `CLASS` according to the following specification:**

---

### 1. Overview

Implement a lightweight class system for Basil.
Classes are defined in their own `.basil` file, precompiled to `.basilx` if desired, and loaded into a program using this syntax:

```basil
REM myclass.basil is a standalone source file but will be precompiled to bytecode automatically.
DIM user@ AS CLASS("myclass.basil")
```
or

```basil
REM myclass.basilx is precompiled to this bytecode file and we will not look for a source file.
DIM user@ AS CLASS("/lib/myclass.basilx")
```

or

```basil
REM myclass.basil is on another drive or in a different directory, so we must specify the full path.
DIM user@ AS CLASS("E:\basil\classes\myclass.basil")
```
Classes contain only **public variables** and **public functions**, with **no private, protected, or inherited behavior**.
All code outside of function definitions executes at instantiation time and acts as the implicit constructor.

Attempting to `RUN` a class file directly must produce an error message:

```
This is a Class and must be used within a main program.
```

---

### 2. File Structure and Semantics

Each class file:

* Exists as a standalone `.basil` source or `.basilx` bytecode file.
* Contains variable declarations (DIM, LET, etc.) and FUNC definitions.
* Has no `MAIN` or `PROGRAM` entry point.
* May contain top-level executable statements (executed once when instantiated).
* Does not include `CHAIN` or external file references, but can instantiate other classes.

When instantiated via `CLASS("filename")`, the runtime:

1. Searches for `filename.basil` in the current or specified directory.
2. If not found, searches for `filename.basilx`.
3. If neither exists, throws a runtime error: `Class file not found.`
4. Compiles `.basil` to `.basilx` automatically if needed.
5. Creates a new class instance in memory and executes any top-level initialization.

---

### 3. Syntax and Example Usage

```basil
DIM user@ AS CLASS("myclass.basil")

user@.Description$ = "These are my favorite users."

user@.AddUser("Erik")
user@.AddUser("Junie")
user@.AddUser("ChatGPT")

FOR EACH name$ IN user@.UserNames$()
  PRINTLN name$
NEXT

PRINTLN user@.CountMyUsers%()
```

Class file `myclass.basil` example:

```basil
REM UserNames$ and Description$ are member variable of the class instance and global to all functions, but limited in scope to this class
REM UserNames$ and Description$ are publics variable and can be accessed from outside the class using the class@.variableName$ or myclass@.UserName$ syntax.

DIM UserNames$(100)
LET Description$ = "Default description"

REM AddUser() and CountMyUsers%() are public functions and can be accessed from outside the class using the class@.functionName() syntax.

FUNC AddUser(name$)
  REM i% is a local variable and will be destroyed when the function returns.
  REM UserNames$ is a member variable of the class instance and global to all functions, but limited in scope to this class
  FOR i% = 0 TO UBOUND(UserNames$)
    IF UserNames$(i%) = "" THEN
      UserNames$(i%) = name$
      RETURN
    ENDIF
  NEXT
END FUNC

FUNC CountMyUsers%()
  REM count% is a local varaable and will be destroyed when the function returns.
  count% = 0
  FOR i% = 0 TO UBOUND(UserNames$)
    IF UserNames$(i%) <> "" THEN count% = count% + 1
  NEXT
  RETURN count%
END FUNC
```

---

### 4. Parser and Compiler Changes

#### 4.1 Parser

* Add recognition for the keyword `CLASS`.
* The expression `CLASS("filename")` should be parsed as a **ClassInstantiation node**.
* Member access syntax `object@.MemberName` must be parsed as:

    * `MemberAccess { object, name }`
* Support both variable and function references through the same dot-notation.

#### 4.2 AST Additions

* New AST node types:

    * `ExprClassInstantiate { filename: Expr }`
    * `ExprMemberAccess { target: Expr, name: String }`
    * `ExprMemberCall { target: Expr, name: String, args: Vec<Expr> }`

#### 4.3 Bytecode / Compiler

Add new opcodes:

* `NEW_CLASS` → load a `.basilx` or `.basil` file and initialize a class instance.
* `GET_MEMBER` → retrieve variable from the class instance.
* `SET_MEMBER` → assign variable in the class instance.
* `CALL_MEMBER` → invoke a class method.
* `DESTROY_INSTANCE` → deallocate when the `@` variable is overwritten.

Each instance must maintain:

* A variable table (name → value)
* A function table (name → function pointer)
* A context pointer used when executing functions to access its own variables.

---

### 5. VM Runtime Behavior

#### 5.1 Class Instantiation

When executing `NEW_CLASS`:

1. Load and compile the `.basil` file (if `.basilx` missing or stale).
2. Create a new `Instance` object with:

    * `vars: HashMap<String, Value>`
    * `funcs: HashMap<String, FunctionRef>`
    * `scope: LocalScope`
3. Execute top-level statements once to initialize variables.
4. Return a reference handle to the new instance (`Value::InstanceRef`).

#### 5.2 Member Resolution

* `GET_MEMBER` and `SET_MEMBER` operate on the instance’s variable map.
* `CALL_MEMBER` executes the function within the instance context, giving access to its variables.

#### 5.3 Destruction

When the `@` variable referencing an instance is reassigned or goes out of scope:

* The old instance is destroyed.
* Optionally free memory or perform clean-up (no user destructor yet).

---

### 6. Error Handling

| Scenario                      | Message                                                 |
| ----------------------------- | ------------------------------------------------------- |
| Missing class file            | `Runtime Error: Class file not found.`                  |
| Running a class file directly | `Runtime Error: Cannot execute a class directly.`       |
| Unknown member                | `Runtime Error: Unknown property or function in class.` |
| Invalid instantiation syntax  | `Syntax Error: Expected CLASS(filename$)`               |

---

### 7. Optional Features for Future Release

Keep placeholders for future features but do **not implement them yet**:

* `.toString()` and `.toJSON()` special methods.
* `SAVE` / `LOAD` serialization helpers.
* `CLONE(class@)` to duplicate class instance.
* Default property access (e.g., `PRINTLN user@` prints a default field).
* `CLASSVARS()` and `CLASSFUNCS()` introspection helpers.
* Circular reference protection.

---

### 8. Testing and Validation

Implement tests verifying:

1. Loading and instantiating `.basil` and `.basilx` classes.
2. Public variable modification and function calls.
3. Top-level code executes once per instance.
4. Multiple instances of the same class retain independent state.
5. Error messages for invalid operations.

---

### 9. Deliverables

* Updated `basil-parser` with new AST nodes and syntax.
* Updated `basil-compiler` to emit the new opcodes.
* Updated `basil-bytecode` definitions.
* Updated `basil-vm` runtime for instance creation, destruction, and access.
* Unit tests for instantiation, variable access, and function invocation.
* Example programs demonstrating the feature.

---

**End of prompt.**

---

Would you like me to now follow up by drafting a **companion test suite plan** for this feature (showing specific Basil test cases Junie should include for validation)? That’s usually the next step after a feature prompt like this.
