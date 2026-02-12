### What’s happening today
- `RENDER$` evaluates `{{ … }}` and `@…` arguments by compiling a tiny snippet and running it in a fresh child VM. See `basilcore/vm/src/render.rs:327–347`:
  - It builds `LET __RENDER_TMP = (expr);`, compiles it, spins up a child `VM`, then seeds ONLY the parent’s globals into that child via `vm.globals_snapshot()`.
  - Locals/parameters from the current function frame are not captured or available.
- That’s why module-level globals are visible in templates, while function locals and parameters are not.

### Two viable solutions

#### Option 1 — Make `RENDER$` see caller locals/params automatically
- What’s required
  - Compiler update: preserve local variable names per frame in bytecode (e.g., `Chunk.local_names: Vec<Option<String>>`). The doc already hints this approach: `docs/development/VIEW_RENDERING_AND_FRED.md` under “Evaluating Basil expressions from inside RENDER$”.
  - VM update: expose a helper to capture the current frame’s locals by name → value, alongside existing globals.
  - Renderer change: when evaluating an expression, pre‑seed the child VM with both globals and captured locals so identifiers resolve as if you were in the caller’s scope.
- Pros
  - Most ergonomic: template expressions “just work” with locals/params, matching natural expectations and the doc’s intent.
  - No template call‑site changes.
- Cons / Risks
  - Cross‑crate change touching the compiler and VM; adds metadata to bytecode (small but non‑zero); requires careful versioning.
  - Must define shadowing precedence: locals/params should win over globals; ensure this matches regular Basil scoping.
  - Debugger/introspection may also need to learn about local names if not already present.
- Effort/complexity: medium–high (compiler + VM + renderer), but straightforward conceptually.

#### Option 2 — Add an optional context dictionary: `RENDER$(template$, context@)`
- What’s required
  - Extend builtin id 149 to accept 1 or 2 args: second arg must be a Dict.
  - Pass this dict into the renderer; when evaluating an expression, inject the dict’s key/value pairs as variables in the child VM before running the snippet.
  - Reuse the same context for nested `@INCLUDE` calls.
- Pros
  - Low risk, self‑contained in VM/renderer (no compiler changes).
  - Very flexible: the caller chooses exactly what to expose; easy to test; enables use cases like mapping database rows directly into template scope.
- Cons
  - Requires a small change at each call‑site to pass the dict.
  - Doesn’t automatically expose existing locals/params unless you build the dict (which is often fine and explicit).
- Effort/complexity: low (VM/renderer only).

### Name mapping details for Option 2 (proposed)
- Variable creation:
  - For each context key `k` (string): inject a global named exactly `k` with the corresponding value.
  - Additionally, if the value’s type implies a Basil suffix, also inject the suffixed form to match common template habits:
    - String → also inject `k$`
    - Integer → also inject `k%`
    - Float/Bool/Dict/List/Array → just `k` (no suffix), unless the key already includes one
  - If the key itself already ends with a type suffix, keep it verbatim (no auto‑duplication).
- Precedence (if we later also add Option 1): locals/params > context dict > module globals. With Option 2 alone: context dict > module globals.
- Includes: pass the same overlay so `@INCLUDE` retains the calling scope.

### Recommendation
- Implement Option 2 now (optional `context@` argument). It delivers immediate value with minimal risk and no compiler changes. Your example becomes:
  ```basil
  LET pet@ = { "name": "Fido", "species": "dog", "age": 7, "weight": 15.0 }
  PRINT RENDER$(view$, pet@)  ' template can use name$, species$, age%, weight
  ```
- Then, as a follow‑up enhancement, consider Option 1 to align with the “caller’s scope” goal in the dev doc. Keep Option 2 even after Option 1 lands (it’s useful for augmenting/overriding and for rendering with arbitrary data).

### Implementation outline (Option 2)
- VM builtin: `basilcore/vm/src/lib.rs` (id 149)
  - Accept 1 or 2 args; if 2nd is a Dict, pass it; otherwise, type error.
  - Signature change at the VM level only; compiler keeps mapping name → id, no syntax change.
- Renderer:
  - `render::render_template(vm, tpl)` → `render::render_template(vm, tpl, ctx_dict: Option<Rc<RefCell<HashMap<String, Value>>>>)`
  - Thread `ctx_dict` through all evaluation helpers, including `@INCLUDE` recursion.
  - Before running each compiled snippet inside `eval_basil_expr`, seed the child VM with:
    1) Parent globals (current behavior), then
    2) Context dict keys (applying the suffix mapping noted above), overwriting any existing same‑named globals.
- Tests/Examples:
  - Add/extend `examples/render.basil` to include:
    - Using context to expose locals/params.
    - Conflicts where context overrides a global.
    - `@INCLUDE` retains context.
  - Update `docs/guides/RENDER_AND_FRED.md` with the new signature and examples.

### (If you prefer Option 1 first)
- Plan summary
  - Compiler: store `local_names` per function in `Chunk` (release builds can keep it; size is small). Ensure mapping slot→name excludes temporaries.
  - VM: add a helper `capture_caller_scope()` that returns a name→value map: locals/params + globals.
  - Renderer: when evaluating expressions, seed child VM with that scope; define clear precedence and ensure `@INCLUDE` reuses the same captured scope.
- Risks are manageable, but cross‑crate changes increase cycle time. Performance remains dominated by “compile small snippet per expression,” which is already the case today.

### My suggestion
Given complexity, risk, and user ergonomics:
- Short term: Option 2 (optional context dict) — quick win, minimal surface area, powerful.
- Medium term: Option 1 (automatic locals/params) — aligns behavior with expectations and your dev doc; retain Option 2 alongside it.

Would you like me to proceed with Option 2 now (implementing the optional dictionary parameter), and then scope Option 1 as a follow‑up? I can also add a concrete example to `examples/render.basil` to match your `pet@` scenario.