### Proposal: Add @FOREACH / @FORELSE / @ENDFOREACH to RENDER$

#### Goal

Add a simple, readable looping construct to Fred so templates can iterate over lists, dictionaries, and arrays and
render a block for each element, with an optional fallback block when there are zero iterations.

---

### Syntax

- Basic form
    - `@FOREACH(var IN expr)` … `@ENDFOREACH`
    - `@FOREACH(var IN expr)` … `@FORELSE` … `@ENDFOREACH`

- Where
    - `var` is an identifier name (e.g., `item`, `key$`, `row%`). It becomes available inside the loop body as a
      variable (see scope rules below).
    - `expr` is a normal Basil expression that evaluates to a collection: `List`, `Dict`, or `Array` (details below).

- Semantics by collection type
    - List (`Value::List`): `var` is each list element (any Basil type).
    - Dict (`Value::Dict`): `var` is each key as a string (order unspecified). See optional variants below for
      key/value.
    - Array (`Value::Array`):
        - 1‑D array: `var` is each element value (any Basil type).
        - 2‑D array with exactly 2 columns: treat as key/value rows (see Optional Variants). For the base form, `var`
          iterates keys (column 0). If you prefer explicit key/value access, use the proposed two‑var variant below.
        - Other shapes (>1 dimension not 2×N): zero iterations (or error, configurable; see Error Handling).

- Zero iterations
    - If the collection has zero items, and an accompanying `@FORELSE` is present, render the `@FORELSE` block.
      Otherwise, render nothing.

---

### Scope rules inside the loop

- The loop variable `var` is injected into the render scope for each iteration so it is usable in both `{{ … }}`
  expressions and nested Fred directives.
- Type‑suffix convenience:
    - As with the context dictionary today, when `var` is a String, both `var` and `var$` resolve; when `var` is an
      Integer, both `var` and `var%` resolve. For other types, only `var` is injected. If the chosen `var` name already
      ends with a suffix (`$` or `%`), it is injected verbatim without auto‑aliasing.
- Shadowing precedence during rendering remains:
    - Loop var > context dict overlay > module globals (for the duration of the iteration body).
- Nested loops: inner loop variables shadow outer ones of the same name. Each iteration restores the prior value after
  the body is evaluated.

---

### Parser changes (basilcore/vm/src/render.rs)

- Add a new AST node variant:
    - `Node::ForEach { var: String, enumerable: String, body: Vec<Node>, else_part: Vec<Node>, pos: usize }`
- Update `parse_at_directive()` to recognize `@FOREACH(...)`, and parse the header:
    - Within parentheses: `<ident> IN <expr>`
        - `ident`: use `read_ident()`
        - `IN`: a case‑insensitive keyword match (consume intervening whitespace)
        - `<expr>`: use the existing `read_args_until_rparen()` to capture the entire expression as a string to be
          evaluated later (like other directives) — no comma splitting occurs in this header form.
- Block parsing:
    - After the header, call `parse_block(..., end_tokens=["FORELSE", "ENDFOREACH"])` to collect the primary body.
    - If the next token is `@FORELSE`, parse until `@ENDFOREACH` for the else body.
    - Finally consume `@ENDFOREACH`.
- Keep `pos` as the byte offset of the starting `@` for accurate line:col in errors (using the existing
  `wrap_tmpl_error`).

---

### Evaluator changes

- In `eval_nodes()`, add a case for `Node::ForEach`:
    1) Evaluate `enumerable` via `eval_basil_expr_with_fred()`.
    2) Determine iteration strategy based on the resulting `Value`:
        - List: iterate `rc.borrow().iter().cloned()`.
        - Dict: iterate keys: `for k in map.keys()`; convert to `Value::Str(k.clone())` for loop var.
        - Array:
            - If `dims.len()==1`: iterate underlying `arr.data.borrow().iter().cloned()`.
            - Else if `dims.len()==2 && dims[1]==2`: treat as key/value rows; base form uses column 0 as the loop
              variable (key). See optional two‑var variant below.
            - Else: zero iterations (or error per strategy below).
    3) For each iteration:
        - Temporarily bind `var` into the overlay (see below) and evaluate `body` by calling `eval_nodes()` recursively.
        - Append the produced string to the output buffer.
    4) If the iteration count is zero and `else_part` is non‑empty, evaluate `else_part` and append its result.

- Overlay binding strategy (reuses current context mechanism):
    - `Ctx.overlay` is already an `Option<Rc<RefCell<HashMap<String, Value>>>>` used to surface variables to expression
      snippets.
    - For the loop body, do:
        - Save any existing value for `var` (exact‑case key) in the overlay map (if present).
        - Write `var -> Value` into the overlay map; do not pre‑create suffixed aliases here — the per‑snippet injection
          already creates `$`/`%` aliases when applicable.
        - Evaluate the body nodes; afterwards restore or remove `var` in the overlay map.
    - This keeps `{{ … }}` expression snippets seeing the loop variable through the same path as the context dictionary.

- Error handling and messages
    - If `@FOREACH` header is malformed (missing `IN`, missing identifier), throw
      `BasilError("RENDER$: malformed @FOREACH header")` with line:col using `wrap_tmpl_error`.
    - If `enumerable` evaluates to a type other than List/Dict/Array, iterate zero times silently (consistent with
      tolerant templating), or optionally error in a “strict” mode later.

---

### Types and iteration details

- List (`Value::List`): no constraints; each element’s `format!` string form is used when injected in `{{ … }}` or
  inline `@…` calls.
- Dict (`Value::Dict`): iteration order is unspecified (Rust `HashMap`). If deterministic output is required, consider
  the `@FOREACH_SORTED` enhancement below.
- Array (`Value::Array`):
    - 1‑D: iterate all cells.
    - 2‑D (2 columns): treat as rows of `{key, value}` to mirror how `IMPLODE$` treats 2‑col arrays in existing code.
      Base form yields the key as the loop variable; two‑var form below enables direct access to both key and value.
    - Other shapes: zero iterations (v1).

---

### Examples

- List
  ```basil
  LET nums@ = [1, 2, 3]
  LET tpl$ = """
  @FOREACH(n% IN nums@)
    {{ n% }}
  @FORELSE
    no numbers
  @ENDFOREACH
  """
  PRINT RENDER$(tpl$)
  ```

- Dictionary (keys only in base form)
  ```basil
  LET pet@ = { "name": "Fido", "species": "dog", "age": 7 }
  PRINT RENDER$("""
  @FOREACH(k$ IN pet@)
    {{ k$ }} = {{ pet@[k$] }}
  @FORELSE
    empty dict
  @ENDFOREACH
  """)
  ```

- Array (1‑D)
  ```basil
  DIM a%(3)
  LET a%(1) = 10; LET a%(2) = 20; LET a%(3) = 30
  PRINT RENDER$("""
  @FOREACH(val% IN a%())
    {{ val% }}
  @ENDFOREACH
  """)
  ```

- Array (2‑D, 2 columns as key/value rows)
  ```basil
  DIM rows$(2,2)
  LET rows$(1,1) = "k1"; LET rows$(1,2) = "v1"
  LET rows$(2,1) = "k2"; LET rows$(2,2) = "v2"
  PRINT RENDER$("""
  @FOREACH(k$ IN rows$())
    {{ k$ }} = {{ rows$()[k$] }}  ' when mirrored into a dict first, or use two‑var variant below
  @ENDFOREACH
  """)
  ```

---

### Implementation plan (concrete steps)

1) AST and parsing (render.rs)
    - Add `Node::ForEach { var, enumerable, body, else_part, pos }`.
    - In `parse_at_directive()` handle `FOREACH`:
        - Expect `(` then read raw header until `)`.
        - Split header into `<ident> IN <expr>` by finding the `IN` keyword at top level (ignore `IN` within
          quotes/parentheses). Reuse the existing utility style used for `CASE`/argument parsing.
        - Then parse the body via `parse_block(..., ["FORELSE","ENDFOREACH"])`; optionally parse an else‑block; consume
          `@ENDFOREACH`.

2) Evaluation
    - In `eval_nodes()`, add the `ForEach` branch implementing the iteration pathways described above.
    - Create a small helper `with_overlay_var(ctx, name, value, || eval_nodes(...))` to push/pop the variable in the
      overlay safely.
    - Implement enumerator helpers:
        - `iter_list(&Rc<RefCell<Vec<Value>>>) -> impl Iterator<Item=Value>`
        - `iter_dict_keys(&Rc<RefCell<HashMap<String,Value>>>) -> impl Iterator<Item=Value>` returning
          `Value::Str(key)`.
        - `iter_array_1d(&ArrayObj) -> impl Iterator<Item=Value>`
        - `iter_array_2d_keys(&ArrayObj) -> impl Iterator<Item=Value>` when `cols==2`.

3) Error reporting
    - Reuse `wrap_tmpl_error()` with `pos` for malformed headers and evaluation errors.

4) Include recursion
    - No extra work: loop variables are stored in the overlay; since `@INCLUDE` already reuses the same `Ctx` (and thus
      overlay), included templates see the current loop variables naturally.

5) Tests/examples
    - Add examples:
        - `examples/foreach_list.basil` (already present): adapt or add a template using `RENDER$` with a list.
        - `examples/foreach_dict.basil` (already present): mirror with `RENDER$` usage.
        - Expand `examples/render.basil` with a short `@FOREACH` over a context dict and list to verify `@FORELSE`.

6) Docs
    - Update `docs/guides/RENDER_AND_FRED.md`:
        - New section under Flow Control for `@FOREACH`.
        - Note dict order is unspecified; suggest sorted variant if needed.
        - Describe loop variable scope and suffix aliasing behavior.

---

### Implemented variants and features

- Single‑variable `@FOREACH(var IN expr)` with optional `@FORELSE` and closing `@ENDFOREACH`.
  - Supports List values, Dict keys, and Arrays:
    - 1‑D arrays iterate elements
    - 2×N arrays iterate the first column as keys (see notes)
  - Non‑iterables yield zero iterations (soft no‑op).

- Index and meta variables (read‑only) are injected during iterations and available in expressions/directives:
  - `_index%` (0‑based index)
  - `_number%` (1‑based index)
  - `_count%` (total iterations)
  - `_first%` (Bool)
  - `_last%` (Bool)

- Deterministic dictionary iteration variants (exact names):
  - `@FOREACH_KSORT(key$ IN dict@)` — sort by key ascending
  - `@FOREACH_VSORT(key$ IN dict@)` — sort by value ascending (string comparison of rendered values)
  - `@FOREACH_KSORT_DESC(key$ IN dict@)` — sort by key descending
  - `@FOREACH_DSORT_DESC(key$ IN dict@)` — sort by value descending (string comparison)

- Range and numeric loops:
  - `@TIMES(n)` … `@ENDTIMES`
  - `@FOR(i% = start TO end [STEP step])` … `@ENDFOR`
  - `@WHILE(cond)` … `@ENDWHILE`
  - Meta variables are populated in these loops as well.

- Loop control:
  - `@BREAK` and `@CONTINUE` affect the innermost Fred loop
  - Using them outside a loop is a render error

Notes:
- 2‑D arrays must have exactly 2 columns to be treated as key/value rows; the single‑var form yields keys (column 0).
- Loop variables are injected into the template scope via the same overlay used by RENDER$ context dictionaries. If you choose a suffixed name (like `key$` or `i%`), use that exact name in the header.

### Examples

Iterate a list with meta variables:

```
LET nums@ = [10, 20, 30]
PRINT RENDER$("""
@FOREACH(n% IN nums@)
  {{ _number% }}/{{ _count% }}: {{ n% }}
@FORELSE
  empty
@ENDFOREACH
""")
```

Deterministic dict iteration by key ascending:

```
LET pet@ = { "name": "Fido", "species": "dog", "age": 7 }
PRINT RENDER$("""
@FOREACH_KSORT(k$ IN pet@)
  {{ k$ }} = {{ pet@[k$] }}
@ENDFOREACH
""")
```

Repeat N times and use BREAK/CONTINUE:

```
PRINT RENDER$("""
@TIMES(10)
  {{ _number% }}
  @IF(_number% == 3) @CONTINUE() @ENDIF
  @IF(_number% == 8) @BREAK() @ENDIF
@ENDTIMES
""")
```

Numeric FOR loop with STEP and meta variables:

```
PRINT RENDER$("""
@FOR(i% = 5 TO 1 STEP -2)
  i={{ i% }} first={{ _first% }} last={{ _last% }}
@ENDFOR
""")
```

While loop:

```
LET n% = 3
PRINT RENDER$("""
@WHILE(n% > 0)
  n is {{ n% }}
  {{ n% = n% - 1 }}
@ENDWHILE
""")
```

---

### Backward compatibility and risk

- All changes are confined to the renderer (`basilcore/vm/src/render.rs`); no Basil compiler/VM opcode changes required.
- The new directives do not conflict with existing ones. `@FORELSE` and `@ENDFOREACH` are only meaningful inside
  `@FOREACH` blocks; their appearance elsewhere will be a parse error with a clear message.
- Dictionary iteration order remains unspecified (status quo for related operations like `IMPLODE$`); this is
  documented.

---

### Performance considerations

- Looping does not spawn child VMs by itself. Only `{{ … }}` expressions inside the body spawn the usual child VM per
  expression — unchanged from today.
- Overlay updates are O(1) per iteration; collections are iterated linearly.
- For dicts, if a sorted variant is enabled, sorting is O(n log n) on keys.

---

### Ask/decisions

- Should we include the two‑variable form (`@FOREACH(key$, value IN dict@)`) in v1, or defer to a follow‑up?
- For 2‑D arrays (2 columns), do you want the base `@FOREACH(var IN rows$())` to yield keys (column 0) as proposed, or
  should it default to a two‑var requirement to avoid ambiguity?
- Do you prefer that non‑iterables cause a soft no‑op (0 iterations) or a hard error in v1?

If this proposal looks good, I’ll implement the single‑var form with `@FORELSE` first and keep the two‑var and metadata
conveniences as small, follow‑up additions.