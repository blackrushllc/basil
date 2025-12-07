### Objective
Add a single built-in function `RENDER$(template$)` to Basil that:
- Processes an input string as a template and returns a string.
- Evaluates `{{ … }}` segments as Basil expressions in the caller’s scope.
- Parses and executes a minimal, Blade-like “Fred” directive language using `@NAME(args)` syntax, with two families:
  - Direct evaluation directives, inlined to the output.
  - Flow-control directives (`@IF … @ELSE … @ENDIF`, `@CASE() … @CASE() … @ENDCASE`) that conditionally include parts of the template.
- Allows nesting, recursion via `@INCLUDE`, and a fallback to user-defined Basil functions when a Fred name is not a built-in directive.
- Is designed to be extensible for new directives.

Below is an implementation-ready plan that fits Basil’s architecture (lexer → parser → compiler → VM with built-in dispatch) and keeps room for growth while remaining easy for learners.


### Architectural checkpoints (today’s Basil)
- Built-ins are compiled to `Op::Builtin(id, argc)` and implemented in `basilcore/vm/src/lib.rs` under the big `match bid { … }` (e.g., ids for `REMOVE$`, `REPLACE$`, `INSERT$`, `DATE$`, etc.). The compiler maps names → ids.
- `EVAL(expr$)` exists and compiles a snippet at runtime inside a fresh child VM, not in the caller’s lexical scope.
- The VM has full access to global names/values at runtime (`vm.global_names`, `vm.globals`) but does not currently expose a way to enumerate local variable names in the current frame.
- Strings are UTF‑8; there is already URL encode/decode support (`vm.url_encode_form`, `vm.url_decode_form`) and CGI helpers for GET/POST caches.

Implication for `RENDER$`: we can add it as a new builtin and implement a renderer inside the VM (or a helper module it calls). To evaluate `{{ … }}` and Fred-argument Basil expressions “in the caller’s scope,” we need a practical strategy for seeing variables and functions.


### RENDER$ contract
- Signature: `RENDER$(template$) -> String`
- Semantics: Interprets `template$` as a template:
  1) Replace all `{{ expr }}` with the evaluated result of the Basil expression `expr`.
  2) Parse and execute Fred directives (`@…`) within the string, producing conditional or computed output.
  3) Return the final string.
- Determinism: Pure function with respect to template + current Basil state (variables, functions, environment/CGI when using related directives).
- Errors: Throw a `BasilError` with a clear message (template position, directive name) on parse/eval errors. Prefer recovery where possible (emit visible placeholder text when configured, see “Error handling” below).
- Recursion: `@INCLUDE` calls `RENDER$` recursively.


### Syntax processed inside RENDER$
1) Basil expression interpolation: `{{ expr }}`
   - `expr` uses Basil’s expression grammar.
   - Trims surrounding whitespace inside braces (e.g., `{{  A$  }}` is fine).
   - Output formatting mirrors Basil’s default string conversions for numbers/booleans/null.
   - No escaping variants (raw vs HTML-escaped) for v1, to keep rules simple (see Future Suggestions).

2) Fred directives: `@NAME(arg1, arg2, …)`
   - Names are case-insensitive.
   - Arguments are zero or more Basil expressions separated by commas.
   - Two families:
     - Direct evaluation (inline replacement): `@LEFT(…)`, `@URLENCODE(…)`, etc.
     - Flow control blocks:
       - `@IF(cond)` … `@ELSE` … `@ENDIF`
       - `@CASE(cond1)` … `@CASE(cond2)` … `@ENDCASE`
   - Nesting rules:
     - Blocks may nest (e.g., an `@IF` within another `@IF` block, etc.).
     - Directives can be used inside blocks and inside directive arguments (e.g., `@TRIM(@LEFT(A$,2))`).

3) Fallback to user-defined Basil functions:
   - If `@NAME` is not a recognized Fred directive, check for a Basil function with the same name (case-insensitive) and call it with evaluated Basil-argument values.
   - Return value must be printable (string/integer/real/bool/null); convert using the same rules as `PRINT`/string concatenation.


### Parser and evaluator design for RENDER$
We recommend a single renderer module (e.g., `basilcore/vm/render.rs`) used by `Op::Builtin(RENDER$)`. It implements a light template engine:

- Tokenizer (single pass over the template string) that yields nodes:
  - Text(text)
  - Interp(expr_source) for `{{ … }}`
  - FredCall(name, arg_exprs) for direct `@NAME(args)`
  - FredBlockStart(kind, arg_exprs) for `@IF(args)` / `@CASE(args)`
  - FredElse for `@ELSE` (only valid in IF)
  - FredBlockNextCase(arg_exprs) for subsequent `@CASE(args)` within a CASE block
  - FredBlockEnd(kind) for `@ENDIF` / `@ENDCASE`

- Minimal grammar rules:
  - `{{` … `}}` may appear anywhere. Must handle nested braces inside strings/parentheses of the Basil expr by counting or delegating to Basil’s parser after extraction.
  - `@NAME(` starts a call or block. Parse until matching `)` with parentheses/quotes balance. Then lookahead:
    - If `NAME` is `IF` or `CASE` → BlockStart token.
    - If `NAME` is `ELSE`, `ENDIF`, `ENDCASE` → zero-arg block control tokens.
    - Otherwise → direct FredCall.
  - Text is everything else.

- Block builder (second pass or on-the-fly using a stack):
  - Build a tree/stack of blocks as you consume tokens.
  - For IF: a node with fields `{cond, then_children, else_children}`.
  - For CASE: a list of arms `{cond, children}` with implicit “first true wins” semantics.

- Evaluation:
  - DFS over the tree:
    - Text → append.
    - Interp → evaluate `expr_source` and append its string form.
    - FredCall → dispatch via directive registry (below) or UDF fallback.
    - IF → evaluate cond; choose branch; eval child nodes.
    - CASE → evaluate arms in order; eval first true branch (if none true, output nothing).

- Escaping rules in v1:
  - Literal `@` in text: write `@@` to escape, or recommend wrapping in `{{ "@" }}`. For simplicity, we can define: a single `@` followed by a non-letter isn’t a directive (so emails like `NAME@domain` remain text). The tokenizer should require `@` followed by `[A-Za-z_]` to start a directive.
  - Literal `{{` can be written as `\{{` or by breaking it up via e.g. `{{ "{" }}{` if needed. We can add a first-class escape in a later iteration.


### Evaluating Basil expressions from inside RENDER$
We must evaluate expressions in the caller’s scope (globals + locals + functions). There are two robust approaches:

1) Preferred: Add lightweight runtime reflection for locals.
   - Compiler change: for each function (and top level), store a vector `local_names: Vec<Option<String>>` in the `Chunk` mapping local slot → name. This can be stripped in release builds if desired but is small.
   - VM change: expose a helper that gathers a name→value map for the current frame:
     - Each local name (where present) maps to `stack[frame.base + slot]`.
     - Also gather all globals via `vm.global_names`/`vm.globals`.
   - Expression evaluation strategy:
     - Parse the `expr` using `basil_parser::parse` wrapped into a `LET __T = (expr)` snippet, compile to a small `BCProgram`, but before running the child VM, pre-populate its globals with the captured name→value map from the caller (both globals and locals). This lets the compiled expression refer to variables by name as if in the caller scope.
   - Pros: Reuses the existing compiler/VM and yields consistent semantics for all Basil expressions.

2) Alternative (more work, less invasive later): a same-VM expression evaluator.
   - Parse the expression and evaluate it directly against the current VM using a small interpreter for expressions only (variables, literals, operators, calls). This avoids creating a child VM per expression but requires duplicating evaluation rules and function calling semantics.
   - Start with option 1 (simple and correct), optimize later if needed.

For UDF fallback in Fred (`@MYFUNC(...)`):
- Use the same approach: build a tiny snippet to call the function and capture the result (`LET __T = MYFUNC(args…)`), run it in a child VM pre-populated with caller’s captured scope. This guarantees parameter coercion and return conversions identical to normal Basil.

Performance note: to reduce overhead, implement a micro cache of compiled single-expression programs by source text (and maybe by a hash of the captured symbol set), and reuse the compiled program where safe. Start without caching; measure later.


### Directive registry and built-ins (v1)
Create a `FredRegistry` that maps uppercase directive names to handler functions or trait objects. Handlers receive the VM, a vector of pre-evaluated Basil argument Values (unless they choose to evaluate themselves), and a context object for path resolution and CGI state. Suggested v1 directives:

Direct evaluation directives
- `@LEFT(s$, n%)` → string left `n` chars. Use Basil semantics (UTF‑8 scalar, 1‑based where applicable). If Basil already has `LEFT$`, this directive can call into that same helper or reimplement simply.
- `@RIGHT(s$, n%)` → right `n` chars.
- `@MID(s$, start%, len%)` → substring by character positions and length.
- `@TRIM(s$)` → trim leading/trailing ASCII whitespace (or Unicode whitespace if Basil standard library does so). Use the same rule as Basil’s `TRIM$` if it exists.
- `@URLENCODE(s$)` → use `vm.url_encode_form` for application/x-www-form-urlencoded escaping.
- `@SESSION(name$)` → get session var or `null` if not in CGI/undefined. Implementation detail depends on Basil’s CGI session story; stub to `null` outside CGI.
- `@ENV(name$)` → `std::env::var_os` lookup; to string lossily; `null` if missing.
- `@SERVER(name$)` → read common CGI `SERVER_*` environment variables; else `null`.
- `@REQUEST(name$)` → from GET/POST merged params. Use `vm.ensure_get_params()` / `vm.ensure_post_params()` caches and parse pairs; return first match, or `null` if missing.
- `@INCLUDE(path$)` → resolve relative to the running script’s directory (via `vm.script_path`), read text if the file exists, else `"null"`. Apply `RENDER$` recursively to the included content and insert the rendered string.

Flow-control directives
- `@IF(cond)` … `@ELSE` … `@ENDIF`
  - Truthiness mirrors Basil’s `IF`: nonzero nums and nonempty strings are true; null is false.
- `@CASE(cond)` … `@CASE(cond)` … `@ENDCASE`
  - Interpreted as an if-else-if chain: evaluate `cond` for each `@CASE` in order; render the first true block; skip others. No `@ELSE` in v1; add later if desired.

All directive names are case-insensitive and must be a whole word after `@`.


### Interaction rules between `{{ … }}` and `@…`
- Both can freely nest. Evaluation is performed on the final AST:
  - Flow control determines which text/interpolations/directive calls are visited.
  - When a visited node is an interpolation or directive argument, evaluate Basil expressions at that moment with current scope capture.
- Whitespace control: keep v1 simple; do not strip lines automatically around `@IF`/`@CASE`. In docs, offer examples to handle newlines.


### Security considerations
- Template input is potentially user-provided in CGI contexts. Since `{{ … }}` and arguments are full Basil expressions, executing untrusted templates can be dangerous. Recommend:
  - Treat templates as trusted by default. For untrusted cases, introduce a “safe mode” flag later that disables `{{ … }}` and limits directives to a whitelist (no `@INCLUDE`, no UDF fallback).
  - Path resolution for `@INCLUDE`: normalize and confine to a base directory to avoid directory traversal.
  - Limit maximum recursion depth and overall expansion size (byte budget) to prevent DoS.


### Performance considerations
- Single pass tokenization + stack-based block building are `O(n)` in template length.
- Expression evaluation via “child VM per expression” is simplest but can be heavy in very large templates. Consider:
  - Micro-caching compiled one-liners.
  - Batching: if a block contains many `{{ … }}` that reference only globals, reusing the same pre-populated child VM instance for that block.
  - A later optimization path: implement a compact expression evaluator for a subset (literals, vars, `+ - * /`, comparisons, function calls), falling back to child VM for complex cases.


### Error handling and diagnostics
- On tokenizer/parse error: include the byte index/line/column and a short caret context in the error message (like the lexer does for source code).
- On runtime evaluation error: wrap with template location context: `RENDER$: error in {{ … }} at line X, col Y: <inner error>` or `in @URLENCODE(...)`.
- Optionally provide a mode that inserts a visible marker into output (e.g., `[[ERROR: …]]`) instead of aborting entirely; controlled by a second optional param in a future `RENDER$` overload.


### Integration plan (concrete steps)
1) VM builtin and entry point
   - Assign a new builtin id (e.g., next available after 147) and name `RENDER$` in the compiler’s built-in map. Emit `Op::Builtin(ID_RENDER, 1)`.
   - In `vm::run`, handle `ID_RENDER` by popping a string, invoking `render::render_template(self, template, RenderOptions { … })`, and pushing the resulting string.

2) Add `basilcore/vm/render.rs`
   - Implement tokenizer, parser, AST nodes, evaluator, and directive registry.
   - Provide helpers:
     - `capture_scope(vm) -> HashMap<String, Value>` that merges globals + (if available) locals.
     - `eval_basil_expr_in_scope(vm, expr_src, scope) -> Result<Value>` using child-VM compilation and pre-populated globals.
     - `call_udf_in_scope(vm, name, args_values, scope) -> Result<Value>` similar to above.
   - Implement built-in directives as functions taking `(vm, args, ctx) -> Result<String>`.

3) Scope capture for locals (recommended change)
   - Compiler: store `local_names` in `Chunk` for each function frame. In release mode, keep it; size is small since names are short.
   - VM: expose an internal method to gather `{name → value}` for current frame by inspecting `frame.base` and stack.
   - If you want to ship v1 faster without locals, document that `{{ … }}` and Fred arguments see globals/UDFs only; later add local capture without breaking templates.

4) CGI hooks
   - Ensure GET/POST cache helpers are public to the renderer, and add small helpers:
     - `get_request_param(vm, name) -> Option<String>`
     - `get_server_var(name) -> Option<String>`
     - `get_session_var(name) -> Option<String>` (no-op/null outside CGI until sessions are formalized).

5) File inclusion
   - Resolve `@INCLUDE(path)` against `vm.script_path` directory by default.
   - Detect cycles with a small stack of absolute paths; enforce a max include depth (e.g., 16).

6) Testing and examples
   - Unit tests for tokenizer (edge cases: nested `()`, commas in strings, `@@`, `{{` in strings).
   - AST/block builder tests (nested IF/CASE, unmatched ENDIF/ENDCASE errors).
   - Eval tests: interpolation, directives, nested directives, INCLUDE recursion, REQUEST/ENV in non-CGI (return null).
   - Golden tests for end-to-end template rendering.
   - Examples under `examples/templating/` with README.

7) Documentation
   - A new guide `docs/guides/RENDER_AND_FRED.md`:
     - Quick start and table of supported directives.
     - Rules for `{{ … }}` vs `@…`.
     - Escaping tips.
     - Security notes.
   - Cross-link from `README.md` and CGI docs.


### Exact behaviors for v1 directives (spec)
- `@LEFT(s$, n%)` → first `n` characters of `s$` (Unicode-safe). Non-positive `n` → `""`. Greater than length → whole string.
- `@RIGHT(s$, n%)` → last `n` chars.
- `@MID(s$, start%, len%)` → starting at 1-based `start%`, length `len%`. Truncate to bounds; negative/zero `len%` → `""`.
- `@TRIM(s$)` → trim leading/trailing whitespace.
- `@URLENCODE(s$)` → use `vm.url_encode_form`.
- `@SESSION(name$)` → `string` or `null` as described.
- `@ENV(name$)` → env var or `null`.
- `@SERVER(name$)` → CGI/`SERVER_*` or `null`.
- `@REQUEST(name$)` → first matching GET/POST param (already parsed), else `null`.
- `@INCLUDE(path$)` → if file exists, read as UTF‑8 text, then `RENDER$` it; else insert `"null"`.
- `@IF(cond)` … `@ELSE` … `@ENDIF` → truthiness as Basil.
- `@CASE(cond)` … `@CASE(cond)` … `@ENDCASE` → evaluate in order; output first true block; no default arm in v1 (add `@ELSECASE` later if desired).


### Backward compatibility and scoping
- This feature is Basil-only; the Basic sister project remains unchanged.
- No syntax changes to Basil source files; all new parsing happens inside `RENDER$` at runtime on plain strings.
- If local-scope capture is deferred to a later minor version, document “in v1, `{{ … }}` and Fred arguments can only see globals and functions; locals not visible yet.” Upgrading later to include locals will only make more templates work, not break existing ones.


### Future extensions (keep rules simple, but helpful)
- Raw vs escaped output: add `{{{ expr }}}` for raw and keep `{{ … }}` to mean HTML-escaped (or vice versa), guarded by a simple HTML escape routine. Start with no auto-escaping and introduce a flag later.
- Shorthand `@ELSEIF(cond)` instead of `@ELSE … @IF(cond)` to reduce boilerplate.
- Looping: `@FOREACH(item, collection)` … `@ENDFOREACH` to iterate lists/arrays/dicts.
- Layouts/sections: `@EXTENDS("layout")`, `@SECTION("name")` … `@ENDSECTION`, `@YIELD("name")` — but only if you want a simple framework path.
- Template comments: `{{-- … --}}` or `@COMMENT … @ENDCOMMENT` to strip content from output.
- Safer mode: a `RENDER_SAFE$` variant or an optional second parameter `mode%` that disables UDF fallback, `@INCLUDE`, or `{{ … }}`.
- Caching: memoize rendered output for stable inputs and limited dynamic bits (etag-style), or cache the parsed template AST keyed by content hash.


### Summary of recommended path
- Implement `RENDER$` as a new builtin that calls a `render` module.
- Start with a clear tokenizer, small AST, and stack-based evaluator.
- Evaluate embedded Basil expressions by compiling one-liners in a child VM pre-populated with captured globals; add local capture metadata soon after for full scope.
- Provide the requested v1 directive set, UDF fallback, `@INCLUDE`, and recursion caps.
- Ship with crisp error messages, solid tests, and a concise guide.
- Extend later with escaping modes, `@ELSEIF`, loops, and an optional safe mode, keeping syntax and rules simple for learners.