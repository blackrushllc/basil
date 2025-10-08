# File I/O and File Management in Basil

This guide documents Basil’s modern, Python-style file I/O based on integer file handles, plus convenient whole-file helpers, file-management statements, and directory listing.

All features here are synchronous and portable across Windows, macOS, and Linux.

- Handles are integers (fh% >= 1). 0 indicates failure and/or raises a runtime error.
- Text modes read/write UTF-8; invalid sequences are replaced when decoding.
- Binary modes read/write raw bytes (coerced to Basil strings when read).
- Files are always flushed on FCLOSE and closed automatically at scope end and program exit.

Quick cheat sheet:

- fh% = FOPEN(path$, mode$)
- FCLOSE fh%
- FFLUSH fh%
- FEOF(fh%) -> BOOL
- FTELL&(fh%) -> LONG
- FSEEK fh%, offset&, whence%  (0=SET, 1=CURRENT, 2=END)
- FREAD$(fh%, n&) -> STRING
- FREADLINE$(fh%) -> STRING
- FWRITE fh%, s$
- FWRITELN fh%, s$
- READFILE$(path$) -> STRING
- WRITEFILE path$, data$
- APPENDFILE path$, data$
- COPY src$, dst$
- MOVE src$, dst$
- RENAME path$, newname$
- DELETE path$
- DIR$(pattern$) -> STRING[]

## 1. Opening and closing files

FOPEN(path$, mode$) -> fh%
- mode$ in: "r", "w", "a", "rb", "wb", "ab", "r+", "w+", "a+", "rb+", "wb+", "ab+".
- Text mode is any mode without the trailing b; otherwise binary.
- Returns an integer handle >= 1 on success. Raises a runtime error on failure (with OS error text).

Example:
```basil
LET fh% = FOPEN("notes.txt", "w");
FWRITELN fh%, "Hello, Basil!";
FCLOSE fh%;
```

FCLOSE fh%
- Safely closes a handle. It is safe to call on an already-closed handle (no-op).

FFLUSH fh%
- Flushes buffered data to disk. Files are also flushed on FCLOSE and at program shutdown.

## 2. Reading and writing

FREAD$(fh%, n&) -> STRING
- Reads up to n& bytes/characters from the current position. Returns fewer if EOF.
- Text mode decodes UTF-8 with replacement on invalid sequences.

FREADLINE$(fh%) -> STRING
- Reads a single line, excluding the trailing newline. Handles CRLF/LF.

FWRITE fh%, s$
- Writes string s$ as-is to the file. Returns TRUE or raises on error.

FWRITELN fh%, s$
- Writes s$ followed by a newline. Returns TRUE or raises on error.

FEOF(fh%) -> BOOL
- Returns TRUE if at end of file (next read would return 0 bytes).

FTELL&(fh%) -> LONG
- Returns current byte offset in file as a LONG integer.

FSEEK fh%, offset&, whence%
- Moves the file position. whence%: 0=SET (absolute), 1=CURRENT (relative), 2=END (relative to end).
- Returns TRUE or raises on error.

Example: reading lines
```basil
LET fh% = FOPEN("notes.txt", "r");
WHILE NOT FEOF(fh%) BEGIN
  LET line$ = FREADLINE$(fh%);
  PRINTLN line$;
END
FCLOSE fh%;
```

Example: seek and tell
```basil
LET fh% = FOPEN("data.bin", "rb");
LET pos& = FTELL&(fh%);
PRINTLN "pos before = ", pos&;
FSEEK fh%, 10, 0;   ' move to absolute 10
PRINTLN "pos now = ", FTELL&(fh%);
FCLOSE fh%;
```

## 3. Whole-file helpers (no explicit open/close)

READFILE$(path$) -> STRING
- Reads an entire file as a UTF-8 string. Raises on error.

WRITEFILE path$, data$
- Overwrites existing file or creates a new one.

APPENDFILE path$, data$
- Appends to existing file or creates a new one.

Example:
```basil
WRITEFILE "out.txt", "Alpha\nBeta\n";
APPENDFILE "out.txt", "Gamma\n";
PRINT READFILE$("out.txt");
```

## 4. File management statements

COPY src$, dst$
- Copies a file. Overwrites destination if allowed by OS; raises on error.

MOVE src$, dst$
- Moves/renames across directories. Raises on error.

RENAME path$, newname$
- Renames within the same directory. For cross-directory moves use MOVE.

DELETE path$
- Deletes a file. Raises on error.

## 5. Directory listing

DIR$(pattern$) -> STRING[]
- Returns an array of file names (no paths) that match pattern$.
- pattern$ may include * and ? wildcards. Default directory is current working directory, or you may include a directory prefix like "examples/*.basil".
- Directories are excluded. The result is sorted ascending. Empty array if no matches.

Example:
```basil
LET files$@ = DIR$("*.basil");
FOR i% = 0 TO UBOUND(files$@)
  PRINTLN files$@(i%);
NEXT
```

## 6. Scope and lifetime

- Handles opened during a function call are local to that call depth and automatically closed when the function returns, unless the handle variable refers to an outer-scope variable.
- Handles in class fields (created by CLASS("file.basil")) persist with the object and are auto-closed when the instance is destroyed or at program end.
- Global handles persist until interpreter shutdown; all remaining open handles are flushed and closed automatically.

## 7. Modes and text vs binary

- Modes ending with "b" are binary; others are text.
- Text decoding/encoding uses UTF-8 with replacement on invalid sequences.
- Binary reads return the raw bytes coerced to a Basil STRING.

## 8. Errors

Most operations raise a descriptive runtime error on failure, such as:
- FileNotFound, PermissionDenied, InvalidHandle, UnexpectedEof, IsDirectory

Errors include the OS error message for clarity, e.g.
```
FOPEN config.ini: No such file or directory
```

## 9. Examples

See the examples folder for complete programs:
- examples/file_io_demo.basil
- examples/whole_file_ops.basil
- examples/dir_glob.basil
- examples/class_file_field.basil
