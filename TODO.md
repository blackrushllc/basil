Boom! 🌿 We now have:

* functions + calls + returns
* `IF/THEN[/ELSE]` + comparisons
* locals/params + recursion
* working VM frames & globals
* WHILE/END loops
* FOR/NEXT loops
* `INPUT$()` + `PRINT`
* `DIM` + arrays
* `NEW` + Objects
* Boolean ops (`AND/OR/NOT`)
* `FUNC` / `RETURN` / `END`
* `BREAK`
* `CONTINUE`


Maybe later:
* `GOTO` + `ON ERROR GOTO`
* `END` + `EXIT`
* `TYPE` + `END TYPE`
* `WITH` + `END WITH`
* `OPEN` + `CLOSE`


**Pretty errors with spans**

    * Map byte spans → line/column + caret diagnostics.
    * Massive QoL when the language grows.

**Disassembler/trace**

    * `basilc run --trace` to print executed opcodes & stack.
    * Super handy for debugging new control flow.

**String ops**

    * Add `&` (or `+`) for string concat and a `toString` for numbers.

**Standard lib seeds**

    * Built-ins like `clock()`, `len()`, `println()`, maybe `assert()` for tiny tests.

🌱🌱🌱🌱🌱🌱🌱🌱🌱🌱

Next we will make Basil feel like “real” BASIC with a little Python where it helps. 

Here's the plan with syntax, semantics, and implementation notes

(parser/AST → bytecode → VM) plus example programs for testing:

# 1) Loops (FOR/NEXT, DO/WHILE, WHILE/WEND)

## Syntax

```basil
FOR i = 1 TO 10              ' optional STEP
    PRINT i
NEXT                         ' or NEXT i  (both allowed)

FOR x = 10 TO 1 STEP -2
    PRINT x
NEXT x

WHILE n < 100
    n = n * 2
WEND

DO
    input$ = INPUT$()
LOOP UNTIL input$ <> ""
' also allow: DO WHILE cond ... LOOP
```

## Semantics

* `FOR var = start TO end [STEP s]`:

   * Evaluate `start`, `end`, `s` once at loop entry.
   * On each NEXT: `var += s`; continue if `(s >= 0 && var <= end) || (s < 0 && var >= end)`.
   * If `s = 0` → runtime error `FORSTEPZERO`.
* `DO`/`LOOP`:

   * Forms: `DO ... LOOP UNTIL expr`, `DO ... LOOP WHILE expr`, `DO WHILE expr ... LOOP`.

## Compiler/VM

* New opcodes: `ForInit`, `ForCheckJump tgt`, `ForStep`, `WhileCheckJump tgt`, `DoCheckAfter`.
* Store loop frame on VM stack: `(var_slot, end_val, step_val, cmp_dir)`.
* `NEXT` pops/updates topmost FOR frame (optionally check name if `NEXT i` used).

---

# 2) Arrays (DIM/REDIM, multi-dim, PRESERVE)

## Syntax

```basil
DIM a(10)            ' 0..10 inclusive like QB? Recommend: Option Base 0, a(10) = length 11
DIM b(3,4)           ' 2D
REDIM PRESERVE a(20) ' keep contents up to min(old,new)+1
```

**Recommendation (simpler & modern):**

* **Base 0** only (no `OPTION BASE`).
* `DIM a(10)` → length **11** (indices 0..10). If you prefer exact length, use `DIM a[10]`; but keeping QB feel, I’d stick with `()` and 0..N.

## Semantics

* Arrays are reference types on heap; elements are variant values (INT/DOUBLE/STRING/BOOL/STRUCT/NULL).
* `REDIM` can change outermost dimension only (QB-style). `PRESERVE` keeps linearized data.

## VM

* Heap object: header (type: Array, rank, dims[], elem_type:=Any), contiguous storage.
* Opcodes: `ArrMake rank`, `ArrGet`, `ArrSet`, `ArrRedim preserveFlag`.

---

# 3) User Types (TYPE…END TYPE)

## Syntax

```basil
TYPE Person
    name AS STRING
    age  AS INTEGER
    tags AS STRING()       ' dynamic array of strings
END TYPE

DIM p AS Person
p.name = "Ada"
p.age = 47
PRINT p.name, p.age
```

## Semantics

* Nominal records with fixed field layout; fields accessed via `.`.
* Allowed field types: primitives, STRING, arrays, other TYPEs (no cycles for now).
* Default initialization: numeric→0, string→"", arrays→empty, user types→zeroed recursively.

## Compiler/VM

* Parser adds `TypeDecl` to AST; compiler builds a `TypeId` with a field table (name→offset).
* VM object: `Struct { type_id, fields[] }`.
* Opcodes: `StructMake type_id`, `StructGetField id`, `StructSetField id`.

---

# 4) Strings & Classic Functions

## Built-ins (QB-flavored names; `$` suffix optional):

* `LEN(s)`, `LEFT$(s, n)`, `RIGHT$(s, n)`, `MID$(s, start [,len])`
* `INSTR([start,] haystack, needle)`
* `UCASE$(s)`, `LCASE$(s)`
* `LTRIM$(s)`, `RTRIM$(s)`, `TRIM$(s)`
* `CHR$(code)`, `ASC(s)`
* Bonus modern helpers: `REPLACE$(s, find, repl)`, `SPLIT$(s, delim) → STRING()`, `JOIN$(arr, delim)`

## Notes

* All strings are UTF-8; `LEN` returns **character** count, not bytes (store cached length).
* `MID$(s, start, len)` 1-based like QB; for sanity, document clearly.

---

# 5) Python-style File I/O (safe & ergonomic)

You wanted Pythonic IO—let’s add a `WITH OPEN … AS` block and an iterable file handle.

## Syntax

```basil
WITH OPEN("log.txt", mode: "w", encoding: "utf-8") AS f
    f.WRITE_LINE("hello")
    f.WRITE("no newline")
END WITH  ' auto-close

' Reading whole file
WITH OPEN("log.txt", mode: "r") AS f
    text$ = f.READ_ALL()
END WITH

' Iterate lines (like Python)
WITH OPEN("log.txt", mode: "r") AS f
    FOR line$ IN f
        PRINT line$
    NEXT
END WITH
```

### Also provide function-style helpers:

```basil
text$ = READ_FILE("path")            ' returns entire file (UTF-8)
WRITE_FILE("path", text$)            ' overwrite
APPEND_FILE("path", text$)           ' append
```

## Semantics

* `OPEN(path, mode, encoding)` returns a `File` object.

   * `mode` in {"r","w","a","rb","wb","ab"} (binary variants return/accept BYTEARRAY later).
   * `File` methods: `READ_ALL()`, `READ_LINE()`, `WRITE(s)`, `WRITE_LINE(s)`, `CLOSE()`, `EOF()`.
* `WITH … AS` guarantees `CLOSE()` (desugars to try/finally).

## Compiler/VM

* New opcodes or native host calls: `IoOpen`, `IoClose`, `IoReadAll`, `IoReadLine`, `IoWrite`, `IoWriteLine`, `IoIterStart`, `IoIterNext`.
* `WITH` compiles to:

   * acquire → push handle
   * `TryFinally` block
   * `finally` emits `IoClose` if not already closed.
* Add `File` as a native object with vtable method dispatch.

---

# 6) Arrays & Loops Together: `FOR EACH`

Nice quality-of-life:

```basil
FOR EACH name$ IN peopleNames
    PRINT name$
NEXT
```

* Works on arrays and file handles (iterable protocol).
* VM opcodes: `IterStart`, `IterNext`, `IterValue`, `IterEnd`.

---

# 7) Minimal Grammar Additions (EBNF-ish)

```
forStmt      := "FOR" ident "=" expr "TO" expr [ "STEP" expr ] block "NEXT" [ ident ]
whileStmt    := "WHILE" expr block "WEND"
doStmt       := "DO" [ "WHILE" expr ] block "LOOP" [ ("WHILE"|"UNTIL") expr ]
withStmt     := "WITH" "OPEN" "(" arglist ")" "AS" ident block "END" "WITH"
dimStmt      := "DIM" ident "(" dimlist ")" [ "AS" typeName ]
redimStmt    := "REDIM" ["PRESERVE"] ident "(" dimlist ")"
typeDecl     := "TYPE" ident { fieldDecl } "END" "TYPE"
fieldDecl    := ident "AS" typeSpec
typeSpec     := "INTEGER" | "DOUBLE" | "STRING" | ident | ( baseType "(" ")" )  ; array sugar
forEachStmt  := "FOR" "EACH" ident "IN" expr block "NEXT" [ ident ]
```

---

# 8) Error Model (for now)

* Runtime errors with file/line info: `IOERROR`, `FORSTEPZERO`, `INDEXOUTOFRANGE`, `NILACCESS`, `TYPEMISMATCH`.
* (Later) `ON ERROR GOTO` or `TRY/CATCH` can be added; keep v1 simple.

---

# 9) Implementation Order (low-risk path)

1. **Loops** (FOR/NEXT, WHILE/WEND, DO/LOOP) — self-contained, no heap work.
2. **Arrays (1D)** + `LEN`, `ArrGet/Set`, bounds checks, `REDIM`.
3. **Strings & functions** (pure library + a few VM helpers like `SubstrUtf8`).
4. **File I/O** (`File` native object + WITH).
5. **User TYPEs** (parser + struct heap objects + dot access).
6. **FOR EACH** (iterator protocol for arrays & File).

---

# 10) Test Programs (drop-in)

## 10.1 Loops & Strings

```basil
FOR i = 1 TO 5
    PRINT LEFT$("*****", i)
NEXT

n = 1
DO
    n = n * 2
    PRINT n
LOOP UNTIL n >= 32
```

## 10.2 Arrays & FOR EACH

```basil
DIM a(4)
FOR i = 0 TO 4
    a(i) = i*i
NEXT

FOR EACH v IN a
    PRINT v
NEXT

REDIM PRESERVE a(8)
PRINT "len:", LEN(a)       ' should print 9 if base-0 with 0..8
```

## 10.3 File I/O (Python-ish)

```basil
WITH OPEN("notes.txt", mode:"w") AS f
    f.WRITE_LINE("alpha")
    f.WRITE_LINE("beta")
END WITH

WITH OPEN("notes.txt", mode:"r") AS f
    FOR line$ IN f
        PRINT TRIM$(line$)
    NEXT
END WITH

text$ = READ_FILE("notes.txt")
PRINT "All:", text$
```

## 10.4 TYPEs

```basil
TYPE Person
    name AS STRING
    age  AS INTEGER
END TYPE

DIM p AS Person
p.name = "Grace"
p.age = 36
PRINT p.name; " is "; p.age
```

---

# 11) Rust TODOs (quick pointers)

* **lexer**: add keywords: ` WITH, OPEN, AS, TYPE, REDIM, PRESERVE`.
* **parser**: new AST nodes (`DoStmt`, `WithStmt`, `DimStmt`, `RedimStmt`, `TypeDecl`, `MemberExpr`).
* **bytecode**: loop labels & fixups; object model tags (Array, Struct, File).
* **vm**:

   * Value enum gains `Array`, `Struct`, `Native(File)`.
   * Heap arena + GC (you likely already have a simple RC; RC is fine short-term).
   * Native methods table for File & String ops.
* **stdlib**: implement string functions in Rust; expose via intrinsic map.

---

Next Steps:

* draft the exact AST structs and bytecode ops,
* write the **reference tests** (golden outputs) for each feature,
* and sketch the Rust trait for the **Iterable protocol** so arrays and file handles “just work” with `FOR EACH`.

.. To be continued! 🌿🚀
