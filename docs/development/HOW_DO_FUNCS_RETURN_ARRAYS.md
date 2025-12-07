### Short answer
Yes — your BASIC interpreter already supports returning arrays (single- and multi‑dimensional) from functions, and function return types are dynamic. You do not need any new syntax such as `MyFunc[]` or `MyFunc[][]`, and you don’t need a type identifier on the function name. A plain `FUNC MyFunc()` can return any value type, including arrays.

### Why this works today (with code pointers)
- Functions have no declared return type. A `RETURN` in a function simply evaluates whatever expression you give it and returns that value.
  - See compiler: `Stmt::Return(eopt)` pushes the expression (or `Null`) and emits `Ret` (basilcore/compiler/src/lib.rs around lines 1372–1381).
- The runtime value type includes arrays (and more), so any function can return an array value:
  - `Value::Array(Rc<ArrayObj>)` and also a special `Value::StrArray2D` helper exist (basilcore/bytecode/src/lib.rs lines 96–112).
- Arrays are true multi‑dimensional: `ArrayObj` stores `dims: Vec<usize>`; arrays are created with `ArrMake` for however many dimensions you pass (basilcore/bytecode/src/lib.rs lines 51–56; compiler emits `Op::ArrMake` with `dims.len()` count — basilcore/compiler/src/lib.rs e.g. lines ~360–376 and ~1057–1077).
- Call vs array indexing is disambiguated by the compiler. If a name with parentheses is not a known routine, it’s treated as array access with 1–4 indices (compiler: handling of `Expr::Call` around lines ~1985–2005).
- Whole‑array assignment is supported (e.g., `LET a$() = expr`), and for 2‑D string tables there’s an auto‑conversion builtin under the hood (builtin id 138) used by the compiler (see around lines ~1030–1041 and ~344–349). This is relevant if you return the special `StrArray2D` value from a builtin and want it as a real string array variable.

### Practical usage examples
- Returning a 1‑D integer array:
```basil
FUNC MakeRow%(n%)
    DIM a%(n%)
    FOR i% = 0 TO n% - 1
        LET a%(i%) = i% * 10
    NEXT
    RETURN a%
END

LET row = MakeRow%(5)   ' row now holds an array reference
PRINTLN row(2)          ' -> 20
```
Notes:
- The array’s element type is chosen by the `DIM`’d name (`a%` → integer elements). The receiving variable (`row`) can be un‑suffixed; the array carries its element type in the array object itself.

- Returning a 2‑D numeric array:
```basil
FUNC MakeGrid%(rows%, cols%)
    DIM g%(rows%, cols%)
    ' ... fill as needed ...
    RETURN g%
END

LET grid = MakeGrid%(3, 4)
PRINTLN grid(1, 2)
```

- Returning a 2‑D string table from a builtin and materializing it into an array variable:
```basil
FUNC Query()
    ' Many builtins (e.g., SQLITE_QUERY2D$) yield a special StrArray2D value
    RETURN SQLITE_QUERY2D$("SELECT a, b FROM t;")
END

DIM cells$(0, 0)
LET cells$() = Query()   ' whole-array assignment; auto redims and converts
PRINTLN cells$(0, 1)
```

### SUB vs FUNC
- `SUB` has no value and is disallowed in value contexts; `FUNC` produces a value. The compiler enforces this at call sites.

### About function name suffixes
- For functions, the `$`/`%` suffix on the function name is not required or enforced for return type. It’s safe to write `FUNC MyFunc()` and return numbers, strings, arrays, objects, lists, or dicts.
- Suffixes continue to matter for variables (e.g., `x%` coerces to integer on assignment; arrays `DIM a%()` create integer-element arrays).
- Therefore, you do not need an array notation on the function name (`MyFunc[]`/`MyFunc[][]`); that would be inconsistent with the current dynamic return behavior.

### Caveats and tips
- Arrays are reference types. `LET dst = src` copies the reference; both names point to the same array. To copy contents or to re-dimension, use whole-array assignment: `LET dst() = src`.
- Multi‑dimensional arrays are supported for 1–4 dimensions.
- If you want integer elements in an array, `DIM a%(...)`. For strings, `DIM s$(...)`. For numeric floats, omit the suffix: `DIM n(...)`.

### Optional improvements (nice-to-haves, not required)
- Documentation: Clarify in `docs/guides/FUNCTIONS.md` that function returns are dynamic and demonstrate returning arrays and whole‑array assignment.
- Array literals for convenience (e.g., `[...]`): There’s already a design note in `docs/development/ARRAY_NOTATION_BRACKETS.md`. Implementing this would make it easier to construct and return small arrays without `DIM`+assign loops.
- Linting (optional): If desired, add a compile-time warning when a function name uses a type suffix but returns a mismatched type — purely advisory.

### Conclusion
You can proceed today: define `FUNC` without a type suffix and `RETURN` any value, including single- or multi‑dimensional arrays. No language changes are required for the behavior you want; just use `DIM` inside the function to build the array you plan to return, and on the caller side, use normal assignment for by‑reference behavior or whole‑array assignment (`name() = expr`) to copy/convert and auto‑redim.