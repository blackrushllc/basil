
# Task: Add Python-style File I/O and File Management to **Basil** (Rust BASIC)

**Context:**
Basil is a BASIC-inspired language implemented in Rust with a bytecode compiler and VM. We need modern, Python-like file I/O using **variables (integer handles)** instead of legacy `#buffers`, plus a few convenience functions and file-management statements. Handle lifetime must respect Basil’s scoping rules (function/class/global), and all files must be safely closed automatically at interpreter shutdown.

## Goals

Implement the following in **lexer, parser, AST, compiler, VM, and host FS layer**, with tests and docs:

### 1) File handles (integer) + open/close

* **Design**

  * `FOPEN(path$, mode$) -> fh%` — returns an **integer file handle** (`fh% >= 1`). `0` indicates failure; raise a Basil runtime error with a descriptive message as well.
  * `FCLOSE fh%` — closes the handle; safe if already closed (no-op).
  * **Modes** (`mode$`): `"r"`, `"w"`, `"a"`, `"rb"`, `"wb"`, `"ab"`, `"r+"`, `"w+"`, `"a+"`, `"rb+"`, `"wb+"`, `"ab+"`.
  * **Auto-close rules**

    * All files **auto-close when interpreter exits** (flush first, then close; avoid 0-byte truncation edge cases).
    * Handles **opened inside a function** are **local** and auto-closed at **function end** *unless* the handle variable was declared in an **outer scope** (then it’s global/outer and persists).
    * Handles opened in a **class method** are local to the **object/class scope** and auto-closed when that scope ends. If the handle variable is declared in the class’ **field section** (outside methods) and is **public**, it is a **public file-handle field** that persists with the object and can be used across methods.
  * **Extra helpers**

    * `FEOF(fh%) -> BOOL`
    * `FTELL&(fh%) -> LONG` (byte offset)
    * `FSEEK fh%, offset&, whence%` (whence: 0=SET, 1=CURRENT, 2=END). Return BOOL or raise on error.

* **Error handling**

  * Graceful runtime errors with OS error text: e.g., `FileNotFound`, `PermissionDenied`, `InvalidHandle`, `UnexpectedEof`, `IsDirectory`, etc.

### 2) Read/write primitives (line, bytes, full strings)

* **Design**

  * `FREAD$(fh%, n&) -> STRING` — read **up to** `n&` bytes/chars (text mode respects encoding; binary is raw bytes coerced to Basil string/byte semantics).
  * `FREADLINE$(fh%) -> STRING` — read one line (without trailing newline).
  * `FWRITE fh%, s$` — write string as-is; return BOOL or raise on error.
  * `FWRITELN fh%, s$` — write string + newline; return BOOL or raise on error.
  * All writes **flush on FCLOSE**; provide `FFLUSH fh%` for explicit flush.

* **Text vs binary**

  * Modes ending with `b` are **binary**; others are **text** (UTF-8 by default). For text, decode/encode UTF-8 with replacement on invalid sequences (document behavior).

### 3) One-shot whole-file helpers (function + statement)

We want **both** a function (to *read* whole file) and a statement (to *write* whole file) that do **not** require explicit `FOPEN/FCLOSE`.

* **Read function**

  * `READFILE$(path$) -> STRING`
    Reads the entire file into a string (text mode, UTF-8). Raises on error.

* **Write statements (two variants)**

  * `WRITEFILE path$, data$` — **overwrite** if exists or **create** if not.
  * `APPENDFILE path$, data$` — **append** if exists or **create** if not.
  * Both perform open/write/flush/close internally and raise on error.

> If you strongly prefer different names, propose alternates (`GETFILE$`, `PUTFILE`, `APPENDFILE`) in the docs, but implement the above as canonical.

### 4) File management commands

Add statements that operate on the filesystem. They should raise runtime errors on failure and return no value:

* `COPY src$, dst$`
* `MOVE src$, dst$`
* `RENAME src$, newname$`  *(same directory semantic; for cross-dir use MOVE)*
* `DELETE path$`  *(files only; consider a `RMDIR` future extension)*

### 5) Directory listing

* `DIR$(pattern$) -> STRING[]`
  Returns an **array of file names** (no paths) that match `pattern$` (supports simple glob: `*`, `?`).

  * Default: current working directory.
  * Exclude directories from results.
  * Return alphabetical ascending.
  * If no matches, return an empty array.

### 6) Scoping & lifetime specifics (compiler/VM)

* Track variable scopes so that handle lifetimes follow Basil’s existing symbol table rules:

  * **Function locals:** auto-close on function return.
  * **Class fields:** persist with the instance; auto-close on object finalization or program end.
  * **Globals:** persist to interpreter exit; auto-close at shutdown.
* Implement **Drop**/finalizer paths in VM or host layer to guarantee **close-on-drop** and **close-at-exit** (flush before close). Ensure panic paths also close.

### 7) Security & portability

* Host FS should go through Basil’s **Host** trait; honor any existing sandboxes (e.g., chroot/project root).
* Normalized paths (`.`/`..`), reject invalid NUL bytes, preserve case sensitivity per platform.
* Unit tests must pass on Windows, macOS, Linux.

### 8) Documentation & examples

Produce/modify the following docs with examples:

* `docs/FILE_IO.md` — Full reference for all new keywords/functions with short examples.
* `examples/file_io_demo.basil` — shows open/readline/write/seek/eof.
* `examples/whole_file_ops.basil` — uses `READFILE$`, `WRITEFILE`, `APPENDFILE`.
* `examples/dir_glob.basil` — uses `DIR$("*.*")`.
* `examples/class_file_field.basil` — demonstrates a **public class file handle field** shared across methods.

### 9) Tests

Add unit and integration tests:

* Opening non-existent file in `r` mode → error.
* Write/append then read back contents (text & binary).
* `FREADLINE$` handling of final line without newline.
* `FSEEK/FTELL` correctness.
* Scope auto-close: local function handle closes on return.
* Interpreter shutdown closes all remaining handles (simulate via test harness).
* `DIR$` glob correctness on multiple platforms.
* File management statements success & failure cases.

---

## Public API Summary (Cheat Sheet)

**Open/Close**

* `fh% = FOPEN(path$, mode$)`
* `FCLOSE fh%`
* `FFLUSH fh%`
* `FEOF(fh%) -> BOOL`
* `FTELL&(fh%) -> LONG`
* `FSEEK fh%, offset&, whence%`  *(0=SET,1=CURRENT,2=END)*

**Read/Write**

* `FREAD$(fh%, n&) -> STRING`
* `FREADLINE$(fh%) -> STRING`
* `FWRITE fh%, s$`
* `FWRITELN fh%, s$`

**Whole-file helpers**

* `READFILE$(path$) -> STRING`
* `WRITEFILE path$, data$`
* `APPENDFILE path$, data$`

**File management**

* `COPY src$, dst$`
* `MOVE src$, dst$`
* `RENAME src$, newname$`
* `DELETE path$`

**Directory listing**

* `DIR$(pattern$) -> STRING[]`  *(names only, no paths)*

**Modes:** `"r" "w" "a" "rb" "wb" "ab" "r+" "w+" "a+" "rb+" "wb+" "ab+"`

---

## Example Snippets (Basil)

**Basic read/write**

```basil
LET fh% = FOPEN("notes.txt", "w");
FWRITELN fh%, "Hello, Basil!";
FCLOSE fh%;

LET fh% = FOPEN("notes.txt", "r");
WHILE NOT FEOF(fh%) BEGIN
  PRINT FREADLINE$(fh%);
END
FCLOSE fh%;
```

**Whole-file helpers**

```basil
WRITEFILE "out.txt", "Alpha\nBeta\nGamma\n";
APPENDFILE "out.txt", "Delta\n";

LET all$ = READFILE$("out.txt");
PRINT all$;
```

**Glob listing**

```basil
LET files$ = DIR$("*.basil");
FOR i% = 0 TO UBOUND(files$)
  PRINT files$(i%);
NEXT
```

**Class-scoped public handle**

```basil
REM CLASS Logger file logger.basil

fh%=0;  // class field, persists

FUNC Init(path$)
  LET fh% = FOPEN(path$, "a");
END

FUNC Log(msg$)
  FWRITELN fh%, msg$;
END

FUNC Close()
  IF fh% <> 0 THEN FCLOSE fh%;
END
```

```basil
REM Use the logger.basil class:
DIM lg@ AS CLASS("logger.basil");
lg@.Init("app.log");
lg@.Log("Started");
' ... later ...
lg@.Close();
```

---

## Implementation Notes

* Extend the Basil grammar with the above identifiers (treat function names and statements per Basil conventions).
* Add new opcodes or host calls for IO; ensure **synchronous, blocking** semantics for now.
* Respect existing array/string types; ensure `DIR$` returns a Basil string array.
* Update bytecode verifier for handle types if needed (runtime checks are fine).
* Provide crisp error messages including path and OS error.

