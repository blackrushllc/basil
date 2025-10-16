### Short answer

It’s very doable. Adding array literals like `[...]` is a modest parser change, but you’ll want clear type-resolution
rules (especially for `[]`) and a couple of guardrails to avoid ambiguities with indexing. The biggest work is in type
inference and static checks, not the surface syntax.

### Proposed syntax (consistent with your style)

Given the existing `DIM`/`AS` style (e.g., `DIM db@ AS DB_MYSQL(dsn$)`), a natural, unambiguous shape is:

- Declaration with initialization:
    - ```
    DIM animals$[] AS STRING[] = ["cows", "horses", "chickens"]
    ```
    - or (if your grammar prefers type before variable name):
      ```
      DIM animals$ AS STRING[] = ["cows", "horses", "chickens"]
      ```
- Empty array literal with contextual type:
    - ```
    Foo([])               // If Foo expects, say, STRING[]
    ```
    - If there’s no context, allow an explicit type:
      ```
      LET empty$ = [] AS STRING[]
      // or
      LET empty$ = STRING[]([])
      ```
- Array literal anywhere an expression is valid:
    - ```
    Bar([1%, 2%, 3%])
    LET grid%% = [[1%,2%], [3%,4%]]   // array of int arrays
    ```

Note: Your example `DIM A$ AS ["cows", "horses", "chickens"]` reads like the `AS` slot is receiving a value (literal)
instead of a type. That would be unusual for your language’s existing pattern. Prefer `= [...]` to assign the literal,
and keep `AS` for the type.

### Behavior and rules

- Array literal grammar:
    - `ArrayLiteral := '[' (Expression (',' Expression)*)? ']'`
- Distinguish from indexing:
    - A `[` that starts a primary expression forms an array literal.
    - A `[` immediately following an expression/identifier is indexing. This is a standard, unambiguous parse.
- Element type inference:
    - All elements must be compatible with a single element type `T`.
    - For `[1%, 2%, 3%]` ⇒ `INT%[]`.
    - For `["a", "b"]` ⇒ `STRING[]`.
    - Mixed types: either
        - coerce to a common supertype (e.g., `ANY`/`VARIANT`), or
        - reject unless explicitly annotated, e.g., `([] AS ANY[])` or `[1%, "a"] AS ANY[]`.
- Empty array `[]`:
    - Requires contextual type (from assignment or parameter type), else a compile error asking for `AS T[]`.
- Constness and immutability:
    - If you have `CONST`, decide whether `CONST arr = [...]` freezes the array reference only or also the contents.
      Document it.
- Multi-dimensional vs. jagged arrays:
    - Literal `[[1,2],[3,4]]` is naturally a jagged array (`INT%[][]`). If you support true multi-dimensional arrays,
      keep literal syntax for jagged, and provide constructors for rectangular (e.g., `ARRAY2D(2,2, 1,2,3,4)`), unless
      you want additional sugar.
- Slices/spreads (optional future):
    - Consider later: `[..., xs, ...ys]` to splice arrays. Not necessary for v1.

### Passing `[]` as an argument

- Works out-of-the-box via contextual typing:
    - ```
    SUB TakesStrings(arr$ AS STRING[])
      ' ...
    END SUB

    TakesStrings([])  ' `[]` is typed as STRING[]
    ```
- Overload resolution prefers the signature for which the element type is solvable from context.

### Errors and diagnostics to include

- Mixed-element types without a common supertype: "array literal elements must have a compatible type; expected STRING,
  found INT".
- Bare `[]` where no type is inferable: "cannot infer element type for empty array literal; annotate with `AS T[]`".
- Trailing comma policy (pick one and document): allow `[1,2,]` or reject.

### Code generation/runtime

- For a literal `[e1, e2, ..., en]`:
    - Allocate a new array of length `n` with element type `T`.
    - Evaluate elements left-to-right and store.
    - If `n` is small and elements are constants, you may constant-fold into a static data structure (if your runtime
      supports it), but a simple allocate-and-fill is fine for v1.

### Backward compatibility concerns

- If `[` is already used for indexing, there’s no conflict as long as the parser distinguishes
  literal-at-expression-start vs. postfix indexing.
- If you use `[]` anywhere else (e.g., attribute syntax), verify no ambiguity.

### Implementation plan (low risk)

1. Lexer: ensure `[` and `]` tokens already exist (they do, if you support indexing).
2. Parser: add `ArrayLiteral` as a primary expression; precedence same as other literals.
3. AST: `ArrayLiteral { elements: Vec<Expr> }`.
4. Type checker:
    - If `elements` non-empty: unify element types; compute `T[]`.
    - If empty: request expected type from context; if none, emit diagnostic.
5. Constant folding (optional): detect all-constant literals.
6. Codegen: create array instance of type `T[]`, assign elements.
7. Tests:
    - Basic `[1,2,3]`, `[]` with/without context.
    - Nested arrays; mixed types errors; passing to functions; overload resolution.
    - Indexing after literal: `[1,2,3][0]` should work if you support it.

### Example suite

- Declarations and assignment:
  ```
  DIM animals$ AS STRING[] = ["cows", "horses", "chickens"]
  LET nums% = [1%, 2%, 3%]
  LET matrix%% = [[1%,2%], [3%,4%]]
  ```
- As arguments:
  ```
  SUB PrintAll(items$ AS STRING[])
    FOR EACH s$ IN items$
      PRINTLN s$
    NEXT s$
  END SUB

  PrintAll(["alpha", "beta"])  ' literal
  PrintAll([])                   ' empty literal; element type from parameter
  ```
- With explicit type on empty literal:
  ```
  LET emptyStrings$ = [] AS STRING[]
  LET emptyInts% = [] AS INT%[]
  ```

### How hard is it?

- Parser change: small.
- Type-checker changes: moderate (unification and context-based inference for `[]`).
- Codegen: trivial (allocate-and-fill).
- Overall: a tidy feature with high usability and low implementation risk if you keep the `[]` → element-type inference
  rules tight.

### Recommendation

Adopt array literals with the `[...]` syntax, require contextual typing for `[]`, and keep `AS` reserved for types with
`=` used for initialization. That stays consistent with your existing style and yields the ergonomic benefit you’re
after without risking ambiguity.