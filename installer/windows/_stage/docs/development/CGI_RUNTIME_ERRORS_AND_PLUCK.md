### Goal
This report explains how to implement four enhancements in Basil:
1) Accurate runtime error locations with both filename and per-file line numbers, even with `#include`s.
2) Richer CGI error output that appends the real error to the current message when headers are missing.
3) An ORM query builder method `.Unique()`.
4) An ORM query builder method `.Pluck()` for extracting one or two columns as arrays or dictionaries.

---

### 1) Runtime errors must show the correct file and per-file line (not the preprocessed/global line)

#### Current behavior (what the code does now)
- The parser emits `Stmt::Line(line)` for each source statement (from the already-preprocessed text); the compiler then emits `Op::SetLine u16` for those `Stmt::Line` markers. See:
  - `basilcore/parser/src/lib.rs` around the push of `Stmt::Line(line)` before every statement
  - `basilcore/compiler/src/lib.rs` (e.g., at `Stmt::Line(line) => { chunk.push_op(Op::SetLine); chunk.push_u16(line) }`)
- The VM keeps a single `current_line` updated at every `Op::SetLine` (see `basilcore/vm/src/lib.rs`, `current_line` field and `Op::SetLine` handler).
- CLI/REPL/Embedder print runtime errors using only this one number (examples):
  - `basilc/src/main.rs` (e.g., lines around 229–232, 546–549, 1014–1016): `runtime error at line {}: {}`
  - `basilc/src/repl.rs` (lines 114–116) formats similarly.
- The preprocessor already knows which file is being expanded at each moment (it threads a `current_file: &str` through `expand`), but its `SourceMap` is currently an empty stub:
  - `crates/basil-preprocessor/src/lib.rs`: `#[derive(Debug, Clone, Default)] pub struct SourceMap;`

Result: when `#include` expands files inline, line numbers accumulate across the entire expanded buffer. So a fault at `main.basil:100` after including a 100‑line `include.basil` is displayed as 200.

#### Target behavior
- When an error occurs, display: `runtime error at line 100 in main.basil: <message>` (format can be tweaked, but it must include the per-file line and the filename).

#### Recommended design (incremental, minimal invasive)
There are two viable designs; I recommend Phase A first because it is minimal and works without bytecode changes. Phase B is a robust follow-up that benefits all runners uniformly.

- Phase A (no bytecode change; CLI/REPL/CGI only):
  1) Build a real preprocessor source map.
     - Extend `SourceMap` to record, for every output line (1‑based), the origin `(file_path, line_in_that_file)`.
       - Data structure example:
         - Intern strings to avoid duplication: `files: Vec<String>` and `lines: Vec<(u16 /*file_idx*/, u32 /*line*/)>` where `lines.len() == out_line_count + 1` and `lines[pre_line]` yields origin.
       - In `expand(...)`, as you iterate `for (lineno0, raw_line) in text.lines().enumerate()`: when you push each line into `out`, also push a mapping entry for that new `out` line to `(current_file, lineno)`. The `current_file` already varies correctly across nested includes.
     - Return this map in `PreprocessResult { source_map, .. }` (currently it always returns a default):
       - Update the `PreprocessResult` initializer to use the collected map.
  2) Persist the map through the front end and use it at error reporting sites:
     - In `basilc/src/main.rs` (both normal `run` and `--debug`, and in CGI dispatcher), after preprocessing and before/after compile+run, keep the `preprocessed.source_map` around.
     - When an error occurs (`if let Err(e) = vm.run()`), fetch `let line = vm.current_line();` and if `line > 0`, look up `(file,line)` via the map and print: `runtime error at line {origin_line} in {basename(file)}: {e}`. If mapping is missing for some reason, fall back to current behavior.
     - Do similarly in `basilc/src/repl.rs` and `crates/basil-embed/src/lib.rs` (the embedder currently parses and compiles raw files without preprocessing; for include-aware line mapping you should also run the preprocessor there when executing files; see below).
  3) Ensure CGI path also uses the map (see section 2); the same mapping lookup applies.
  4) Note on the embedder: `crates/basil-embed/src/lib.rs::run_file_simple` currently does `parse`/`compile` directly. For correct file+line across includes, switch to: preprocess -> parse(preprocessed.text) -> compile, keep `source_map` in memory for error mapping when you report errors via `RunnerEvent::Error`.

- Phase B (embed the map in bytecode Program for universal availability):
  1) Extend `basilcore/bytecode::Program` to carry an optional compact source map built by the preprocessor caller (CLI/REPL/Embedder):
     - Add to `Program`:
       - `pub source_map: Option<SourceMapMini>` where `SourceMapMini` stores `Vec<String>` (files) + `Vec<(u16,u32)>` (file index + line per preprocessed line).
     - Update `serialize_program`/`deserialize_program` to include this option; bump the format/ABI accordingly and handle backward compatibility in `.basilx` caching (`basilc/src/main.rs` already stores a header with a format/abi version).
  2) Plumb the `source_map` from the preprocessor call site into the `Program` right after `compile(&ast)` returns (before run or serialize). This lets every runner (CLI, REPL, embedder, host integrations) resolve `vm.current_line()` to `(file, line)` from the active program without passing side data around.
  3) Error printing sites then consult `program.source_map` if present to format `line {n} in {file}` consistently everywhere.

Either Phase A or Phase B yields the requested user-facing result. Phase B is cleaner for long-term maintenance but needs a small `.basilx` format bump.

#### Edge cases and notes
- For embedded includes (those returned by the embedded include provider), display a friendly logical path like `<embedded:path>`; `crates/basil-preprocessor` already passes a readable `current_file` string for embedded.
- If an error occurs before the first `Op::SetLine` (line=0), keep the current fallback: `runtime error: ...` or, if you have additional context, you can mention the script path.
- Tests:
  - Program with `#include` that fails in included file: ensure message shows `Line X in include.basil` (not cumulative preprocessed line).
  - Program that fails after included content in the root file: ensure message shows root file and the correct per-root-file line.
  - Programs run via CGI and via the embedder should also show mapped locations.

---

### 2) CGI mode: include the real error text in the “No CGI header sent…” response

#### Current behavior
- In `basilc/src/main.rs`, CGI dispatcher spawns a child Basil process. It wires:
  - Child `stderr` → parent `stderr` (Apache error log) via `eprintln!` (lines 718–721).
  - Child `stdout` is inspected for headers if `#CGI_NO_HEADER` is set (lines 729–741). If headers missing, it prints:
    - `No CGI header sent. Add headers or remove #CGI_NO_HEADER.`
  - The real error text is only in Apache’s error log, not in the HTTP body.

#### Target behavior
- Append the first line (or a concise excerpt) of the child’s real error output to that message, e.g.:
  - `No CGI header sent. Add headers or remove #CGI_NO_HEADER. - parse error at line 239: expected Semicolon or Colon`
  - In Apache logs it might appear prefixed (e.g., `AH01215:`); you can include it as-is or trim the prefix if you prefer.

#### Minimal change
- In the `dirs.cgi_no_header` branch when no header block is found:
  - We already have `output.stderr` as a `Vec<u8>` (captured earlier). Convert to string and get the first non-empty line.
  - Append it to the existing message.

Pseudo-diff (illustrative only):
```
// after let stdout = output.stdout;
let stderr_text = String::from_utf8_lossy(&output.stderr);
let first_line = stderr_text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
...
if dirs.cgi_no_header {
  let has_blank = stdout.windows(4).any(|w| w == b"\r\n\r\n");
  if has_blank { io::stdout().write_all(&stdout).ok(); }
  else {
    println!("Status: 500 Internal Server Error");
    println!("Content-Type: text/plain; charset=utf-8");
    println!();
    if first_line.is_empty() {
      println!("No CGI header sent. Add headers or remove #CGI_NO_HEADER.");
    } else {
      println!("No CGI header sent. Add headers or remove #CGI_NO_HEADER. - {}", first_line.trim());
    }
  }
  return;
}
```
- Keep forwarding full `stderr` to Apache via `eprintln!` (current behavior) for diagnostics.
- Optional: truncate `first_line` to, say, 512 chars to avoid flooding the response.

Tests:
- Script that produces `stdout` with no headers and a parse error → verify body includes both the standard message and the first line of the error.
- Script that emits valid headers → pass through unmodified.

---

### 3) ORM: add `.Unique()` to return unique results

#### Current ORM surface
- Query builder is implemented in `basil-objects-orm/src/lib.rs` via `QueryObj` with methods like `Where$`, `OrderBy$`, `Limit%`, `Offset%`, `With$`, `Select$`, `Get`, `Find%`, `First`, `ToJson$`.
- `Get()` compiles SQL in `compile_select()` and materializes rows as `ARRAY<ORM_ROW>`.

#### Design
- Add `distinct: bool` to `QueryObj` and expose a chainable, zero-arity method `Unique()` that sets `distinct = true` and returns a cloned `ORM_QUERY` (same pattern as existing chainers).
- In `compile_select()`, if `distinct`, output `SELECT DISTINCT ...` instead of `SELECT ...`.
- This makes `.Unique()` effective for both `.Get()` and `.Pluck()` (see next section).

#### API and constraints
- Support `.Unique()` only as a query modifier (i.e., before materialization). So use:
  - `orm@.Table("keywords").Select$(cols).Unique().Get()`
  - Or with `Pluck`, `orm@.Table("keywords").Select$(cols).Unique().Pluck("initial")`
- Do not support `.Get().Unique()` (that would require deduplication after materialization over `ARRAY<ORM_ROW>`; it’s non-trivial and inconsistent with the builder design). If you need post-fetch dedupe in the future, consider a separate helper operating on arrays.

#### Implementation notes
- Update `QueryObj::descriptor_static()` to register `MethodDesc { name: "Unique".into(), arity: 0, return_type: "ORM_QUERY" }`.
- Add a `match` arm in `BasicObject for QueryObj::call`:
  - `"UNIQUE" => { self.distinct = true; Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }`
- Update `compile_select()` to prefix `SELECT DISTINCT` when `distinct` is true.

---

### 4) ORM: add `.Pluck()` to extract one or two columns

#### Desired UX (from your examples)
- One column → return an array of that column’s values.
  - `productIds$ = orm@.Table("products").Get().Pluck('id')` → string array of IDs
  - `productIds% = orm@.Table("products").Get().Pluck('id')` → integer array of IDs, or throw if any value can’t be converted to integer
- Two columns → return a dictionary mapping key → value.
  - `LET userEmailsById@ = orm@.Table("users").OrderBy$("id%").Get().Unique().Pluck('email', 'id')` → dict of `{ id -> email }`

#### Where to implement and semantics
To keep the API coherent with the existing builder, implement `Pluck` on `ORM_QUERY` (not on `ARRAY<ORM_ROW>`). This makes `Pluck` a terminal that “implies Get()” — it compiles and executes a `SELECT` and returns the requested projection as an array or dictionary.

- Supported forms:
  - `Pluck(name$)` → returns `ARRAY` of scalars (strings by default; can be numeric when type is known)
  - `Pluck(value_name$, key_name$)` → returns `DICT` mapping key (as string) to value (typed as below)
- Works with `.Unique()` by leveraging SQL `DISTINCT` if `.Unique()` is chained before `.Pluck()`.
- Accept column names with or without Basil suffixes; use model metadata (`ModelMeta.cols`) to infer suffix when omitted.
- If the column is an expression alias (e.g., `Select$("UPPER(LEFT(keyword,1)) AS initial")` then `Pluck("initial")`), default to returning strings.

#### Type inference and conversions
- For element type in the 1‑arg case:
  - If the resolved column name in the model has suffix: `%` → return an `Int` array; `$` → `Str` array; no suffix → `Num` array.
  - If type is unknown (alias/expr), default to strings.
- Conversions and errors:
  - Creating an `Int` array: parse the value (which originates as JSON text or number) to `i64`. If any row fails, raise `BasilError("Pluck: value at row N is not an integer: ...")`.
  - Creating a `Num` array: parse to `f64`; error on invalid.
  - Creating a `Str` array: no conversion needed.
- For the 2‑arg dict case:
  - Keys are strings (use `to_string()`), values typed as above.
  - If a duplicate key occurs, last write wins (or you can choose to error; however Eloquent overwrites silently; document whichever you choose). I recommend overwriting silently for now for simplicity.

#### SQL generation
- For 1‑arg `Pluck(col)`: generate `SELECT [DISTINCT] col FROM table ...` honoring WHERE/ORDER/LIMIT/OFFSET. For Postgres, use `$1..$n` placeholders; for MySQL, `?` — reuse `compile_select` logic but supply a narrowed set of columns for this specialized fetch.
- For 2‑arg `Pluck(val_col, key_col)`: generate `SELECT [DISTINCT] key_col, val_col FROM table ...` and build the dict from the two fields.

#### Return values
- 1‑arg: return a `Value::Array` with appropriate `ElemType` (`Str`, `Int`, or `Num`).
- 2‑arg: return a `Value::Dict` mapping `String -> Value` where `Value` uses the inferred scalar type.

#### API surface change
- Add to `QueryObj::descriptor_static()` a new method descriptor, e.g.:
  - `MethodDesc { name: "Pluck".into(), arity: 1 /* or 2 */, arg_names: vec!["col_or_value$".into(), "key$".into()], return_type: "ARRAY|DICT".into() }`
  - The `return_type` string is documentation only in Phase 1.

#### Examples (final, recommended usage)
- Unique initials (strings):
  - `orm@.Table("keywords").Select$( ["UPPER(LEFT(keyword,1)) AS initial"] ).Unique().Pluck("initial")`
- Emails by user id:
  - `LET userEmailsById@ = orm@.Table("users").OrderBy$("id%","ASC").Unique().Pluck("email$", "id%")`
- Product ids as integers (and error on non-integer):
  - `DIM productIds%[] = orm@.Table("products").Pluck("id%")`

Note: In the above, `Pluck()` does not require `Get()`; it implies fetching. Using `Get().Pluck(...)` is not supported in this Phase 1 API because `Get()` returns an `ARRAY<ORM_ROW>` and arrays don’t expose `Pluck`. If you strongly prefer `Get().Pluck(...)`, we can add a global helper in a future phase that operates on arrays of `ORM_ROW`.

#### Implementation hotspots
- `basil-objects-orm/src/lib.rs`:
  - Extend `QueryObj` with `distinct: bool`.
  - Update `descriptor_static()` to register `Unique` and `Pluck`.
  - Add `call` arms for `UNIQUE` and `PLUCK`.
  - Implement helpers to:
    - Resolve column name to model meta (to get suffix/type) when possible.
    - Build `Value::Array` of proper `ElemType` or `Value::Dict`.
    - Convert/validate numbers.

#### Tests
- `.Unique()` produces `SELECT DISTINCT` and deduplicates results at the DB level; verify via query capturing or on a small known table.
- `.Pluck("id%")` → `Int` array; add one non-integer row and assert it raises a runtime error.
- `.Pluck("email$", "id%")` returns a dictionary with keys set to the string form of user IDs and values equal to emails.
- `.Pluck("initial")` after an expression alias works and returns strings.

---

### Documentation updates
- `docs/guides/ORM.md`:
  - Add `Unique()` and `Pluck()` to the “Supported methods” list.
  - Document that `Unique()` is a chainable query modifier that must be used before a terminal (`Get`, `First`, `Pluck`, `ToJson$`).
  - Document `Pluck()` as a terminal: one-arg returns an array; two-arg returns a dictionary; it implies fetching (no prior `Get()` needed).
  - Add examples mirroring the ones above.
- `docs/guides/EXCEPTIONS.md` and CLI help snippets:
  - Update the runtime error format examples to show `line N in file.basil`.

---

### Rollout plan (safe and incremental)
1) Implement Phase A source mapping in the preprocessor and wire it up in CLI/CGI/REPL. Verify with examples and tests.
2) Implement the CGI error message enhancement (tiny change, independent of step 1).
3) Add `.Unique()` and `.Pluck()` to the ORM module alongside tests/examples.
4) Optional: Implement Phase B (embed mapping into bytecode `Program`) and bump `.basilx` format for a cleaner long-term solution.

---

### Summary of what will change
- Runtime error messages: will read like `runtime error at line 100 in main.basil: <message>` and correctly identify the file in which the error occurred, regardless of `#include`s.
- CGI failures without headers: the response body will also include the first line of the real error, appended to the current message.
- ORM query builder: supports `.Unique()` (SQL `DISTINCT`).
- ORM query builder: supports `.Pluck()` as a terminal; one column → array, two columns → dictionary; types inferred by suffix when available; raises on invalid numeric conversions.

If you’d like, I can turn this into patches in a follow-up step, starting with the preprocessor `SourceMap` and CLI/CGI wiring, then the ORM changes.