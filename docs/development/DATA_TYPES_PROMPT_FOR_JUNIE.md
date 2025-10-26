Implement Basil Data Literals, Fixed Strings, and BASIC Structs (TYPE)

Goal
Add list literals, dictionary literals, fixed-length strings, and BASIC-style TYPE…END TYPE structs to Basil. Keep everything backward-compatible with classic arrays and existing syntax. We already support `{`/`}` as block delimiters; now we’ll ALSO allow `{…}` in expression position for dictionary literals. Disambiguate by context (detailed below). Provide multiple example scripts in `examples/`. No tests. No Git steps.

Scope (must do)

1. Parser & AST
2. Runtime data structures and ops
3. Desugaring and typing rules
4. Indexing semantics (`[]` vs `()`)
5. LEN semantics for lists/dicts/strings/structs
6. Examples in `examples/` (at least 6 files)
7. Provide a written description of the new features in the Basil docs, including examples, in /docs/guides.
Do NOT

* Add unit/integration tests.
* Modify Git/branches/PRs.
* Change existing bytecode formats unless needed for new ops. If you add ops, keep them minimal and documented.

Language Additions

A) List Literals

* Syntax (expression position ONLY):
  `[ <expr> ( , <expr> )* ]` with optional trailing comma.
* Typing & desugaring:
  • If all elements unify to a concrete primitive type (int `%`, string `$`, float), treat as a 1-D **typed array** of that type; desugar to `DIM …(N)` + element assigns when bound to a variable with an array type.
  • Otherwise, treat as a **dynamic list** (vector of `Value`).
* Indexing: lists use square brackets: `L[1]`. (1-based indexing to match Basil arrays—keep consistent with current semantics.)
* Mutability: allow element assignment `L[i] = expr`. (We don’t need push/pop yet.)

B) Dictionary Literals (Associative Arrays)

* Syntax (expression position ONLY; JSON-ish):
  `{ <string_literal> : <expr> ( , <string_literal> : <expr> )* }` with optional trailing comma.
  Keys must be quoted strings for now (no bare identifiers).
* Typing: `Dict<String, Value>` (or equivalent).
* Indexing: `D["name"]` using square brackets with a string key.
* Optional alias OFF by default: do not allow `D("name")` now to avoid conflict with classic arrays. You may add it later behind a feature flag if needed.

C) Disambiguation: `{}` as Block vs Dictionary

* Rule: if `{` appears **immediately** after a control header that expects a block (`IF …`, `ELSE …`, `ELSEIF …`, `WHILE …`, `FOR …`, `FOR EACH …`, `SELECT CASE …`, `CASE …`, `DEFAULT`), it is a **block delimiter**.
* Otherwise, if `{` appears where an **expression** is expected, parse a **dictionary literal**.
* Lookahead guidance: after `IF <expr>` with optional `THEN`, seeing `{` → block; after tokens that can start PrimaryExpr, `{` → dict literal. Keep existing modern-syntax behavior for braces in control flow.

D) Fixed-Length Strings

* Support BOTH syntaxes (both must work):

    1. `DIM A$ AS STRING * N`
    2. `DIM A$[N]`
* Semantics: value stores exactly N bytes. On assignment, **truncate** at UTF-8 codepoint boundary to ≤ N bytes, then **pad** with spaces to N.
* `LEN(A$)` returns **N** (declared byte length), not visible character count. (Later we can add a function to count codepoints if needed.)
* Fixed strings allowed in struct fields (see below).

E) TYPE … END TYPE (Structs)

* Syntax:

  ```
  TYPE TypeName
    DIM FieldDecl...
  END TYPE
  ```

  FieldDecl uses existing DIM grammar with additions above, including arrays and fixed strings.
  Example:

  ```
  TYPE MyStructType
    DIM A$ AS STRING * 50
    DIM B$ AS STRING * 50
    DIM Y%
    DIM Z%
    DIM Child AS TYPE AnotherStructType
  END TYPE
  ```
* Declaration:

  ```
  DIM rec AS MyStructType
  DIM rows(100) AS MyStructType
  ```
* Field access: `rec.A$`, `rows(5).Y%`, `rec.Child.Z%`.
* Layout & size: **packed** by default (no implicit padding).
  • `%` = 32-bit int (use our existing size), float = current default, etc.
  • `STRING * N` contributes N bytes inline.
  • Variable-length `STRING` contributes a pointer/handle size (document size used), but for **LEN(TypeName)**:

    * If any variable-length fields exist, LEN(type) is **not a fixed record size**. Return 0 or raise a compile-time/ runtime error with message: “Struct contains variable-length fields; size is not fixed.”
    * If all fields are fixed-size, LEN(type) returns the sum of fixed bytes.
* Initialization sugar (optional but preferred now): allow partial initializer with dict-style field map on declaration:

  ```
  DIM rec AS MyStructType = { A$: "Hello", Y%: 42 }
  ```

  Desugar to zero-init + assignments in order.

Grammar/Parser Work

Tokens to ensure are present (case-insensitive where relevant):

* `LBRACKET '['`, `RBRACKET ']'`, `LBRACE '{'`, `RBRACE '}'`, `COLON ':'`, `COMMA ','`, `STAR '*'`.
* Existing `TYPE`, `END`, `AS`, `STRING`, `DIM`, `FOR`, `WHILE`, `IF`, `ELSE`, `ELSEIF`, `SELECT`, `CASE`, `DEFAULT` remain unchanged.

Add to expression grammar (PrimaryExpr):

* `ListLiteral := '[' (Expr (',' Expr)*)? ','? ']'`
* `DictLiteral := '{' (StringLiteral ':' Expr (',' StringLiteral ':' Expr)*)? ','? '}'`
* `PostfixIndex := PrimaryExpr '[' Expr ']'` (binds after PrimaryExpr; can chain with `.`)
* Keep classic array indexing with `()` unchanged.

DIM extensions:

* `DIM Id (Dims?) (TypeClause?) (Init?)`
* `Dims` remains classic array dims via `(`…`)`.
* `TypeClause` additions:
  • `AS STRING ('*' IntegerLiteral)?`
  • `AS TYPE TypeName`
  • `AS TypeName` (struct alias without keyword is okay if unambiguous)
* Fixed-string bracket form: recognize `Id '$' '[' IntegerLiteral ']'` as equivalent to `AS STRING * N`.

TYPE grammar:

* `TYPE TypeName FieldList END TYPE`
* `Field := 'DIM' Id (Dims?) (TypeClause)`
* Allow nested `AS TYPE OtherType`.

Name Resolution & Typing Rules

* List literal typing:
  • Try to unify element types; if unified to a primitive, it can initialize a typed array target.
  • If assigned to a variable with explicit array type (e.g., `DIM a%()`), type-check elements and perform element-wise assignment at init.
  • Else produce a dynamic list object.
* Dictionary literal: always `Dict<String, Value>`.
* Indexing:
  • `array(i)` = classic arrays.
  • `list[i]`, `dict["k"]` via new postfix `[]`.
  • Allow chained access: `people(1).Name$`, `pets[1]["name"]`, `rows(2).Child.Label$`.
* LEN:
  • `LEN([…])` = element count.
  • `LEN({…})` = key count.
  • `LEN(A$)` fixed string = declared N.
  • `LEN(TypeName)` or `LEN(var)` when fixed = fixed byte size; error/0 when variable-length fields exist.

Runtime Additions (sketch)

* Implement `List` (Vec<Value>) and `Dict` (HashMap<String, Value>) in the VM runtime layer.
* Add ops or library helpers for:
  • Construct list/dict literals.
  • Index get/set for lists and dicts (bounds/key checks → runtime errors with clear messages).
* Fixed strings:
  • Store as inline fixed-size buffers in variables/struct fields.
  • On assign: UTF-8 safe truncate + space pad.
  • On read: produce a String value (without trimming) or a view; keep trailing spaces unless the user calls a trim function.
* Structs:
  • Type descriptor carries field offsets and fixed size (if fully fixed).
  • Instances: contiguous blob for fixed part; variable strings or containers stored by handle/refs if present.
  • Field get/set uses computed offsets.

Errors & Messages (make friendly)

* “`{` here starts a dictionary literal. For a block, place it after IF/WHILE/FOR/SELECT headers.”
* “Attempted `[]` on a non-list/dict value.”
* “Attempted `()` on a non-array value.”
* “Struct contains variable-length fields; size is not fixed” (on LEN when variable fields present).
* Bounds/key errors include the index/key value in the message.

Examples (place these exact files under `examples/`)

1. `examples/lists_basic.basil`

```
DIM a% = [1,2,3]
PRINT a%(1); ", "; a%(2); ", "; a%(3)

DIM mixed@ = [1, "two", 3.0]
PRINT mixed[2]
mixed[2] = "TWO"
PRINT mixed[2]
PRINT "len mixed = "; LEN(mixed)
```

2. `examples/dicts_basic.basil`

```
DIM pet@ = { "name": "Fido", "species": "dog", "age": 7 }
PRINT pet["name"]
pet["age"] = pet["age"] + 1
PRINT "age = "; pet["age"]
PRINT "keys = "; LEN(pet)
```

3. `examples/fixed_string.basil`

```
DIM code$ AS STRING * 5
code$ = "ABCDEF"
PRINT code$           ' prints "ABCDE" + trailing spaces may be preserved
PRINT LEN(code$)      ' 5

DIM tag$[3]
tag$ = "ππππ"
PRINT LEN(tag$)       ' 3 bytes; value truncated safely at codepoint boundary
```

4. `examples/struct_fixed.basil`

```
TYPE MyStructType
  DIM A$ AS STRING * 8
  DIM B$ AS STRING * 4
  DIM Y%
  DIM Z%
END TYPE

DIM rec AS MyStructType = { A$: "Hello", Y%: 42 }
PRINT rec.A$; " "; rec.Y%
PRINT "RecordSize="; LEN(MyStructType)
```

5. `examples/struct_array.basil`

```
TYPE Person
  DIM Name$ AS STRING * 16
  DIM Age%
END TYPE

DIM people(3) AS Person
people(1).Name$ = "Alice"
people(1).Age% = 30
people(2).Name$ = "Bob"
people(2).Age% = 25

FOR i = 1 TO 2 {
  PRINT people(i).Name$; ": "; people(i).Age%
}
```

6. `examples/nested_structs_and_containers.basil`

```
TYPE ChildType
  DIM Label$ AS STRING * 8
END TYPE

TYPE ParentType
  DIM Child AS TYPE ChildType
  DIM Notes$   ' variable-length
END TYPE

DIM p AS ParentType = { Child: { Label$: "X1" }, Notes$: "hello" }
PRINT p.Child.Label$
PRINT "LEN(ParentType) = "; LEN(ParentType)   ' expect 0 or error due to variable field
```

7. `examples/lists_dicts_mix.basil`

```
DIM pets@ = [
  { "name": "Fido", "species": "dog" },
  { "name": "Misty", "species": "cat" }
]
PRINT pets[1]["name"]; " & "; pets[2]["species"]
```

Implementation Notes / Hints

* Keep arrays on `()` and lists/dicts on `[]`. Do not normalize them in the parser; keep them distinct through AST and VM.
* Make `{}` decision purely by syntactic position (control-header vs expression). Parenthesized expression like `x = ({ "a": 1 })` must parse as dict.
* Allow commas and optional trailing commas in both list and dict literals to ease formatting.
* Respect existing case-insensitivity; keywords in examples may be lower-case or upper-case.
* 1-based indexing applies to lists for consistency with arrays (document it in comments at the top of examples).

Deliverables

* Updated lexer/parser/AST with new literal nodes and postfix `[]` indexing.
* Type checker or binder updates to support list→array unification on homogeneous literals.
* Runtime support for `List`, `Dict`, fixed-length string storage, struct layout (packed), field access.
* Clear error messages described above.
* The example Basil scripts under `examples/` exactly as listed (plus any additional you find useful).

Out of scope (for now)

* Methods like `PUSH`, `POP`, `KEYS()`, etc. (We can add library helpers later.)
* Formatter/linter changes.
* Editor/LSP changes.

Please proceed with this implementation.
