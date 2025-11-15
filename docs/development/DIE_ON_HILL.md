You are working in a Rust-based BASIC interpreter project.
There are **two sibling projects** that are almost identical:

* **Basil** (full-featured): interpreter + compiler + web server.
* **Basic** (stripped-down “naked” BASIC): interpreter only.

This prompt is designed to work in either repo.
When I run it in one repo, treat *that* repo as the active project and ignore the other.

You are working on our BASIC dialect and its Rust-based cousin Basil. Please implement a small set of language improvements in **both** projects, keeping their syntax identical where possible.

## High-level goals

We want to:

1. Add a `CONST` declaration:
   - `CONST NAME = value` (no type suffix in the name).
   - Value can be string, integer, or float.
   - Constants should be immutable and have the same scope rules as regular variables (module-level by default in our dialect).

2. Upgrade `DIM` to:
   - Allow multiple variables on a single line.
   - Initialize them to reasonable defaults (0 for numeric, empty string for string, etc.).
   - This is mostly preparatory for a future strict mode / more formal scoping.

3. Allow **implicit LET**:
   - Assignments can omit the `LET` keyword when the statement clearly starts with a variable assignment.

4. (Optional / exploratory) Investigate making `BEGIN` **optional** for multi-line blocks such as `IF ... THEN` / `END IF`.
   - If this is too invasive right now or risks breaking existing code, leave TODOs and comments but don’t force the change.

Please keep these changes consistent between the “classic” BASIC interpreter and the Rust-based Basil interpreter.

---

## 1. Add CONST declarations

### Desired syntax

```basic
CONST DEFAULT_OS = "L"
CONST MAX_RETRIES = 3
CONST PI = 3.14159
````

**Rules:**

* **No type suffix** on the constant name:

    * `CONST MAX_RETRIES = 3` ✅
    * `CONST MAX_RETRIES% = 3` ❌ (don’t support this, or treat as a syntax error).
* The type is inferred from the literal assigned:

    * `"string"` → string
    * integer literal → integer
    * float literal → float
* Scope:

    * Constants declared at **module level** behave like module-level globals (same as normal variables in module-level code).
    * If we already support local variables inside `SUB`/`FUNCTION`, also allow local constants there, with the same scoping rules as local variables in that block.
* **Immutability**:

    * Reassigning a constant should be rejected (ideally at compile/parse time, or at runtime with a clear error).
    * Examples that must be illegal:

      ```basic
      CONST MAX_RETRIES = 3
      LET MAX_RETRIES = 4      ' should be an error
      MAX_RETRIES = 4          ' should be an error (with implicit LET as well)
      ```

### Implementation notes

For **both** interpreters:

* Find the parser / grammar code that handles declarations and top-level statements.
* Add a new “const declaration” statement form:

    * Syntax: `CONST` <identifier-without-type-suffix> `=` <expression>
* Disallow a type suffix on the identifier in this context.
* In the symbol table / environment:

    * Add a flag on symbols to mark them as **constant**.
    * On assignment, check for this flag and raise an error if someone tries to change a const.
* Make sure const values are evaluated once (constant expression). For now, it’s acceptable if the right-hand side is any expression; we don’t need full constant folding yet, but:

    * If possible, prefer to evaluate the expression once at declaration time.
    * If not practical, evaluating it on first access is acceptable as long as it can’t be changed.

Please also:

* Add tests / sample code that demonstrate:

    * Declaration of string, int, and float constants.
    * Module-level constants used inside `SUB`/`FUNCTION`.
    * Attempted reassignment causing a clear error.

---

## 2. Upgrade DIM to allow multiple variables & default initialization

We want to support:

```basic
DIM osChoice$, itemChoice$, targetDir$
DIM ok%
```

**Behavior:**

* `DIM` should accept **one or more variable names**, separated by commas, each optionally having a type suffix (`$`, `%`, etc.) or array dimensions (if we already support arrays in DIM).
* For now, this is largely a no-op in our current semantics because variables auto-create on first use, but we want:

    * To **formally recognize** the declarations, and
    * To **initialize** variables to default values:

        * String variables (`$`) → `""`
        * Integer variables (`%`) → `0`
        * Float / un-suffixed → `0` (float or default numeric type used by the interpreter)
        * Arrays (if supported) → allocate with default element values.

This is groundwork for a future strict mode where:

* using an undeclared variable could be an error or warning.

### Implementation notes

For **both** interpreters:

* Extend the `DIM` parsing rule to support:

  ```ebnf
  DimStmt ::= "DIM" DimItem ( "," DimItem )*
  DimItem ::= Identifier[TypeSuffixOrArray]
  ```

* Ensure that for each `DimItem` we:

    * Look up or create the symbol in the current scope.
    * If newly created, assign the correct default value (and allocate arrays, if arrays are supported).
    * If it already exists:

        * For now, do nothing special (don’t error; we can tighten this in strict mode later).

* Make sure this works both at module level and inside `SUB`/`FUNCTION` scopes (if we have them).

Add tests/examples:

```basic
DIM a$, b$, c$
DIM i%, j%
DIM x, y

PRINT a$    ' should print empty string or nothing
PRINT i%    ' should print 0
PRINT x     ' 0 or default numeric value
```

---

## 3. Allow implicit LET for assignments

Right now, we require:

```basic
LET osChoice$ = PromptChoice$("Select OS (W)indows (L)inux or (M)ac <L> : ", OS_LINUX, "WLM")
```

We want to also allow:

```basic
osChoice$ = PromptChoice$("Select OS (W)indows (L)inux or (M)ac <L> : ", OS_LINUX, "WLM")
```

**Rules:**

* If a statement starts with a valid variable name (including type suffix, e.g. `osChoice$`) followed by `=`, treat it as an assignment **even if** `LET` is omitted.

* `LET` remains supported for backward compatibility:

  ```basic
  LET x% = 1
  x% = 1      ' both should work
  ```

* Attempting to assign to a `CONST` via implicit assignment must still be rejected (see section 1).

### Implementation notes

For **both** interpreters:

* In the statement parser, change the grammar to something like:

  ```ebnf
  Statement ::=
      LetAssignmentStmt
    | AssignmentStmtWithoutLet
    | IfStmt
    | WhileStmt
    | ForStmt
    | CallStmt
    | PrintStmt
    | ...
  ```

  where:

  ```ebnf
  LetAssignmentStmt        ::= "LET" IdentifierWithOptionalType "=" Expression
  AssignmentStmtWithoutLet ::= IdentifierWithOptionalType "=" Expression
  ```

* Be careful not to misinterpret function calls as assignments:

    * E.g. `Foo(1, 2)` should still be parsed as a call.
    * Only treat it as assignment if the first token is a plain identifier (with optional type suffix) followed directly by `=`.

* Make sure this interacts correctly with constants:

    * If the identifier refers to a const, reject the assignment with a clear error.

Add tests/examples:

```basic
DIM x%
x% = 5
PRINT x%       ' 5

LET y = 10
y = 20
PRINT y        ' 20

CONST LIMIT = 3
LIMIT = 4      ' should be an error
```

---

## 4. (Exploratory) BEGIN-less blocks for multi-line IF and other constructs

Current requirement (simplified):

```basic
IF targetDir$ = "" THEN BEGIN
    PRINT "No target directory specified. Aborting."
    EXIT SUB
END IF
```

or:

```basic
IF targetDir$ = "" THEN
    BEGIN
        PRINT "No target directory specified. Aborting."
        EXIT SUB
END IF
```

We would *like* to support a more classic BASIC style:

```basic
IF targetDir$ = "" THEN
    PRINT "No target directory specified. Aborting."
    EXIT SUB
END IF
```

**Important**: This is **optional and exploratory**. If it turns out to be too tightly coupled to the existing block parser (where `BEGIN`/`END` act like `{}`), then:

* Do not ship a half-working version.
* Prefer to leave:

    * Clear TODO comments,
    * A small design note describing what would need to change, and
    * Possibly a feature flag or parser hook that we can revisit later.

### Desired behavior (if feasible)

* Still support the existing `BEGIN`/`END` style for backwards compatibility.

* Additionally support “implicit blocks” for constructs like:

  ```basic
  IF cond THEN
      ' body...
  END IF

  WHILE cond
      ' body...
  WEND or END WHILE (depending on what we use)

  SUB Foo()
      ' body...
  END SUB
  ```

* In other words, treat:

  ```basic
  IF cond THEN BEGIN
      ...
  END IF
  ```

  and

  ```basic
  IF cond THEN
      ...
  END IF
  ```

  as equivalent.

### Implementation sketch

ONLY do this if the parser structure allows it without massive surgery.

General idea:

* Wherever we parse a **block** now, we currently expect something like `BEGIN ... END` (or the internal equivalents).
* Enhance the block parser so that:

    * It recognizes the existing `BEGIN ... END` braced form, **and also**
    * Recognizes an implicit “block until matching END tag” form when `BEGIN` is omitted.
* For IF specifically:

    * After parsing `IF <expr> THEN`:

        * If the next token is `BEGIN`, parse a braced block as we do now.
        * Otherwise, parse a block that continues until `END IF` (handling `ELSE`/`ELSEIF` as needed, if we support them).

If this requires making the parser newline-sensitive in ways that conflict with the rest of the language, or if it risks breaking existing behavior, then:

1. Document what you found in a short design note / comment block.
2. Leave the current `BEGIN` requirement in place.
3. Add a TODO and maybe a stub function/enum that we can later use to switch between “braced blocks only” and “implicit or braced blocks”.

---

## 5. Tests, examples, and documentation

For **both** projects, please:

1. Add or update tests that cover:

    * `CONST` declarations of all supported types.
    * Attempted reassignment of consts.
    * `DIM` with multiple variables and default initialization.
    * Assignments with and without `LET`.
    * (If implemented) BEGIN-less IF blocks, ensuring backward compatibility with the existing `BEGIN ... END` style.

2. Add or update language documentation / reference files to describe:

    * The `CONST` syntax and semantics.
    * The upgraded `DIM` behavior (multiple vars, defaults).
    * The fact that `LET` is now optional for assignments, but still allowed.
    * The current status of `BEGIN` (required vs optional); if BEGIN-less blocks are not yet implemented, mention that as a future enhancement.

3. Keep BASIC and Basil behavior aligned:

    * Update both codebases so that user-facing syntax and semantics match.
    * If something must differ, document it very clearly.

Please implement these changes in small, well-structured commits with clear messages so they’re easy to review.

