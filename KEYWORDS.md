# Basil Core Keywords (for Editor Highlighting)

This list contains every keyword recognized by the core Basil interpreter, excluding all add-on feature modules (anything gated by `--features`, such as `obj-term`, `obj-daw`, `obj-json`, etc.).

Generated: 2025-11-08 20:05 (local)

Notes:
- All entries are uppercase as they commonly appear in docs; Basil is case-insensitive for keywords.
- Built-in functions are included (they are part of the core language surface without feature flags).
- Multi-word forms are listed as aliases for convenience.
- Preprocessor-like `#...` directives are excluded because the core lexer treats them as comments.

## Reserved Words and Statements (Core Lexer Keywords)
One keyword per line.

```
AND
AS
AUTHOR
BEGIN
BREAK
CATCH
CASE
CLASS
CONTINUE
DECLARE
DESCRIBE
DIM
DO
EACH
ELSE
ENDBLOCK
ENDFOR
ENDFUNC
ENDFUNCTION
ENDIF
ENDSUB
ENDWHILE
END
EVAL
EXEC
EXPORTENV
EXIT
FALSE
FINALLY
FOR
FOREACH
FUNC
FUNCTION
GOSUB
GOTO
IF
IN
IS
LABEL
LET
MOD
NEW
NEXT
NOT
NULL
OR
PRINT
PRINTLN
RAISE
RETURN
SELECT
SETENV
SHELL
STEP
STOP
SUB
THEN
TO
TRUE
TRY
TYPE
WHILE
WITH
```

## Core Built-in Functions (No feature flags)
One keyword per line.

```
ABS
APPENDFILE
ARRAY_COLS%
ARRAY_ROWS%
ASC%
AT
ATN
CHR$
COPY
COS
DELETE
DIR$
ENV$
ESCAPE$
EXP
FCLOSE
FEOF
FFLUSH
FOPEN
FREAD$
FREADLINE$
FSEEK
FTELL&
FWRITE
FWRITELN
GET$
HTML
HTML$
INKEY$
INKEY%
INPUT
INPUT$
INPUTC$
INSTR
INT
LCASE$
LEFT$
LEN
LOADENV%
LOG
MID$
MKDIRS%
MOVE
POST$
READFILE$
RENAME
REQUEST$
RIGHT$
RND
SIN
SLEEP
SPC
SQR
STRING$
TAB
TAN
TRIM$
TYPE$
UCASE$
UNESCAPE$
URLDECODE$
URLENCODE$
USING$
WRITEFILE
```

## Comment Introducer (recognized by core lexer)

```
REM
```

## Common Multi‑word Forms (aliases)
These are commonly written as two-word forms in code; the core lexer also recognizes the single-word variants shown above.

```
END IF
END FUNC
END FUNCTION
END SUB
END WHILE
SELECT CASE
CASE ELSE
```

If you want this in another format (CSV/JSON/regex), let me know and I can add alternative exports.
