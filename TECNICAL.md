# Basil Technical Architecture and Execution Model

This document is a technical description of the Basil language toolchain and runtime as implemented in this repository. It is aimed at a technically sophisticated audience (e.g., PhD-level computer scientists) and focuses on the internal models, algorithms, bytecode, and runtime semantics. Where appropriate, we include simple BASIL code examples to illustrate the observable behavior of the underlying mechanisms.

- Project root: this repository is a Rust workspace with the following relevant crates:
  - basilc: command-line driver (compiler + runner + CGI template preprocessor)
  - basilcore/lexer: lexical analysis
  - basilcore/parser: parsing to AST
  - basilcore/ast: abstract syntax tree definitions
  - basilcore/compiler: AST → bytecode compiler
  - basilcore/bytecode: bytecode IR, value model, (de)serializer for .basilx cache files
  - basilcore/vm: stack-based, single-threaded bytecode virtual machine
  - basil-objects: extension registry for runtime OBJECT types (behind Cargo features)

The system supports direct CLI execution of .basil source files (with transparent bytecode caching) and HTTP CGI templating similar to classic PHP (with <?basil ... ?> blocks and a small directive language).


## 1. Language front-end

### 1.1 Lexical layer (basilcore/lexer)
- Source is tokenized into Token { kind, lexeme, span } where span carries byte offsets.
- TokenKind includes punctuation, operators, and reserved words for control flow and IO:
  - Structural: LParen, RParen, Comma, Semicolon, Dot
  - Arithmetic: Plus, Minus, Star, Slash
  - Relational: Lt, Gt, LtEq, GtEq, EqEq, BangEq
  - Keywords: LET, PRINT, PRINTLN, IF, THEN, ELSE, WHILE, DO, BEGIN, END, TRUE, FALSE, NULL, AND, OR, NOT
  - Control flow: FOR, TO, STEP, NEXT, EACH, IN, FOREACH, ENDFOR, BREAK, CONTINUE
  - Declarations: FUNC, RETURN, DIM, AS, NEW, DESCRIBE
  - Literals: Number (f64), String (UTF-8)
- Comments and whitespace are discarded. Strings are UTF-8 with standard escape handling.

Implementation notes:
- The lexer is a single-pass, demand-driven scanner with explicit handling of compound tokens (e.g., <=, >=, ==, !=) and BASIC-like keywords.
- Spans enable precise error reporting in later phases.


### 1.2 Abstract Syntax Tree (basilcore/ast)
- Expressions (Expr): Number, Str, Bool, Var, unary neg/not, binary ops, function Call, MemberGet/MemberCall, NewObject(type, args).
- Statements (Stmt): Let (with optional indices for array element assignment), Dim (arrays), DimObject/DimObjectArray, SetProp (object property set), Describe, Print, ExprStmt, Return, If, While, Break, Continue, Block, Func (named, parameter list, body), For (numeric), ForEach (enumeration), Line (for runtime line mapping).
- Program is a Vec<Stmt> (single compilation unit).


### 1.3 Parser (basilcore/parser)
- The parser is a hand-written, single-token lookahead recursive-descent parser using Pratt/precedence climbing for expressions (parse_expr_bp with binding powers).
- Statements follow BASIC-like forms, including both single-line and block forms (BEGIN..END). Control flow supports IF/THEN[/ELSE], WHILE, FOR, and FOR EACH.
- Function declarations produce named functions with arity checks enforced at call sites in the VM.


## 2. Compilation (basilcore/compiler)

The compiler lowers AST → bytecode in a single pass with environment tracking.

- Core structures:
  - Chunk { code: Vec<u8>, consts: Vec<Value> }: holds linearized bytecode and a constant pool (literals, function values, interned strings for property names, etc.).
  - Program { chunk, globals }: top-level Chunk plus a global-symbol vector for indexed lookup.
  - Function { arity, name, chunk }: first-class function value; nested functions are values in the constant pool and pushed onto the stack at use sites.

- Environment model:
  - Global names are interned into program.globals and referenced by u8 indices (Op::LoadGlobal/StoreGlobal).
  - Function-local temporaries and parameters are addressed via u8 slots relative to a frame base (Op::LoadLocal/StoreLocal). Locals are not lexically nested; functions close over nothing in this design (no closures).

- Control-flow lowering:
  - Structured constructs are translated to forward/backward branches via Jump, JumpIfFalse, JumpBack with 16-bit little-endian relative offsets. The compiler emits placeholders (Chunk::emit_u16_placeholder) and later patches them (patch_u16_at).
  - VM line correlation: the compiler emits SetLine opcodes (Op::SetLine u16) so the VM can report a current_line on runtime error.

- Type-directed operations:
  - The compiler does not encode static types; the VM performs dynamic dispatch/coercions. For Add, the VM concatenates if any operand is a string; otherwise numeric addition.

- Arrays and objects:
  - DIM and related forms compile to ArrMake, ArrGet, ArrSet, and object ops as appropriate.
  - Object member access, method calls, and construction produce GetProp, SetProp, CallMethod, NewObj bytecodes parameterized by constant-pool strings.


## 3. Bytecode IR and serialization (basilcore/bytecode)

### 3.1 Value model
- Value is a tagged union with variants: Null, Bool(bool), Num(f64), Int(i64), Str(String), Func(Rc<Function>), Array(Rc<ArrayObj>), Object(ObjectRef).
- Arrays are ArrayObj { elem: ElemType, dims: Vec<usize>, data: RefCell<Vec<Value>> } with element kinds: Num, Int, Str, Obj(Option<String>) — the last optionally constrains object type name.

### 3.2 Instruction set (selected)
- 8-bit opcode stream with variable-length instruction encodings (immediates serialized little-endian).
- Load/store:
  - ConstU8 idx
  - LoadGlobal idx, StoreGlobal idx
  - LoadLocal idx, StoreLocal idx
- Arithmetic/logic: Add, Sub, Mul, Div, Neg; Eq, Ne, Lt, Le, Gt, Ge
- Control flow: Jump u16, JumpIfFalse u16, JumpBack u16, Ret, Halt
- Stack: Pop, Print (pop and print), ToInt
- Functions: Call argc (callee/value placed before args), Ret
- Arrays: ArrMake(rank,u8 elem,u8 type_cidx), ArrGet(rank), ArrSet(rank)
- Enumeration: EnumNew, EnumMoveNext, EnumCurrent, EnumDispose
- Objects: NewObj, GetProp, SetProp, CallMethod, DescribeObj
- Metadata: SetLine u16
- Builtin: Builtin id,u8 argc — deferred to VM’s builtin dispatch

### 3.3 .basilx cache format
- CLI compiles .basil → Program and caches to sidecar .basilx. The cache begins with a fixed header then the serialized Program payload from serialize_program():
  - Magic: ASCII "BSLX"
  - Format version (u32 LE)
  - ABI/runtime version (u32 LE)
  - Flags (u32 LE): includes templating and "short tag" switches from the precompiler
  - Source size (u64 LE) and source mtime in nanoseconds (u64 LE)
  - Followed by the serialize_program byte stream (chunk + globals)
- On run, basilc attempts to validate and reuse the cache if header fields match (format/ABI/flags/size/mtime); otherwise re-parses and re-compiles, then rewrites the cache atomically.


## 4. Virtual Machine (basilcore/vm)

The VM is a deterministic, single-threaded, stack-based interpreter.

- Core state:
  - frames: Vec<Frame { chunk: Rc<Chunk>, ip: usize, base: usize }>
  - stack: Vec<Value>
  - globals: Vec<Value> sized to Program.globals
  - enums: small table for active array enumeration handles
  - registry: basil-objects Registry for NEW/GET/SET/CALL of OBJECTs
  - current_line: u32, updated by SetLine to produce accurate runtime error messages

- Execution loop:
  - Fetch-decode-dispatch over Op, reading immediates via read_u8/read_u16.
  - Function calls push frames with a base pointing to the callee slot; Return pops the frame, truncates the stack to base, and pushes the return value.
  - Add performs string concatenation if any operand is Str; otherwise numeric addition with dynamic coercion (Num/Int). Comparisons do numeric comparison where applicable.

- Arrays:
  - ArrMake validates rank ∈ [1,4], computes row-major size, allocates data with default element values (0.0, 0, "", or Null for object slots). DIM upper bounds are inclusive (BASIC style); dimension lengths are upper+1.
  - ArrGet/ArrSet compute linearized row-major indices; ArrSet includes element-type coercions and runtime checks for typed object arrays (ElemType::Obj(Some(type))).

- Enumeration:
  - FOR EACH over arrays lowers to EnumNew/MoveNext/Current/Dispose. The VM realizes an ArrEnum with total = product(dims), linear index over the array buffer.

- Objects:
  - NEW type(args…) calls Registry::make(type, args). Property get/set and method calls call into BasicObject trait on dynamically created instances; DescribeObj prints a synthesized descriptor string. FEATURES: types are registered conditionally via Cargo features (see basil-objects/src/lib.rs).

- Builtins:
  - Builtin dispatch is by small integer id with argc; arguments are popped, reversed to call order, and semantics enforced at runtime. Implemented functions include:
    - 1: LEN(x) — length of string in characters or total element count of array; else length of Display-coerced string
    - 2: MID$(s, start[, len]) — 1-based start index over characters; returns substring
    - 3: LEFT$(s, n) — first n characters
    - 4: RIGHT$(s, n) — last n characters
    - 5: INSTR(hay, needle[, start]) — returns 0-based character index or 0 if not found; empty needle returns start clamped to [0, |hay|]
    - 6: INPUT$([prompt]) — reads a line from stdin without trailing CR/LF
    - 7: INKEY$() — non-blocking key read; returns "" if no key
    - 8: INKEY%() — non-blocking key read; integer key codes (ASCII, arrows as 1000+, F-keys 1100+n, etc.)
    - 9: TYPE$(value) — returns a printable type string (e.g., "INTEGER", "STRING", "ARRAY", type name of object)
    - 10: HTML(x) / HTML$(x) — HTML-escapes & < > " '
    - 11: GET$() — returns STRING[] array of "key=value" for URL query (CGI)
    - 12: POST$() — returns STRING[] array of "key=value" for URL-encoded body (CGI)
    - 13: REQUEST$() — concatenation of GET$ and POST$
    - 14: UCASE$(s) — uppercase
    - 15: LCASE$(s) — lowercase
    - 16: TRIM$(s) — trim ASCII/Unicode whitespace at both ends
    - 17: CHR$(n) — Unicode scalar by code point (empty string if invalid)
    - 18: ASC%(s) — code point of first char or 0 if empty
    - 19: INPUTC$([prompt]) — raw single ASCII character capture with echo, draining pending events

- I/O and terminal details:
  - INKEY$/% and INPUTC$ use crossterm with raw mode toggling; the VM ensures raw mode is disabled on exit paths.


## 5. CLI driver and template engine (basilc)

### 5.1 CLI mode
- Commands (prototype): init, run, lex, plus stubs (build/test/fmt/etc.).
- run <file.basil> behavior:
  - Reads UTF-8 source; determines if template precompiler should be used (looks_like_template if contains "<?").
  - Precompiles templates to Basil source if applicable; otherwise uses source as-is.
  - Computes cache flags (templating, short tags), source size, and mtime; attempts to reuse sidecar .basilx if header matches; otherwise parses and compiles.
  - Serializes Program and writes .basilx atomically; then executes via VM with runtime error line mapping using SetLine.

### 5.2 CGI mode
- The same binary can operate as a CGI interpreter. A wrapper entrypoint detects CGI via GATEWAY_INTERFACE and REQUEST_METHOD and spawns itself in CLI run-mode on the target .basil script path resolved from the web server environment.
- A directive prelude in templates allows header policy control:
  - #CGI_NO_HEADER — program must print full headers, then a blank line
  - Optional default header override; short-tag toggle
- In automatic header mode, basilc sends Content-Type before the program output.
- Within Basil, REQUEST$(), GET$(), POST$(), and HTML$() connect templating to HTTP semantics.


## 6. Memory and safety model
- Values are reference-counted (Rc) with interior mutability (RefCell) where necessary (arrays, objects). There is no shared-state concurrency in the VM; execution is single-threaded.
- Arrays store Values and enforce element-type constraints at ArrSet time, including runtime checks for typed object arrays. Out-of-bounds and rank mismatches raise runtime errors.
- Functions are first-class; frames carry Rc<Chunk> allowing code sharing.


## 7. Error handling and diagnostics
- All front-end and VM phases use basil_common::Result with textual BasilError on failure.
- The compiler injects SetLine opcodes; the VM tracks current_line and the CLI emits "runtime error at line N: ..." when available.
- The lexer and parser include token/line tracking sufficient for precise diagnostics.


## 8. Extensibility via OBJECTs (basil-objects)
- The Registry maps type names → TypeInfo { factory, descriptor, constants }.
- Types can be conditionally compiled behind Cargo features (e.g., obj-bmx-rider, obj-bmx-team). NEW constructs an instance, GET/SET/CALL bridge to trait methods, and DESCRIBE formats property/method descriptors.
- Object arrays can be typed: DIM riders@(N) AS BMX_RIDER enforces type at ArrSet.


## 9. BASIL examples

### 9.1 Hello, world
```basil
PRINT "Hello, Basil!";
```

### 9.2 Fibonacci function
```basil
FUNC fib(n)
  IF n < 2 THEN RETURN n;
  RETURN fib(n - 1) + fib(n - 2);
END

FOR i = 0 TO 10
  PRINT fib(i);
  PRINT " ";
NEXT
PRINTLN;  ' newline
```

### 9.3 Arrays and FOR EACH
```basil
DIM a(4) AS INTEGER;   ' indices 0..4 (inclusive upper bound)
LET i = 0;
WHILE i <= 4 BEGIN
  LET a(i) = i * i;
  LET i = i + 1;
END

FOR EACH v IN a
  PRINT v; PRINT " ";
NEXT
PRINTLN;
```

### 9.4 Strings and builtins
```basil
PRINT LEFT$("Basil", 2);      ' Ba
PRINT MID$("Basil", 3);       ' sil
PRINT RIGHT$("Basil", 3);     ' sil
PRINT INSTR("banana", "na"); ' 2 (0-based)
PRINT UCASE$("MiXeD");       ' MIXED
PRINT TRIM$("  spaced  ");   ' spaced
PRINT CHR$(65);               ' A
PRINT ASC%("A");              ' 65
```

### 9.5 CGI template fragment
```html
#CGI_NO_HEADER
<?basil
  ' Manual header mode
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
?>
<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Basil CGI Demo</title></head>
<body>
  <h1>Hello, World</h1>
  <ul>
  <?basil
    FOR EACH p$ IN REQUEST$()
      PRINT "<li>" + HTML$(p$) + "</li>\n";
    NEXT
  ?>
  </ul>
</body>
</html>
```


## 10. Operational semantics (summary)
- Evaluation order is left-to-right; function arguments are evaluated before Call. Builtins pop their arguments and push a single result unless specified as statement-like (e.g., INPUT$ used in an expression still returns a value).
- Numeric values are f64 (Num) or i64 (Int). Many operations coerce Int→Num; ToInt truncates.
- String concatenation uses Add if either operand is Str.
- Arrays are row-major, rank ≤ 4; DIM bounds are inclusive.
- FOR EACH enumerates in linear row-major index order over the underlying array buffer.


## 11. Reproducibility and performance
- The VM is intentionally simple and deterministic. Bytecode caching amortizes parse/compile costs; the cache is invalidated by header mismatches (format/ABI/flags) and by changes to source size/mtime.
- The interpreter’s dispatch is a straightforward match over Op with fixed-cost read of immediates. No JIT is present.


## 12. Building and running
- CLI examples:
  - cargo run -q -p basilc -- run examples\fib.basil
  - cargo run -q -p basilc -- lex examples\for.basil
  - To enable object types, build with features (example):
    cargo run -q -p basilc --features obj-bmx-rider -- run examples\objects_arrays.basil


## 13. Notes and future work
- Additional numeric and string builtins are planned (see README/TODO for lists like SIN, COS, TAN, EXP, LOG, HEX, etc.).
- More extensive object library and iterable protocol would enable FOR EACH over objects.
- Additional error recovery in parser and improved static analysis could be layered atop the current architecture without changing the VM.
