### Overview
This report specifies how to add the following functions to the BASIC interpreter (Basil) in a way that is consistent with the current architecture and language surface:

- `REMOVE$(hay$, needle$)`
- `REPLACE$(needle$, new$, hay$)`
- `INSERT$(hay$, ins$, pos%)`
- `DATE$()`, `TIME$()`, `NOW$()`
- `EXPLODE(...)` for splitting into List/Array/2D array/Dict
- `IMPLODE$(var, delim1$ [,delim2$])`

It includes precise semantics, integration points (compiler and VM), edge cases, Unicode/indexing rules, algorithm notes, tests, and documentation updates. Where Basil has existing conventions (e.g., built-in dispatch, 1-based character indexing for string slices), these functions follow them.


### How built-ins integrate today (baseline)
- The compiler recognizes built-in names and emits `Op::Builtin` with a small numeric id: see `basilcore/compiler/src/lib.rs` (mapping around other built-ins like `MID$`, `LEFT$`, `RIGHT$`).
- The VM executes built-ins by switching on that id: see `basilcore/vm/src/lib.rs` (handling `argc`, type checks, and pushing a `Value`).
- Strings are full UTF‑8; existing string ops (e.g., `MID$`) are implemented on Unicode scalar values (`.chars()`), with 1‑based positions for string slicing and 0‑based return for `INSTR`.
- Dynamic containers exist:
  - `Value::List(Rc<RefCell<Vec<Value>>>)`
  - `Value::Dict(Rc<RefCell<HashMap<String, Value>>>)`
  - `Value::Array(Rc<ArrayObj>)` with 1–4 dimensions and element type, plus a special `Value::StrArray2D` for whole‑array assignment conversions.

We’ll add new ids and implementations consistent with that pattern.


### Function specifications (user-facing behavior)
#### 1) REMOVE$(hay$, needle$) → String
- Returns `hay$` with all occurrences of `needle$` removed. Case sensitive. No regex; plain substring.
- If `needle$` is empty, returns `hay$` unchanged.
- Unicode-safe: operations treat `needle$` as a byte substring match, which is okay for UTF‑8 as long as we replace only at valid byte boundaries found by `find` (standard library ensures that). No re-indexing by character is needed because we don’t expose indices here.
- Examples:
  - `REMOVE$("Hello World", "World")` → `"Hello "`
  - `REMOVE$("aaaa", "aa")` → `""` (remove all two-char occurrences, repeatedly; see algorithm note to ensure global replacement)

Edge cases and errors:
- Arg count must be 2; arg types must be strings, else `BasilError("REMOVE$ arg N must be string")`.


#### 2) REPLACE$(needle$, new$, hay$) → String
- Returns `hay$` with all occurrences of `needle$` replaced by `new$`.
- Arg order matches user example: `REPLACE$(needle, new, hay)`.
- Case sensitive; plain substring.
- If `needle$` is empty, returns `hay$` unchanged (avoids infinite insertion between chars).
- Examples:
  - `REPLACE$("Hello", "Hi", "Hello World")` → `"Hi World"`
  - `REPLACE$("aa", "b", "aaaa")` → `"bb"`

Errors and types similar to `REMOVE$`.


#### 3) INSERT$(hay$, ins$, pos%) → String
- Inserts `ins$` into `hay$` at character position `pos%` and returns the result.
- Indexing: 1‑based by character (Unicode scalar), consistent with `MID$`.
- Boundary rules:
  - If `pos% <= 0`: prepend.
  - If `pos% > LEN(hay$)`: append.
  - Otherwise, split at the `pos%`-th character and insert between left/right parts.
- Examples:
  - `INSERT$("Hello World", "Cruel ", 6)` → `"Hello Cruel World"`
  - `INSERT$("abc", "-", 0)` → `"-abc"`
  - `INSERT$("abc", "-", 10)` → `"abc-"`

Errors and types:
- Arg count = 3.
- `hay$`, `ins$` must be strings; `pos` numeric; non-integers are truncated like `MID$` does.


#### 4) DATE$(), TIME$(), NOW$() → String
- `DATE$()` returns local date in `YYYY-MM-DD`.
- `TIME$()` returns local time in `HH:MM:SS` (24-hour).
- `NOW$()` returns `DATE$() + " " + TIME$()` (e.g., `"2025-11-29 21:37:00"`).
- Timezone: Local system time zone.
- Determinism in tests: VM will support a test clock override (see implementation plan), else use system clock.

Errors:
- No arguments allowed; otherwise `BasilError("DATE$ expects 0 arguments")`, similarly for `TIME$`, `NOW$`.


#### 5) EXPLODE(...)
Goal: Split a string into structured data. We support three primary forms:

A. Two-argument form → List of strings
- `EXPLODE(src$, delim1$)` → returns a `List` containing the token strings (untrimmed), preserving empty tokens (e.g., `",,"` → `{"", "", ""}`).
- Example: `EXPLODE("This,That,Other", ",")` → `["This","That","Other"]`.
- Intended use: `LET items@ = EXPLODE("a,b,c", ",")` then `items@[i%]` with `1..LEN(items@)`.

B. Three-argument form → Dictionary (map) of key→value
- `EXPLODE(src$, pairDelim$, kvDelim$)` → returns a `Dict`:
  - Pairs are split by `pairDelim$`; within each pair, the first `kvDelim$` splits key and value.
  - Missing `kvDelim$` in a pair ⇒ value is `""`.
  - Duplicate keys: last write wins.
  - Key is taken as-is; value decoding:
    - Keep as string by default (consistent and predictable). Callers may opt-in to parsing numbers/booleans if needed later.
  - Example: `EXPLODE("name=Fido&species=dog&age=7", "&", "=")` → `{ "name":"Fido","species":"dog","age":"7" }`.
- Tip (web): If parsing URL-encoded query strings, decode first: `EXPLODE(URLDECODE$(Request$), "&", "=")`.

C. Array-returning variants (optional aliases) → for array workflows
- Basil already supports alias names ending with `[]` to indicate an array-returning builtin (e.g., `ZIP_ARRAY$[]`). We mirror that for predictability:
  - `EXPLODE$[](src$, delim1$)` → 1‑D string array (rank 1) of tokens.
  - `EXPLODE2D$[](src$, pairDelim$, kvDelim$)` → 2‑D string table with rows = number of pairs, cols = 2, row-major `data = [[key,value], ...]`.
- These make it easy to use whole‑array assignment:
  - `DIM items$(0)` then `LET items$() = EXPLODE$[]("a,b,c", ",")`.
  - `DIM param$(0,0)` then `LET param$() = EXPLODE2D$[](Request$, "&", "=")`.
- If you prefer not to add these aliases now, you can still return arrays directly as `Value::Array` and assign them to a scalar variable; use `LET a$() = arr` for whole‑array materialization as needed.

General rules (all forms):
- Do not trim whitespace automatically; leave tokens as-is for caller control (`TRIM$` exists if needed).
- Empty `delim` is an error: `BasilError("EXPLODE: delimiter must not be empty")`.
- Arg types: all delimiters and `src` must be strings.


#### 6) IMPLODE$(var, delim1$ [,delim2$]) → String
- Converts structured data to a string.
- Behavior depends on `var`’s runtime type:
  - List → join elements using `delim1$`.
  - 1‑D Array → join elements using `delim1$`.
  - 2‑D Array (string) with exactly 2 columns → produce key/value pairs joined with `delim2$` and pairs separated by `delim1$`.
  - Dict → produce key/value pairs joined with `delim2$` and pairs separated by `delim1$`.
- Elements/values are converted to strings using existing `Value`→string convention (same as `PRINT`), with no quoting/escaping added by default.
- Ordering:
  - For Dict: iteration order is unspecified (hash map). For reproducible results, optionally sort keys first. Implementation will default to insertion/HashMap order; document that it’s unspecified.
- Errors:
  - List/1‑D Array: require exactly 2 args (var, delim1$).
  - Dict/2‑D Array: require 3 args (`delim2$` is required for key/value join); error if missing.
  - For 2‑D Array: error if columns ≠ 2.

Examples:
- `IMPLODE$(["a","b","c"], ",")` → `"a,b,c"` (List).
- `IMPLODE$(dict@, "&", "=")` → e.g., `"name=Fido&species=dog&age=7"`.
- With 2‑D array: if `param$()` is 3x2 of keys/values, `IMPLODE$(param$(), "&", "=")` → line.


### Implementation plan (compiler and VM)
#### 1) Choose builtin ids and names
Add these to the compiler’s builtin map (in `basilcore/compiler/src/lib.rs`, near other built-ins):

- Strings:
  - `"REMOVE$"  => Some(141u8)`
  - `"REPLACE$" => Some(142u8)`
  - `"INSERT$"  => Some(143u8)`
- Date/Time:
  - `"DATE$" => Some(144u8)`
  - `"TIME$" => Some(145u8)`
  - `"NOW$"  => Some(146u8)`
- Split/Join:
  - `"EXPLODE" => Some(147u8)`           // List or Dict, depending on arity
  - `"IMPLODE$" => Some(148u8)`          // Requires type-driven arg checks
  - Optional array-returning aliases:
    - `"EXPLODE$[]"  => Some(149u8)`     // 1‑D string array from (src, delim)
    - `"EXPLODE2D$[]"=> Some(150u8)`     // 2‑D string array from (src, pairDelim, kvDelim)

These id numbers are unused in core today and leave room for future adjacent functions. If you prefer a different block, adjust consistently in both compiler and VM.

Compiler emission:
- For each matched name, compile arguments left→right, then emit `Op::Builtin, <id>, <argc>` (same as other built-ins).

#### 2) VM dispatch (basilcore/vm/src/lib.rs)
Add `match` arms under the `Builtins` handler:

- 141 REMOVE$ (argc=2)
  - Args: `(hay: Str, needle: Str)`
  - If `needle.is_empty()`: push `hay`.
  - Else perform global replace with empty string.
  - Implementation: a simple loop with `find` building `String` or use `replace` (it already handles global replacement).

- 142 REPLACE$ (argc=3)
  - Args: `(needle: Str, new: Str, hay: Str)` note the order.
  - If `needle.is_empty()`: push `hay`.
  - Else `hay.replace(&needle, &new)`.

- 143 INSERT$ (argc=3)
  - Args: `(hay: Str, ins: Str, pos: Int/Num)` → truncate numeric to `i64`.
  - Compute `nchars = hay.chars().count()`.
  - Normalize `pos`:
    - `pos <= 0` → index0 = 0.
    - `pos >= nchars + 1` → index0 = `nchars` (append).
    - else `index0 = pos - 1`.
  - Split at character index `index0`:
    - Use `char_indices()` to compute the byte index; slice safely.
  - Concatenate `left + ins + right`.

- 144 DATE$ (argc=0)
- 145 TIME$ (argc=0)
- 146 NOW$ (argc=0)
  - Use `chrono`’s `Local::now()`.
  - `DATE$`: `format("%Y-%m-%d")` → push `Value::Str`.
  - `TIME$`: `format("%H:%M:%S")`.
  - `NOW$`: `format("%Y-%m-%d %H:%M:%S")`.
  - Testing support (optional but recommended): if env var `BASIL_TEST_TIME` is set to an RFC3339 timestamp, parse and use that instead of system time. This keeps VM single-sourced (no new trait), and makes unit tests deterministic. Example value: `2025-11-29T21:37:05-05:00`.

- 147 EXPLODE
  - If `argc == 2`: return `List` of strings.
    - Validate `delim1` non-empty; if empty → error.
    - Split: preserve empty tokens, do not trim.
    - Build `Vec<Value::Str>` inside a `List(Rc<RefCell<Vec<Value>>>)` for mutability.
  - Else if `argc == 3`: return `Dict` of string→string.
    - Validate delimiters non-empty.
    - For each pair: split once on first occurrence of `kvDelim` (so values can contain it).
    - Key may be empty string; value default to empty if separator absent.
    - Insert into `HashMap<String, Value>` as strings.
  - Else: error `"EXPLODE expects 2 or 3 arguments"`.

- 148 IMPLODE$
  - `argc` must be `2` or `3`.
  - Inspect `var`’s type:
    - List: require 2 args; join by `delim1` using each element’s string form.
    - Array:
      - If rank==1: require 2 args; join elements’ string forms.
      - If rank==2 and `cols==2`: require 3 args; iterate rows, form `key + delim2 + val`, join rows by `delim1`.
      - Otherwise: `BasilError("IMPLODE$: array must be 1-D or 2-D (2 columns)")`.
    - Dict: require 3 args; iterate entries and produce `key + delim2 + value` joined by `delim1`.
    - Other types: `BasilError("IMPLODE$: unsupported type ...")`.

- 149 EXPLODE$[] (optional)
  - Argc must be 2; create and return a 1‑D string `Array` (`Value::Array`) whose `elem` is `ElemType::Str` and `dims=[tokens.len()]`, using helper similar to current `make_string_array`.

- 150 EXPLODE2D$[] (optional)
  - Argc must be 3; return `Value::StrArray2D { rows, cols:2, data }` to interoperate with whole‑array assignment easily, or return a true 2‑D string `Array`. Using `StrArray2D` is consistent with existing whole‑array assignment optimizations.

Notes:
- Error messages should follow existing style, e.g., `"FUNCNAME expects N arguments"` and `"FUNCNAME arg i must be string"`.


### Algorithms and complexity
- `REMOVE$`/`REPLACE$`: linear in `|hay|` (Rust’s `replace` is efficient and Unicode-safe). For repeated insertions in `REMOVE$`, prefer `replace` with empty string.
- `INSERT$`: O(|hay|) to locate split and build new string.
- `EXPLODE`:
  - List form: linear scan; preserve empties.
  - Dict form: linear; single split per pair; overall O(n).
- `IMPLODE$`: O(n) over elements/pairs with string conversions.


### Unicode and indexing decisions
- All indices exposed to users are character-based and 1‑based for string position (`INSERT$`), matching `MID$`.
- `INSTR` currently returns 0‑based index or 0 when not found; these new functions do not return indices, so they won’t add inconsistency.
- Splitting and replacing use Rust’s byte indices but only at delimiter/substring boundaries returned by standard library; this is safe for UTF‑8 and produces expected results for users.


### Tests (unit and integration)
Create tests under `testprogs` or Rust unit tests in VM module, covering:

- REMOVE$:
  - Basic removal, multiple occurrences, overlapping patterns (`"aaaa"` vs `"aa"` → `""`).
  - Empty needle → unchanged.
  - Unicode hay/needle with multi-byte chars.

- REPLACE$:
  - Global replacement, overlapping, empty needle.

- INSERT$:
  - Prepend (`pos<=0`), append (`pos>LEN`), middle insert.
  - Unicode string where split is inside multi-byte glyph; verify integrity.

- DATE$/TIME$/NOW$:
  - Smoke test that strings parse and have expected lengths/shapes.

- EXPLODE (2 args → List):
  - Preserve empty tokens: `",,"` → 3 empties.
  - Single element (no delimiter) → one token.
  - Unicode delimiter and tokens.

- EXPLODE (3 args → Dict):
  - Missing kv delimiter in a pair → value empty.
  - Duplicate keys → last wins.
  - URL-like input with `URLDECODE$` in front to validate doc guidance.

- EXPLODE$[] / EXPLODE2D$[] (if implemented):
  - Round-trip via whole‑array assignment: `LET arr$() = EXPLODE$[](...); PRINTLEN/LEN checks`.

- IMPLODE$:
  - List and 1‑D array joining.
  - Dict/2‑D array with both delimiters.
  - Error when missing `delim2$` for key/value cases.

Property tests (optional but useful):
- For arbitrary strings `s` and delimiter `d` (non-empty): `IMPLODE$(EXPLODE(s,d), d)` should equal `s` for the List form, provided `s` contains no `d` at ends when checking empties; or test that `EXPLODE(IMPLODE(list,d), d)` round-trips the list content.


### Documentation updates
- Add entries to `docs/guides/BASIL_REFERENCE.md` and `docs/guides/BASIL_KEYWORDS_BY_CATEGORY.md` under String and Collections.
- Document 1‑based indexing for `INSERT$`, behavior for empty delimiters, and the forms of `EXPLODE` and `IMPLODE$` with examples:

Examples to include:
```
REM REMOVE$
LET a$ = "Hello World"
LET a$ = REMOVE$(a$, "World")
PRINT a$           REM -> Hello 

REM REPLACE$
LET a$ = "Hello World"
LET a$ = REPLACE$("Hello", "Hi", a$)
PRINT a$           REM -> Hi World

REM INSERT$
PRINT INSERT$("Hello World", "Cruel ", 6)  REM -> Hello Cruel World

REM DATE$/TIME$/NOW$
PRINT DATE$(), TIME$(), NOW$()

REM EXPLODE to list
LET items@ = EXPLODE("This,That,Other", ",")
FOR i% = 1 TO LEN(items@) { PRINT items@[i%] }
NEXT

REM EXPLODE to dict
LET params@ = EXPLODE(URLDECODE$(REQUEST$()), "&", "=")
PRINT params@["name"]

REM (Note: array-returning alias forms like EXPLODE$[]/EXPLODE2D$[] are not added.)

REM IMPLODE$
PRINT IMPLODE$(items@, ",")                  REM list -> string
PRINT IMPLODE$(params@, "&", "=")           REM dict -> query string
```

- Add keyword entries to `KEYWORDS.md` under “Core Built-in Functions”.


### Backward compatibility and error behavior
- All functions introduce new names only; no existing semantics change.
- Keep error messages consistent (same phrasing style as `MID$`, `INSTR` etc.).
- Dict `IMPLODE$` order is unspecified; document clearly to avoid surprises.


### Sister project BASIC guidance
If the sister BASIC has a different internal architecture, preserve user-facing behavior:
- String index base for `INSERT$`: 1‑based by character, 0 and >LEN rules identical.
- Global replacement semantics for `REMOVE$`/`REPLACE$`.
- `EXPLODE` defaulting to List for 2-arg form and Dict for 3-arg form is easy to implement in any interpreter without LHS-type inspection.
- Provide optional array-returning aliases if whole-array assignment is supported (e.g., `EXPLODE$[]`, `EXPLODE2D$[]`). If not, return native arrays and rely on ordinary assignment.
- Date/time: use local timezone; format as specified.


### Implementation checklist
1) Compiler
- Add name→id entries listed above.

2) VM
- Add the builtin id cases with argument validation and behavior as specified.
- Add helper(s):
  - `fn split_preserve_empty(src: &str, delim: &str) -> Vec<String>` (if not using `str::split` variants directly).
  - `fn make_string_array(Vec<String>) -> Value` already exists for 1‑D string arrays.

3) Tests
- Unit tests for each function, including Unicode and edge cases.
- Round-trip tests for `EXPLODE`/`IMPLODE$`.

4) Docs
- Update reference, categorized keywords, and add examples.


### Notes on potential future extensions
- `REPLACE$`/`REMOVE$` could accept an optional 4th `count%` argument to limit replacements, or an optional `start%` position.
- `EXPLODE` could optionally parse numbers/booleans for dict values based on a flag (e.g., 4th argument `"typed"`). For now, keep values as strings to stay predictable.
- Deterministic dict ordering in `IMPLODE$` could be enabled behind a feature flag by sorting keys.


### Summary
This plan introduces the requested functions with clear, consistent semantics, minimal changes to the compiler and VM (simple new built-in ids and match arms), careful Unicode and indexing rules, and a robust testing/documentation strategy. Optional array-focused aliases (`EXPLODE$[]`, `EXPLODE2D$[]`) align with an existing pattern and make 1‑D/2‑D array workflows convenient without adding ambiguity to the base `EXPLODE` name. Implementers can use this report directly to add the features to Basil, and the sister project can replicate the same surface behavior even with a different runtime architecture.