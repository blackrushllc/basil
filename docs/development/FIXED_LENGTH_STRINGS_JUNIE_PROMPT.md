### Goal
Deliver complete, production‑ready support for:
- Fixed‑length strings with both syntaxes: `DIM A$ AS STRING * N` and `DIM A$[N]`
- `TYPE … END TYPE` structs (also `TYPE Name { … }`), nested structs, arrays of structs
- Dot (`.`) field access and nested access
- Accurate `LEN` semantics (lists/dicts/strings/structs)
- Struct ↔ string conversion for binary I/O (pack/unpack) with packed layout and clear rules
- Keep backward compatibility (arrays on `()`, lists/dicts on `[]`, current programs still run)

Below is a coding plan you can keep and hand back later as an implementation prompt. It is organized by phases with acceptance criteria, detailed design notes, required code touch‑points, opcodes/builtins, and example programs to verify.

---

### Phase 0 — Inventory and constraints (no code)
- Confirm current state (as of 2025‑10‑17):
  - Lists/dicts implemented with `[]`, builtins 251–254, `LEN` extended for list/dict; `FOR EACH` works for list/dict.
  - Parser recognizes `TYPE` and fixed‑string DIM syntaxes but runtime semantics for fixed strings are stubbed.
  - Struct instances are represented as `Value::Dict` and accessed with dot via VM `GETPROP/SETPROP` extended to dict.
  - Arrays of structs are supported by `ArrMake(ElemType::Obj(Some(typeName)))` default‑initializing each elem to an empty dict; parser allows `LET arr(i).Field = …`.
- Constraints:
  - Avoid changing existing bytecode opcodes unless necessary; prefer `Builtin` IDs for new operations.
  - Ensure no ID collisions with feature groups (e.g., `obj-term`, `obj-json`, etc.).
  - Maintain friendly errors with line numbers (the parser already emits `Stmt::Line`).

Acceptance
- Written checklist of used builtin ID ranges; choose a conflict‑free block for new features.
- Document decisions: structs remain `Dict` at runtime but acquire a typed descriptor for packing/unpacking.

---

### Phase 1 — Fixed‑length strings semantics end‑to‑end

#### 1.1 Compiler metadata and lowering
- Track fixed‑length string variables/locals:
  - Extend compiler state with `fixed_globs: HashMap<String, usize>` and `fixed_locals: HashMap<FnName, HashMap<String, usize>>` mapping variable name to `N` bytes.
  - Populate on `Stmt::DimFixedStr { name, len }` for both global and local scopes.
- Lower all assignments to those variables to include a truncate/pad step before storing:
  - In `emit_stmt_toplevel` and `emit_stmt_func`, for `Stmt::Let { name, indices: None, init }` where `name` is fixed‑len:
    - Emit `init`, then `Builtin FIXSTR_ENFORCE (value, N)` → returns coerced string, then store.
  - For array elements of fixed strings (if later added), no change now.
- Lower `LEN(fixedVar$)` at compile time to a numeric constant `N` to avoid runtime metadata plumbing:
  - In compiler `emit_expr_in`, special‑case call `LEN` with a single arg that is `Expr::Var(name)` where `name` is a fixed‑string variable (global or local). Emit `Const Int(N)` instead of builtin `LEN`.
  - Also handle `LEN(rec.Field$)` if the field is a fixed string (see Phase 2.3), by matching `Expr::MemberGet` of a struct instance and looking up the field.

#### 1.2 VM builtin: `FIXSTR_ENFORCE`
- Add a new builtin (choose non‑conflicting ID, e.g., 160) with signature `(value:any, n:int) -> string`:
  - Convert `value` to string via `format!` if not already `Value::Str`.
  - Then enforce length in bytes (UTF‑8 safe):
    - If `s.as_bytes().len() == n` → return as is.
    - If `> n` → truncate at last codepoint boundary ≤ n bytes.
    - If `< n` → pad with ASCII spaces to reach exactly `n` bytes.
- Return `Value::Str` of exact `n` bytes.

#### 1.3 Docs/examples
- Update `docs/guides/NONSCALARS.md` status note to remove caveat once implemented.
- Verify `examples/fixed_length_strings.basil` prints expected results.

Acceptance
- Assigning to `DIM code$ AS STRING * 5` truncates/pads correctly (ASCII and multibyte cases).
- `LEN(code$)` equals declared `N` (via compiler constant folding).
- No runtime regressions for normal strings.

---

### Phase 2 — Structs (TYPE) descriptors, field kinds, and `LEN`

We’ll keep runtime representation as `Value::Dict` but add a compiler+VM side registry of type descriptors for packing/unpacking and for compile‑time smarts like `LEN(TypeName)` and field coercions.

#### 2.1 Type descriptor
- Descriptor struct (in compiler and mirrored into VM via a registration builtin):
  - `name: String`
  - `fields: Vec<FieldDesc>` where `FieldDesc { name: String, kind: FieldKind, offset: usize }`
  - `fixed_size: Option<usize>` where `None` if any variable‑length field exists.
- `FieldKind` matches AST: `Int32`, `Float64`, `FixedString(n)`, `VarString`, `Struct(typeName)`.
- Layout is packed, little‑endian for numeric fields; `FixedString(n)` consumes `n` bytes; `VarString` is variable (no fixed contribution).
- Nested structs compute offsets recursively; `fixed_size` set only if all reachable fields are fixed.

Implementation
- Compiler: When encountering `Stmt::TypeDef { name, fields }`, compute layout and store in compiler’s `struct_types` map with calculated offsets and size. Also emit a registration record into the program so the VM knows descriptors at runtime:
  - Option A (simple): emit a `Builtin STRUCT_REG` call per type at top‑level after parsing all types.
  - The call passes a JSON‑ish info via constants: name, field list `[ (fieldName, kindTag, n?, nestedTypeName?), ... ]`, and `fixed_size`.
  - VM stores in `HashMap<String, VMTypeDesc>` accessible by name.

Choose a builtin ID block, e.g., `161` for `STRUCT_REG`.

#### 2.2 Struct instantiation and dot access (compatibility)
- Keep current behavior: `DIM rec AS TypeName` produces an empty `Dict` with all known field keys present with default values:
  - `Int32` → `0`, `Float64` → `0.0`, `FixedString/VarString` → `""`, `Struct` → empty `Dict`.
- Dot `.` get/set continues using `GETPROP/SETPROP` on dicts.

Enhancement
- For `SETPROP` compiled against a known struct type/field, insert conversions:
  - If field is `Int32` and rhs is numeric float, insert `ToInt` or dedicated numeric conversion.
  - If field is `FixedString(n)`, insert `FIXSTR_ENFORCE` builtin before the set.
  - If field is nested struct, require dict assignment or ignore (users typically set nested fields individually).
- To enable that, when compiling `Stmt::DimObject { name, type_name, .. }`, record `varStructType[name] = type_name` in compiler state (for globals/locals). For arrays, record element type for `name`.
- When lowering `Stmt::SetProp { target, prop, value }`:
  - Try to resolve the `target`’s type:
    - If `target` is a simple var known to be struct `T`, consult `T`’s descriptor.
    - If `target` is `arr(i)` and `arr` is `AS T`, consult `T`.
    - If `target` is `MemberGet` chain where intermediate are known structs, resolve transitively.
  - If resolved, apply field‑kind specific coercion wrappers during codegen.

Acceptance
- Assigning to `rec.Name$` where field is fixed string enforces trunc/pad.
- Assigning `int` to `%` fields truncates to int.
- Arrays of structs continue to work; element default value is a dict with present keys.

#### 2.3 `LEN` semantics for structs
- `LEN(TypeName)` → if `fixed_size` is Some(s), return `s`; else either return `0` or compile‑time error message “Struct contains variable‑length fields; size is not fixed.”
- `LEN(rec)` where `rec` is an instance variable of `TypeName`: same handling as `LEN(TypeName)` (constant if fixed, else 0/runtime error). As a first pass, implement compile‑time folding for `LEN(TypeName)` and `LEN(var)` known to be of that type; fallback to a runtime builtin query for dynamic cases.

Implementation
- Compiler: intercept `LEN` calls:
  - If arg is `Expr::Var(id)` and `id` is a struct type name (not a variable), emit `Const Int(size or 0)`.
  - If arg is `Expr::Var(var)` and `var` is bound to struct `T`: emit `Const Int(size or 0)`.
  - Else, emit normal `LEN` builtin.
- Optional VM builtin `STRUCT_SIZEOF(name$)` (ID e.g., 162) for dynamic cases (e.g., in generic code). The compiler can call it when it cannot fold.

Acceptance
- `PRINT LEN(MyStructType)` prints correct fixed size or 0.
- `PRINT LEN(rec)` prints same.

---

### Phase 3 — Struct ↔ string binary conversion

Goal: Assigning a struct value to a string packs its fixed‑size binary image; assigning a string to a struct unpacks into fields. Only allowed for fixed‑size types.

Rules
- Packed layout uses the descriptor offsets and sizes:
  - `%` (Int32): little‑endian 32‑bit signed.
  - float (Float64): little‑endian 64‑bit IEEE‑754.
  - `STRING * N`: exactly `N` bytes (already enforced on field sets; pack uses bytes as‑is, not trimming trailing spaces).
  - Nested `STRUCT`: inline its bytes according to its descriptor (recursive). All nested must be fixed for the top‑level to be fixed.
  - `STRING` variable‑length: forbids fixed record; packing error: “Struct contains variable‑length fields; size is not fixed.”

Syntax detection at compile time
- If `LET s$ = rec` and `rec` is known struct type `T`:
  - Lower to: push `rec` value; push `"T"`; call `STRUCT_PACK(rec, "T") -> string`; then store into `s$`.
- If `LET rec = s$` and `rec` is struct variable of type `T`:
  - Lower to: push `s$`; push `"T"`; call `STRUCT_UNPACK(str, "T") -> dict`; then assign back to `rec` (via `SetProp` for each field or by replacing the dict value stored in `rec`). Simplest: replace the whole dict state: load current dict, merge values, or reassign variable name to returned dict (we store rec in a global/local slot). In current model, `rec` is stored in a global/local slot, so compile to store the new dict into that slot.

VM builtins
- `STRUCT_PACK(value:any, name$:string) -> string` (ID e.g., 163)
  - Validate `value` is a `Dict` and descriptor exists and is fixed.
  - Produce a `Vec<u8>` with computed `fixed_size`.
  - For each field in order, read from dict by `field.name` (missing → default 0/0.0/empty string), coerce types as needed, write bytes.
  - Return `Value::Str(String::from_utf8_lossy(&bytes).into_owned())` or better, use `String` from raw bytes; since Basil strings are UTF‑8, arbitrary bytes may fail. To support arbitrary binary, we need a byte buffer type or a “binary string” with lossless semantics. If we must reuse `String`, base64 encode is a workaround but undesirable.
  - Recommendation: introduce a `Value::Str` that can store any bytes by using `Vec<u8>` internally and display as `…` when printed; but that changes ABI. Simpler: pack into a `String` by lossy `from_utf8_lossy` for now, or use base64 as explicit helper. If real binary is a requirement, add `Value::Bytes(Vec<u8>)` and extend I/O builtins to accept it. Decide now:
    - Minimal: accept that `FWRITE`/`READFILE$` are text‑oriented; for true binary, follow‑up task introduces `Bytes`. In this plan, keep `String` but note constraint in docs.
- `STRUCT_UNPACK(str:string, name$:string) -> dict` (ID e.g., 164)
  - Validate descriptor and `fixed_size`; check byte length equals `fixed_size` (using `s.as_bytes().len()`); else error: “Unpack: expected N bytes, got M.”
  - Read fields in order, convert to `Value` and put into a new dict.

Notes on binary strings
- If Basil needs exact bytes, strongly consider a separate `BYTES` type in a subsequent milestone. This plan keeps using `String` plus caution in docs. If the project already writes “binary structured data to disk” with `FWRITE`, confirm it writes raw bytes; if yes, then your `String` already holds arbitrary bytes; keep as is.

Acceptance
- For a fixed `TYPE`, `LET s$ = rec` yields a string of expected byte length; `LET rec = s$` round‑trips.
- Attempting to pack a variable‑length struct errors with a clear message.

---

### Phase 4 — Arrays of structs polish + iteration

Already working at a basic level; improve ergonomics and safety.

#### 4.1 Default element initialization
- Current VM `ArrMake` for `ElemType::Obj(Some(_))` already initializes each element to an empty dict. Leave as is.
- Optional: fill with default field keys (like scalar struct init) by calling a helper that builds a dict with defaults `0/0.0/""/empty dict` for each slot. This is heavier; keep current behavior because field assignment code works fine without pre‑filled keys (dict accepts new keys).

#### 4.2 Field setting type safety (compile‑time)
- When compiling `arr(i).Field = value` and `arr` element type is a known struct, apply the same field kind coercions as Phase 2.2.

#### 4.3 Iteration examples and docs
- Ensure examples for `FOR` loops over arrays of structs are present (see verification list).

Acceptance
- `examples/struct_array_of_structs.basil` runs and prints expected lines.

---

### Phase 5 — Error messages and diagnostics
- Parser errors already include line numbers via `Stmt::Line`; keep that behavior.
- Add friendly messages:
  - Packing variable‑length structs: “Struct contains variable‑length fields; size is not fixed.”
  - Fixed string enforce builtin: for wrong `n`: “FIXSTR_ENFORCE expects (string, N).”
  - Unpack size mismatch: include expected and actual sizes.
- `[]` vs `()` misuse already has friendly errors.

Acceptance
- Manual tests show messages with line numbers when run through `basilc`.

---

### Phase 6 — Builtin IDs, collisions, and wiring
- Pick a safe, contiguous block (example allocation; adjust to what’s really free):
  - `160` FIXSTR_ENFORCE(value, n) -> string
  - `161` STRUCT_REG(name$, fieldsSpec$, fixedSize%)  [fieldsSpec$ can be a JSON string; or pass as tuples]
  - `162` STRUCT_SIZEOF(name$) -> int
  - `163` STRUCT_PACK(value, name$) -> string
  - `164` STRUCT_UNPACK(str, name$) -> dict
- Centralize builtin IDs in one place (compiler and VM) to avoid future collisions; add comments for ranges in use.
- Update compiler emission sites; update VM builtin dispatcher.

Acceptance
- Build under feature sets `obj-bmx`, `obj-all` succeeds without unreachable pattern warnings; no ID conflicts.

---

### Phase 7 — Documentation and examples
- Update `docs/guides/NONSCALARS.md`:
  - Remove fixed‑string caveat; add concrete examples with multibyte truncation.
  - Document `LEN(TypeName)` and `LEN(rec)` behaviors.
  - Explain struct packing/unpacking and its restrictions.
- Ensure example set runs:
  - `examples/fixed_length_strings.basil`
  - `examples/struct_basic.basil`
  - `examples/struct_array_of_structs.basil`
  - Mixed container examples already present
- Add an example for struct ↔ string pack/unpack and simple file write/read:
  - `examples/struct_pack_io.basil` (see below).

Example: `examples/struct_pack_io.basil`
```basic
TYPE Header
  DIM Magic$ AS STRING * 4
  DIM Version%
  DIM Count%
END TYPE

DIM h AS Header
h.Magic$ = "BASL"
h.Version% = 1
h.Count% = 42

' Pack to string and write
LET buf$ = h        ' implicit: STRUCT_PACK(h, "Header")
LET f% = FOPEN("hdr.bin", "wb")
FWRITE(f%, buf$)
FFLUSH(f%)
FCLOSE(f%)

' Read back and unpack
LET g% = FOPEN("hdr.bin", "rb")
LET raw$ = FREAD$(g%, LEN(Header))
FCLOSE(g%)
LET h2 AS Header
h2 = raw$           ' implicit: STRUCT_UNPACK(raw$, "Header")
PRINTLN "Magic:", h2.Magic$, " Version:", h2.Version%, " Count:", h2.Count%
```

---

### Phase 8 — Testing and verification (manual + scripted runs)
- Manual runs (Windows):
  - `cargo run -q -p basilc -- run .\examples\fixed_length_strings.basil`
  - `cargo run -q -p basilc -- run .\examples\struct_basic.basil`
  - `cargo run -q -p basilc -- run .\examples\struct_array_of_structs.basil`
  - `cargo run -q -p basilc -- run .\examples\struct_pack_io.basil`
  - `cargo run -q -p basilc --features obj-all -- run .\examples\bigtest3.basil`
- Sanity for lists/dicts examples to ensure no regressions.
- Verify `LEN(TypeName)` and `LEN(rec)` values.

Acceptance
- All example programs above run without parse/compile/runtime errors.
- `LEN` behaviors match spec.
- Struct pack/unpack round‑trips; binary length equals `LEN(Header)` and matches fields.

---

### Detailed code touch‑points

#### Parser/AST
- Already supports:
  - `Stmt::DimFixedStr { name, len }`, `Stmt::TypeDef { name, fields }`
  - `StructFieldKind::{Int32, Float64, VarString, FixedString(n), Struct(name)}`
- Optional small additions (not required for MVP):
  - Support `DIM rec AS MyStructType = { Field: Value, ... }` initializer sugar:
    - Parse as `Stmt::DimObject { name, type_name, args: [] }` followed by synthesized `SetProp` statements for listed fields (in declared order).

#### Compiler
- New state:
  - `fixed_globs`, `fixed_locals`
  - `varStructType_glob: HashMap<String, String>` and `varStructType_local: HashMap<Fn, HashMap<String, String>>`
  - `struct_types: HashMap<String, TypeDescWithLayout>` (enhanced to store offsets/size)
- New/changed lowering:
  - Wrap `LET` to fixed strings with `FIXSTR_ENFORCE`.
  - Coerce `SetProp` based on resolved field kind (int trunc, fixed string enforce).
  - Fold `LEN(fixedVar$)`, `LEN(rec.Field$)` when the field is fixed, and `LEN(TypeName)`/`LEN(var)` for struct sizes.
  - Emit `STRUCT_REG` after all `TYPE` definitions (top‑level only is sufficient).
  - Emit `STRUCT_PACK/UNPACK` for whole‑value assignment to/from strings as per rules.

#### VM
- Builtins:
  - `FIXSTR_ENFORCE` (160): UTF‑8 codepoint‑safe coercion + pad.
  - `STRUCT_REG` (161): install descriptors in VM `HashMap<String, VMTypeDesc>`.
  - `STRUCT_SIZEOF` (162): return `fixed_size` or `0` if not fixed.
  - `STRUCT_PACK` (163), `STRUCT_UNPACK` (164): pack/unpack using descriptor; error if not fixed.
- Existing ops unchanged; `GETPROP/SETPROP` keep dict support.
- `LEN` builtin unchanged for general strings; compile‑time folding covers fixed‑string variables/fields.

---

### Edge cases and decisions
- Encoding for binary strings:
  - Current plan uses `String` with raw bytes. If Windows text I/O corrupts bytes, consider adding a `Bytes` value type later. Mention in docs that binary I/O expects 8‑bit safe paths.
- Variable‑length strings in structs:
  - `fixed_size = None`; `LEN(TypeName)` returns 0; packing/unpacking is disallowed with clear error.
- Nested structs:
  - Allowed as long as the nested graph is acyclic and fixed; offsets calculated recursively.
- Multi‑dimensional arrays of structs:
  - Currently 1D is primary; 2D+ should work since `ArrMake` supports up to rank 4. Keep examples to 1D; QA more if needed.

---

### Migration and compatibility notes
- Existing code continues to work; new features are opt‑in.
- Lists/dicts unchanged; `[]` vs `()` remains disjoint.
- Adding `LEN(TypeName)` is additive.
- Structs remain dicts at runtime; `DESCRIBE` on dict shows keys; no change to class/object system.

---

### Deliverables checklist
- Parser: no major changes (optional initializer sugar)
- Compiler:
  - Fixed string metadata + lowering to `FIXSTR_ENFORCE`
  - Field‑kind coercions on `SetProp`
  - `LEN` folding for fixed strings and struct sizes
  - Emission of `STRUCT_REG`, pack/unpack where required
- VM:
  - Implement builtins 160–164 (or selected non‑conflicting IDs)
  - Maintain descriptor map and pack/unpack logic
- Examples:
  - `examples/fixed_length_strings.basil` (works)
  - `examples/struct_basic.basil` (works)
  - `examples/struct_array_of_structs.basil` (works)
  - `examples/struct_pack_io.basil` (new)
- Docs:
  - Update `docs/guides/NONSCALARS.md` accordingly
- Smoke runs under `--features obj-bmx` and `--features obj-all`

---

### Rough timeline (can be compressed)
- Day 1: Phase 1 (fixed strings) — compiler lowering + VM builtin; update docs; verify example
- Day 2–3: Phase 2 (type descriptors, coercions, LEN folding); run struct examples
- Day 4: Phase 3 (pack/unpack) — VM builtins + compiler detection; create pack/unpack example; test
- Day 5: Phase 4–7 polish; collision audit; docs; final regression through `examples/bigtest3.basil`

---

### Acceptance demo script (quick)
```
DIM code$ AS STRING * 5
code$ = "ABCDEF"
PRINT "LEN(code$)="; LEN(code$); " code$='"; code$; "'\n"

TYPE Person
  DIM Name$ AS STRING * 8
  DIM Age%
END TYPE

DIM p AS Person
p.Name$ = "Alice"
p.Age% = 30
PRINT p.Name$; " "; p.Age%; " (LEN(Person)="; LEN(Person); ")\n"

DIM s$
s$ = p        ' pack
DIM q AS Person
q = s$        ' unpack
PRINT q.Name$; " "; q.Age%

TYPE ScoreRow { DIM Name$ AS STRING * 12 DIM Score }
DIM rows(1) AS ScoreRow
rows(1).Name$ = "Bob"
rows(1).Score = 88.5
PRINT rows(1).Name$; ": "; rows(1).Score
```

Hand this plan back to me later and I’ll execute it step by step, keeping the implementation minimal and compatible with the current codebase. 