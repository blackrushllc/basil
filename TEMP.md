# Feature request: Arrays of objects + `FOR EACH` iteration

> **Title:** Add arrays of objects + `FOR EACH` iteration to Basil (arrays & iterables)

**Context**
Basil already supports: strings `$`, integers `%`, floats (no suffix), arrays of those with inclusive upper bounds; objects (suffix `@`) with `DIM v@ AS TYPE(args)`; string-array returns from object methods; and `DESCRIBE`. We now want:

1. Arrays **of objects** (typed and untyped).
2. A `FOR EACH … IN … NEXT` construct that iterates over arrays (any element type) and over iterable objects.

**Scope & requirements**

1. **Syntax & parsing**

  * Add `FOR EACH` form:

    ```
    for_each_stmt
      : 'FOR' 'EACH' ident 'IN' expr block ('NEXT' ident?)    # canonical
      | 'FOREACH' ident 'IN' expr block 'ENDFOR'              # alias (optional)
    ```

    * `ident` must use the correct suffix for the element type (`$`, `%`, none, `@`).
    * Keep classic `FOR var% = … TO … [STEP …] … NEXT [var%]` intact.
  * Arrays of objects:

    * `DIM riders@(N) AS BMX_RIDER`  (typed)
    * `DIM bag@(N)`                  (untyped “OBJECT[]”, accepts any object)
    * Multi-dim object arrays allowed; `FOR EACH` enumerates row-major across all elements.
  * Expressions:

    * Add `NEW TYPE(args)` as an **expression** (keeps `DIM v@ AS TYPE(args)` for scalars).
      Example: `LET riders@(0) = NEW BMX_RIDER("Alice", 17, "Expert", 12, 3)`.

2. **Type system & checks**

  * Arrays carry `elem_kind` = {STRING, INTEGER, FLOAT, OBJECT(type_id?)}.
  * Typed object arrays store `type_id` (e.g., `BMX_RIDER`).
  * Assign into typed object arrays: enforce exact type match (no subtyping yet).
  * Untyped object arrays (`OBJECT[]`) accept any `Object` value.
  * `TYPE$(expr)` returns `"BMX_RIDER[]"`, `"OBJECT[]"`, etc.

3. **IR/bytecode & VM**

  * Add generic enumerator opcodes:

    * `ENUM_NEW`     (expects iterable on stack; pushes enumerator handle or error)
    * `ENUM_MOVENEXT`(pop/enumerator or read top? returns bool on stack)
    * `ENUM_CURRENT` (yields current element `Value`)
    * `ENUM_DISPOSE` (best-effort cleanup; no-op for arrays)
  * Compiler lowering for `FOR EACH`:

    * Evaluate enumerable `expr`, `ENUM_NEW`.
    * Label `test`: `ENUM_MOVENEXT`; if false → jump `end`.
    * `ENUM_CURRENT` → assign into loop variable (type-check) → emit body → jump `test`.
    * Bind `end`; `ENUM_DISPOSE`.
  * Array enumerators: built-in implementation for primitive and object arrays (row-major for multi-dim).
  * Iterable objects:

    * Extend `BasicObject` with an optional iterator hook:
      `fn get_enumerator(&mut self) -> Option<Box<dyn BasicEnumerator>>`
      where `BasicEnumerator` provides `move_next() -> bool`, `current() -> Value`, `dispose()`.
    * If `expr` is an object: try `get_enumerator()`. If `None`, error: *“object is not iterable”*.
  * Disallow `DIM/REDIM/ERASE` on the **same array** while an active enumerator exists; raise a runtime error.

4. **Arrays of objects: runtime**

  * Store elements as handles; array copy copies handles (not deep copy).
  * `DESCRIBE <array>` prints: kind, element type (if known), dims, bounds, size.
  * `LEN(array)` continues to return total element count (as in your samples).
  * Keep your existing `SIZE/LBOUND/UBOUND` decisions consistent if you added them.

5. **Errors & messages (exact wording near)**

  * Non-iterable in `FOR EACH`:
    `FOR EACH expects an array or iterable object after IN (got TYPE=<Type$>).`
  * Loop variable mismatch:
    `Loop variable 'v@' must be an object to iterate OBJECT[] (got FLOAT).`
  * Typed object array assignment mismatch:
    `Expected BMX_RIDER in riders@(), got BMX_TEAM.`
  * Shape change during enumeration:
    `Cannot DIM/REDIM/ERASE array 'riders@' during active FOR EACH enumeration.`

6. **Tests**

  * Parser: positive/negative for `FOR EACH`, `FOREACH/ENDFOR` alias (if implemented), object-array `DIM`.
  * VM:

    * `FOR EACH` over `String[]`, `Integer[]`, `Float[]`.
    * `FOR EACH` over typed `BMX_RIDER[]` and untyped `OBJECT[]`.
    * Iterating `t@.RiderNames$()` (String[]) and `t@.RiderDescriptions$()` (String[]).
    * Iterating a **custom iterable object** (add a minimal test stub or adapt `BMX_TEAM.Roster()` to return an iterable object or an array; either is fine).
    * Multi-dim arrays enumerate all elements in row-major order.
    * Runtime errors: non-iterable in `IN`, suffix mismatch, shape-change during loop.
  * Performance: simple micro test to ensure `FOR EACH` over 100k primitive elements does not regress.

7. **Docs**

  * `docs/language/arrays.md`: add typed object arrays; rules for deep vs reference copy.
  * `docs/language/loops.md`: add `FOR EACH … IN … NEXT`, examples for primitives and objects.
  * `docs/language/objects.md`: show `NEW TYPE(args)` expression for array element creation; show `TYPE$`/`DESCRIBE` outputs.

8. **Backward compatibility**

  * Do **not** change existing `FOR … TO … NEXT` semantics.
  * `NEXT var` remains accepted in both loop forms (optional variable name).

**Usage examples for docs/tests (BASIC)**

```
#USE BMX_RIDER, BMX_TEAM

DIM riders@(3) AS BMX_RIDER
LET riders@(0) = NEW BMX_RIDER("Alice", 17, "Expert", 12, 3)
LET riders@(1) = NEW BMX_RIDER("Bob",   21, "Expert",  8, 9)
LET riders@(2) = NEW BMX_RIDER("Carol", 19, "Pro",    30, 4)

FOR EACH r@ IN riders@
  PRINT r@.Describe$()
NEXT

DIM nums%(4)         ' 0..4
FOR EACH n% IN nums%
  LET nums%(n%) = n% * n%
NEXT

DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO)
t@.AddRider(riders@(0)) : t@.AddRider(riders@(1)) : t@.AddRider(riders@(2))

FOR EACH name$ IN t@.RiderNames$()
  PRINT name$
NEXT

FOR EACH desc$ IN t@.RiderDescriptions$()
  PRINT desc$
NEXT
```

**Acceptance checklist**

* [ ] `FOR EACH` works for primitive arrays, typed object arrays, untyped object arrays, and iterable objects.
* [ ] `NEW TYPE(args)` expression usable for array element construction.
* [ ] Clear runtime errors for non-iterable `IN`, type/suffix mismatches, and shape-changes during enumeration.
* [ ] `DESCRIBE` and `TYPE$` show array element types and dims.
* [ ] CI passes with and without object features enabled.


