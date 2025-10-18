# Non‑Scalar Data in Basil: Lists, Dictionaries, and TYPE Structures

This guide introduces Basil’s non‑scalar data structures:

- Arrays (classic BASIC) — fixed rank, typed, indexed with parentheses `()`
- Lists — dynamic, heterogeneous sequences, indexed with square brackets `[]`
- Dictionaries — string‑keyed associative arrays, indexed with `["key"]`
- TYPE structures (structs) — BASIC’s record types, similar to C’s `struct`

You’ll see both classic BASIC syntax and “Basil#” brace/block forms side‑by‑side. Don’t worry about using one style exclusively — they can be mixed based on taste and context.

> Status note — unsupported features
>
> - Lists and dictionaries are supported, including `[]` indexing, mutation, and `FOR EACH`.
> - The following are NOT supported in this build: `TYPE … END TYPE` structures (including arrays of structs and dot member access), fixed‑length strings (`DIM A$ AS STRING * N` or `A$[N]`), `LEN` over structs or struct type names, and struct↔string pack/unpack for binary I/O.
> - The sections below describe intended design and examples for upcoming releases; some examples may not run in this build.


## Quick cheat‑sheet

- Array literal? Classic BASIC uses `DIM` + element stores. As a convenience, `DIM a% = [1,2,3]` will create a typed integer array and fill it.
- List literal: `[ 10, 20, 30 ]`. Index with `list[1]` (1‑based), mutate with `list[i] = x` or `LET list[i] = x`.
- Dictionary literal: `{ "name": "Ada", "age": 36 }`. Index with `dict["name"]`.
- Arrays vs Lists: `arr(i)` vs `list[i]`. Arrays are fixed‑rank and typed; Lists are dynamic and may mix types.
- TYPE (struct):
  ```
  TYPE Person
    DIM Name$ AS STRING * 16
    DIM Age%
  END TYPE
  DIM p AS Person
  p.Name$ = "Ada" : p.Age% = 36
  ```
- Arrays of structs: `DIM people(2) AS Person` then `people(1).Name$ = ...`.
- Braces `{}`: In expression position → dictionary literal; after a control header (like `IF ... {`) → block.

---

## Arrays vs Lists

Basil supports classic BASIC arrays and new dynamic lists. They solve different problems.

### Arrays (classic BASIC)

- Declared with `DIM` and parentheses.
- Have a fixed rank (1–4D supported) and a concrete element type:
  - `%` suffix → 32‑bit integer elements
  - `$` suffix → string elements
  - otherwise → float elements
- Index with parentheses: `arr(i)` (and `arr(i,j)` for 2D, etc.)
- Upper‑bound semantics: `DIM A%(2)` creates indices `0..2` (three elements). Classic BASIC style.

Example (classic):

```basic
DIM scores%(4)          ' indices 0..4
LET scores%(0) = 10
LET scores%(1) = 20
PRINT "scores(1) = ", scores%(1)
PRINT "count = ", LEN(scores%())   ' total elements (5)
```

### Lists (dynamic sequences)

- Created with square‑bracket literals in expression position: `[ a, b, c ]` with optional trailing comma.
- May mix types: numbers, strings, nested lists/dicts, etc.
- Index with square brackets: `list[1]` (1‑based indexing by design).
- Mutate with `list[i] = value` or `LET list[i] = value`.

Example (Basil# style):

```basic
LET mixed@ = [1, "two", 3.0]
PRINT "second = " + mixed@[2]
mixed@[2] = "TWO"
PRINT "second now = " + mixed@[2]
PRINT "len(mixed) = " + LEN(mixed@)
```

### Bridging: list literals to typed arrays

If you write `DIM a% = [1,2,3]`, Basil recognizes a homogeneous integer element list and creates an actual integer array `a%()` sized to the list, then assigns elements. You can then use classic `()` array indexing:

```basic
DIM a% = [1,2,3]
PRINT a%(0); ", "; a%(1); ", "; a%(2)
```

> Note on indices
>
> - Arrays are 0‑based in this build (classic BASIC upper‑bound semantics: `DIM A%(N)` → indices `0..N`).
> - Lists are 1‑based by design: `list[1]` returns the first element.

When to prefer which?

- Use Arrays when you need fixed shape, numeric performance, or multi‑dimensional data.
- Use Lists when you want flexible, growable sequences, heterogeneous data, or when building/ad‑hoc collections.


## Dictionaries (Associative Arrays)

Dictionaries map string keys to values. They’re similar to “associative arrays” (Perl), JavaScript objects, or Python dicts.

- Literal syntax (expression position only): `{ "key": value, ... }`
- Index with `dict["key"]`
- Mutate with `dict["key"] = value` or `LET dict["key"] = value`
- `LEN(dict)` returns the number of keys

Example (classic + Basil# mix):

```basic
DIM pet@ = { "name": "Fido", "species": "dog", "age": 7 }
PRINT pet["name"]
LET pet["age"] = pet["age"] + 1
PRINT "age = ", pet["age"]
PRINT "keys = ", LEN(pet)
```

FOR EACH iterates dictionary KEYS (strings) by default:

```basic
FOR EACH key$ IN pet {
  PRINT key$ + ": " + pet[key$]
}
NEXT
```

> Tip: Keys must be quoted strings in literals for now. Nested containers work fine:
>
> ```basic
> LET user@ = {
>   "name": { "first": "Ada", "last": "Lovelace" },
>   "skills": ["math", "poetry"]
> }
> PRINT user@["name"]["last"]      ' → Lovelace
> PRINT user@["skills"][2]            ' 1-based list index → poetry
> ```


## Control‑flow braces `{}` vs dictionary literals `{}`

Basil uses `{ ... }` for both blocks and dictionary literals. Disambiguation is by context:

- After a control header (`IF`, `ELSE`, `WHILE`, `FOR`, `FOR EACH`, `SELECT CASE`, etc.), `{ ... }` starts a block.
- Where an expression is expected, `{ ... }` is parsed as a dictionary literal.

Examples:

```basic
IF user_is_admin {
  PRINTLN "Welcome, admin!"
} ELSE {
  PRINTLN "Access limited."
}

LET cfg@ = { "host": "localhost", "port": 8080 }   ' dict literal in expression position
```


## TYPE Structures (records)
> Not supported yet in this build. The syntax and examples below describe planned behavior and may not run.

BASIC’s `TYPE … END TYPE` defines a record (similar to C’s `struct`). Basil supports two body styles: classic and brace‑body. Fields are declared with `DIM` inside the type.

### Defining a TYPE

Classic form:

```basic
TYPE Person
  DIM Name$ AS STRING * 16
  DIM Age%
END TYPE
```

Brace‑body form (Basil# style):

```basic
TYPE Person {
  DIM Name$ AS STRING * 16
  DIM Age%
}
```

Field types:

- `%` suffix → 32‑bit integer (e.g., `Age%`)
- default without suffix → float (e.g., `Score`)
- `$` suffix → string; can be variable‑length or fixed‐length with `AS STRING * N` or bracket form `Id$[N]`
- `AS TYPE OtherType` or `AS OtherType` for nested structs

### Using a struct

```basic
DIM p AS Person
p.Name$ = "Ada"
p.Age% = 36
PRINT "Person: " + p.Name$ + " (Age " + p.Age% + ")"
```

Nested struct access chains with `.` as expected: `rec.Child.Label$`.

> Implementation note
>
> In this build, struct instances are represented as dictionaries under the hood, enabling `p.Field` reads/writes. This is transparent to you; just use dot syntax.

### Arrays of structures
> Not supported yet in this build.

Arrays of structs are powerful for table‑like data and binary I/O layouts.

Example:

```basic
TYPE Record
  DIM Name$ AS STRING * 12
  DIM Age%
  DIM Score           ' float
END TYPE

DIM recs(2) AS Record

recs(1).Name$ = "Alice"
recs(1).Age% = 30
recs(1).Score = 95.5

recs(2).Name$ = "Bob"
recs(2).Age% = 25
recs(2).Score = 88.0

FOR i% = 1 TO 2 {
  PRINTLN recs(i%).Name$ + " age=" + recs(i%).Age% + " score=" + recs(i%).Score
}
NEXT
```

> Why arrays of structures?
>
> - Lay out highly structured records (e.g., fixed‑width file rows, device registers, network packet headers).
> - Keep related fields together, improving locality and clarity.
> - Iterate simply with `FOR` or `FOR EACH` when supported.

### Fixed‑length strings in structs

Both forms are accepted in field declarations and standalone DIMs:

```basic
DIM code$ AS STRING * 5
DIM tag$[3]
```

Semantics target (as releases progress):

- On assignment, truncate at UTF‑8 codepoint boundary to ≤ N bytes, then pad with spaces to N bytes.
- `LEN(fixed$)` returns the declared byte length N, not displayed character count.

In this build, fixed strings are stored as regular strings; the above semantics are being rolled out.


## Iteration patterns: FOR and FOR EACH

### FOR with Lists and Dictionaries

- Lists: index from `1` to `LEN(list)` and use `[]`
- Dictionaries: iterate a precomputed keys list

```basic
LET items@ = [ "red", "green", "blue" ]
FOR i% = 1 TO LEN(items@) {
  PRINT "items[" + i% + "] = " + items@[i%]
}
NEXT

LET person@ = { "first": "Ada", "last": "Lovelace", "born": 1815 }
LET keys@ = [ "first", "last", "born" ]
FOR i% = 1 TO LEN(keys@) {
  LET k$ = keys@[i%]
  PRINT k$ + ": " + person@[k$]
}
NEXT
```

### FOR EACH with Lists and Dictionaries

- Lists: yields each element
- Dictionaries: yields keys (strings)

```basic
LET numbers@ = [10, 20, 30, 40]
FOR EACH n IN numbers@ {
  PRINT "n = " + n
}
NEXT

LET pet@ = { "name": "Fido", "species": "dog", "age": 7 }
FOR EACH key$ IN pet {
  PRINT key$ + ": " + pet@[key$]
}
NEXT
```

### Iterating Arrays of Structures

```basic
TYPE Person
  DIM Name$ AS STRING * 16
  DIM Age%
END TYPE

DIM people(2) AS Person
people(1).Name$ = "Alice": people(1).Age% = 30
people(2).Name$ = "Bob"  : people(2).Age% = 25

FOR i% = 1 TO 2 {
  PRINT people(i%).Name$ + ": " + people(i%).Age%
}
NEXT
```


## Common tasks and patterns

### Mixing lists and dictionaries

```basic
LET pets@ = [
  { "name": "Fido",  "species": "dog" },
  { "name": "Misty", "species": "cat" }
]
PRINT pets@[1]["name"] + " & " + pets@[2]["species"]
```

### Building typed arrays from list literals

```basic
DIM xs% = [1,2,3,4,5]
PRINT "total elements in xs%() = ", LEN(xs%())
```

### Nested structures

```basic
TYPE ChildType
  DIM Label$ AS STRING * 8
END TYPE

TYPE ParentType
  DIM Child AS TYPE ChildType
  DIM Notes$
END TYPE

DIM p AS ParentType
p.Child.Label$ = "X1"
PRINT p.Child.Label$
```


## Errors and diagnostics you may see

- `Attempted [] on a non-list/dict value.` — You used `[]` indexing on something that’s not a list or dictionary.
- `Dictionary missing key: "name"` — Reading a key that isn’t present.
- `List index out of range: X` — Out‑of‑bounds 1‑based access on a list.
- `GETPROP on non-object/dict (got TYPE=...)` — Dot member access on a value that isn’t an object/dict/struct.

Include line numbers in error messages by running through basilc normally; many constructs insert `LINE` markers to improve reporting.


## Performance and design notes

- Arrays are best for numeric, fixed‑shape data and multi‑dimensional matrices. They’re typed and compact.
- Lists are flexible and ideal as dynamic, growable collections — they can hold any `Value` (including arrays, dicts, lists, structs-as-dicts).
- Dictionaries give ergonomic field‑by‑name access and interop well with JSON‑like data.
- Structs provide named‑field discipline atop dictionaries with compile‑time shape info; arrays of structs combine structure with indexed storage.


## See also

- docs/educational/BASIL_SHARP.md — Basil# syntax overview
- docs/guides/BASIL_OK_PROMPT.md — Tips for working within the prompt/REPL

If you spot mismatches between this guide and your build, check WHATS_NEW.md and release notes — non‑scalar features are being actively expanded.
