This is here for the sake of documentation.

The BMX Mods (Basil Feature Objects) were the first ones made.  All other feature Mods basically follow this pattern.

# Junie Pro task: Add first-class **Object** type support to Basil + two starter objects (BMX Rider & BMX Team)

## Context / Goal

We’re extending the Basil BASIC interpreter (Rust workspace; crates like `basil-lexer`, `basil-parser`, `basil-ast`, `basil-compiler`, `basil-vm`) to support a new **Object** value type and a modular library of built-in objects that can be selectively enabled at build time.

**Surface syntax** (BASIC):

* Variables whose names end with `@` are **objects** (like `$` for strings and `%` for ints).
* Instantiate via `DIM` with an explicit type:
  `DIM r@ AS BMX_RIDER("Alice", 17, "Expert", 12, 3)`
  `DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO)`
* Member access with dot: `r@.Describe$()`, `t@.AddRider(r@)`
* Introspection: `DESCRIBE r@` (prints human help) and `DESCRIBE$(t@)` (returns string)
* Optional compile-time meta: `#USE BMX_RIDER, BMX_TEAM` → validates availability (see Build features).

Please implement the mechanism and provide **two example objects** (detailed below) to serve as reference patterns for future objects (PDO, HTTP, File, Sockets, etc.). Arrays of objects and `FOR/NEXT` iteration over objects are **out of scope for this task** (we’ll do that later), but string-array returns from methods **are in scope**.

---

## High-level architecture

1. **New Object system (core)**

  * Add an `Object` variant to the VM `Value` type, implemented as a handle (`Rc<RefCell<dyn BasicObject>>` or equivalent).
  * Define a trait (e.g., `BasicObject`) that all objects implement:

    * `type_name(&self) -> &str`
    * `get_prop(&self, name: &str) -> Result<Value>`
    * `set_prop(&mut self, name: &str, v: Value) -> Result<()>`
    * `call(&mut self, method: &str, args: &[Value]) -> Result<Value>`
    * `descriptor(&self) -> ObjectDescriptor` (public props/methods metadata)
  * Create a **global registry** (type name → factory) populated only by compiled-in objects. Factories construct new instances from `(args: &[Value])`.

2. **Bytecode & VM ops (generic, no per-object opcodes)**

  * `NEW_OBJ type_id, argc` → pushes an `Object`
  * `GETPROP prop_id` / `SETPROP prop_id`
  * `CALLMETHOD method_id, argc`
  * `DESCRIBE_OBJ` → pushes a string with a formatted description (also expose `DESCRIBE$(obj)` function)
  * The compiler interns member names per **type** and emits `*_id` indices with a string-lookup slow path available if needed.

3. **Syntax & parsing**

  * Recognize identifier suffix `@` for object-typed variables.
  * Grammar for instantiation (use `DIM`):

    ```
    dim_stmt_object
      : 'DIM' ident_object 'AS' TYPE_NAME ['(' arg_list? ')']
    ```

    * Disallow object dimensions for now; parentheses after **type** are **constructor args**.
  * Member access: `primary '.' IDENT '(' arg_list? ')'` for method calls; `primary '.' IDENT` for property reads; `primary '.' IDENT '=' expr` for property writes.
  * Keywords/tokens to add if not present: `AS`, `DESCRIBE`, dot `.`, `#USE` directive handling (compiler-time validation only).
  * Keep `LET v@ = NEW TYPE(args)` **optional** and low-priority: parse it if trivial to add, but `DIM … AS TYPE(args)` is primary.

4. **Compiler / lowering**

  * Emit `NEW_OBJ` for `DIM x@ AS TYPE(args)`.
  * Emit `GETPROP/SETPROP/CALLMETHOD` for member expressions.
  * Emit `DESCRIBE_OBJ` for `DESCRIBE obj@` and a `DESCRIBE$(obj@)` function that returns the descriptor string.

5. **Feature gating & `#USE` meta**

  * Create a new crate `basil-objects` that exports:

    * The shared `BasicObject` trait + descriptor types.
    * A registry builder `register_objects(&mut Registry)` with modules behind **Cargo features**.
  * Features:

    * `obj-bmx-rider`, `obj-bmx-team`, and umbrella `obj-bmx = ["obj-bmx-rider","obj-bmx-team"]`
    * Future: `obj-file`, `obj-pdo`, etc.
  * At runtime startup, the interpreter calls registry population; `#USE` in BASIC triggers a compile-time check that the requested types exist. If missing, produce a friendly error:
    `Type 'BMX_TEAM' not available; rebuild with Cargo feature 'obj-bmx-team'.`

6. **Reflection & docs**

  * `ObjectDescriptor` includes: type name, version, summary, property list (name, type, RW), method list (name, arity, arg names, return type), examples.
  * `DESCRIBE` prints a pretty table; `DESCRIBE$` returns a multi-line string.
  * Add a `HELP "TYPE_NAME"` statement (or function) that prints the type’s descriptor even without an instance.

---

## Starter objects to implement (examples)

### A) `BMX_RIDER` object

**Properties**

* `Name$` (String)
* `Age%` (Integer)
* `SkillLevel$` (String) — e.g., "Novice", "Intermediate", "Expert", "Pro"
* `Wins%` (Integer)
* `Losses%` (Integer)

**Methods**

* `Describe$()` → String — formatted one-line description of the rider.
* `TotalRaces%()` → Integer — `Wins% + Losses%`.
* `WinRate()` → Float — wins / total (0.0 if total = 0).
* `Tags$()` → String[] — small example list (e.g., `[Name$, SkillLevel$]`) to demonstrate returning a string array.

**Self-description**

* `Info$()` → String — return a compact descriptor; also ensure `DESCRIBE r@` prints a full description via `descriptor()`.

### B) `BMX_TEAM` object

**Properties**

* `Name$` (String)
* `EstablishedYear%` (Integer)
* `TeamWins%` (Integer)
* `TeamLosses%` (Integer)
* `Roster` (internal collection of `BMX_RIDER` objects) — not directly exposed as an array for now.

**Constructor**

* `BMX_TEAM(name$, establishedYear%, [flags%])`

  * Provide bitmask constants available to BASIC when `obj-bmx-team` is compiled:

    * `PRO = 1`, `NOT_PRO = 0`
  * Store flags internally (e.g., `IsPro` boolean) and include in descriptions.

**Methods**

* `AddRider(rider@)` → Void
* `RemoveRider(name$)` → Integer (1 if removed, 0 if not found)
* `WinPct()` → Float — `TeamWins% / (TeamWins% + TeamLosses%)` with 0.0 guard
* `TopRider()` → `BMX_RIDER` — best by win rate, break ties by more wins then younger age.
* `BottomRider()` → `BMX_RIDER` — worst by win rate (single rider; returning multiple riders is deferred until arrays of objects are supported).
* `RiderNames$()` → String[] — names of all riders.
* `RiderDescriptions$()` → String[] — `Describe$()` of each rider.

**Self-description**

* `Info$()` → String — return team summary; and ensure `DESCRIBE t@` prints full descriptor.

> Note: Do **not** implement arrays of objects yet. The `TopRider()`/`BottomRider()` return single objects. The plural “bottom riders” use case will come later with object arrays.

---

## BASIC usage examples (for tests & docs)

```basic
#USE BMX_RIDER, BMX_TEAM

DIM r@ AS BMX_RIDER("Alice", 17, "Expert", 12, 3)
PRINT r@.Describe$()          ' "Alice (17, Expert) — 12W/3L, 80.0%"

DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO)
t@.AddRider(r@)
PRINT t@.WinPct()

DIM names$()
names$ = t@.RiderNames$()
FOR i% = LBOUND(names$) TO UBOUND(names$)
  PRINT names$(i%)
NEXT i%

PRINT DESCRIBE$(r@)
DESCRIBE t@
```

---

## Deliverables

1. **Code changes** (create/modify):

  * `basil-lexer`: tokens for `AS`, `DESCRIBE`, dot, `#USE`, and identifier suffix `@`.
  * `basil-ast`: nodes for object instantiation, member get/set, member call, describe.
  * `basil-parser`: parse `DIM x@ AS TYPE(args?)`; parse `obj@.Prop`, `obj@.Prop = expr`, `obj@.Method(args?)`; parse `DESCRIBE`.
  * `basil-compiler`: emit `NEW_OBJ`, `GETPROP`, `SETPROP`, `CALLMETHOD`, `DESCRIBE_OBJ`. Intern member names by type.
  * `basil-vm`: implement the new opcodes and `Value::Object`.
  * New crate `basil-objects`:

    * `lib.rs` with `BasicObject` trait, `ObjectDescriptor`, registry types and builder.
    * `bmx_rider.rs`, `bmx_team.rs` behind features `obj-bmx-rider`, `obj-bmx-team`.
    * `mod.rs` wires registration based on features; umbrella `obj-bmx`.
  * Host/bootstrap: on VM startup, call registry population; wire `#USE` validation in compiler.

2. **Feature flags**

  * `basil-objects/Cargo.toml`:
    `features = { "obj-bmx-rider" = [], "obj-bmx-team" = [], "obj-bmx" = ["obj-bmx-rider","obj-bmx-team"] }`
  * The workspace compiles with zero objects by default; CI matrices include `--features obj-bmx` runs.

3. **Introspection**

  * Pretty printer for `DESCRIBE obj@` and function `DESCRIBE$(obj@)`.
  * Optional `HELP "TYPE_NAME"` to print descriptor without an instance.

4. **Tests**

  * Lexer/parser tests for new tokens and grammar (incl. negative cases like `DIM x@` without `AS`).
  * Lowering tests asserting opcode sequences for `DIM … AS TYPE(args)` and member calls.
  * VM tests for both example objects:

    * Property get/set
    * String/Int/Float return types
    * String array returns (`Tags$()`, `RiderNames$()`, `RiderDescriptions$()`)
    * `TopRider()`/`BottomRider()` behavior and tie-breaks
    * `DESCRIBE` output smoke tests
  * Feature-gated tests: ensure clean failure messages if `#USE BMX_RIDER` is present but features are off.

5. **Docs**

  * `basil-objects/README.md` explaining how to add a new object type (step-by-step) with annotations.
  * `docs/language/objects.md` covering `@` suffix, `DIM … AS TYPE(args)`, member access, `DESCRIBE`, `#USE`, and examples above.

6. **Developer ergonomics**

  * Provide a minimal derive or helper macro (optional for this PR) to reduce boilerplate for property/method tables.
  * Consistent error messages with type/method/property names in context.

---

## Constraints & quality bar

* Backwards compatible with existing String `$`, Integer `%`, Float (no suffix) variables and arrays.
* No arrays of objects yet; do not change `DIM x() …` semantics for primitives.
* Clean failure if an object type is referenced but not compiled in (point to the correct Cargo feature).
* Clear, friendly error messages for bad member names or wrong arity/types.
* Keep execution on the fast path via interned member ids; support a slow path fallback if necessary.
* Security: nothing in BMX requires I/O; it’s pure compute.

---

## Acceptance checklist

* [ ] `cargo test` passes with and without `--features obj-bmx`.
* [ ] Running a sample script with the examples above produces sane output.
* [ ] `DESCRIBE r@` and `DESCRIBE t@` show properties/methods.
* [ ] Disabling `obj-bmx-team` causes `#USE BMX_TEAM` to error with a helpful message.
* [ ] Code is organized (`basil-objects` crate, features, registry) so adding `FILE`/`PDO` later is straightforward.

When finished, please:

* Summarize changes PR-style with a list of key files touched.
* Include brief examples in the docs and in the commit message.
* Call out any follow-ups needed for “arrays of objects & iteration” so we can schedule the next task.

---
