# Semicolons and Colons — Quick Style Guide

Use this as a concise reference for statement separators in Basil.

- Default terminator: a newline ends a statement. You usually do not need a trailing semicolon.
- Multiple statements on one line: use either `;` or `:` between statements.
- `:` as special separator in SELECT CASE: it may separate a `CASE` header from its single-line body (e.g., `CASE 1,2: PRINTLN "x"`). Elsewhere `:` is equivalent to `;`.
- Labels exception: at the start of a line, `Name:` defines a label. In that position, the `:` is not a separator and you do not need an extra `;`.
- Trailing semicolons are ignored. The last statement in a file does not require a semicolon.
- Around block keywords (`IF/THEN/ELSE/END`, `SELECT/CASE/END`, `FUNC … END`, `WHILE … END`), extra semicolons/newlines are tolerated and ignored by the parser.

Examples

- One statement per line (no semicolons needed):
  
  LET a = 1
  PRINTLN a

- Multiple statements on one line:
  
  LET a = 1; LET b = 2: PRINTLN a + b

- SELECT CASE single-line arms via `:`:
  
  SELECT CASE color$
    CASE "red", "blue": PRINTLN "primary-ish"
    CASE ELSE: PRINTLN "other"
  END SELECT

- Label on the same line as a statement:
  
  Start: PRINTLN "go"
  GOTO Start
