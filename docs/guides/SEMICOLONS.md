### TL;DR

- You generally do NOT need a semicolon at the end of a line. A newline works as a statement terminator.
- Use `;` or `:` if you want multiple statements on one physical line.
- `:` also doubles as the separator between a `CASE` header and its single-line body, and is treated like a semicolon
  elsewhere.
- Trailing extra semicolons are ignored; the parser actively skips them.
- The last statement in a file does not need a trailing semicolon.

---

### Why this is true (from the lexer/parser)

- The lexer inserts a virtual `Semicolon` token at each newline: it sets `pending_nl_semi = true` when it sees `\n` and
  then emits a `Semicolon` token before the next real token. This means a line break terminates a statement without you
  writing `;`.
- The lexer also tokenizes `:` as `Semicolon` (except for label syntax; see below). So `:` is a general statement
  separator too.
- The parser starts each statement by skipping any number of `Semicolon` tokens:
  `while self.match_k(TokenKind::Semicolon) {}`. So trailing semicolons are optional and harmless.
- Many statements call `terminate_stmt()` (e.g., `LET`, `PRINT`, `PRINTLN`, `DESCRIBE`, `SHELL`, `EXIT`, `RETURN`),
  which accepts the newline-inserted `Semicolon` or an explicit `;`. This is why a simple newline is enough.

---

### When a semicolon (or `:`) IS required

Use `;` or `:` only when you need to place more than one statement on the same physical line.

Examples:

- `LET a = 1; LET b = 2; PRINTLN a + b`
- `PRINT "hi" : PRINT "there" : PRINTLN "!"`

`:` is interchangeable with `;` as a separator (except in label syntax; see below).

---

### When semicolons are NOT required

- End of a simple statement on its own line:
    - `LET x = 42`  (no `;` needed)
    - `PRINTLN "ok"`
- Before and after block keywords: extra semicolons are ignored around `IF/THEN/BEGIN/END`, `SELECT/CASE/END`,
  `FUNC ... END`, `WHILE ... END`, etc. The parser has many `while self.match_k(TokenKind::Semicolon) {}` loops.
- End of file: the last statement does not need a `;`.

---

### Special forms and single-line bodies

- `SELECT CASE` and `CASE` arms:
    - After `SELECT CASE <expr>`, you can either newline or `:` before the first `CASE` (both are treated as
      semicolons).
    - After `CASE <patterns>`, you can use newline or `:` before the arm body:
        - `CASE 1, 2, 3: PRINTLN "small"`
        - or
          ```
          CASE 1, 2, 3
            PRINTLN "small"
          ```
- `IF ... THEN`:
    - The parser supports both single-statement and block forms. After `THEN`, it skips optional semicolons/newlines. So
      you can write:
        - `IF x > 0 THEN PRINTLN "pos"` (newline ends it)
        - `IF x > 0 THEN BEGIN ... END` (block form)
    - `ELSE` can follow on the same line (with `;`/newline separation) or the next line; the parser tolerates semicolons
      around it.
- `FUNC/SUB` definitions:
    - After the parameter list, optional semicolons/newlines are allowed before the body. The body can be a
      `BEGIN ... END` block or an implicit block terminated by `END [FUNC]`.

---

### Labels (colon exception)

- A special case: `IDENT ':'` at the start of a label line is lexed as a `Label` token, not as a `Semicolon`.
    - Example: `Start: PRINTLN "Hi"`
    - You do NOT need a `;` after the label. You may place code on the same line right after the label without a
      separator.
    - Everywhere else, `:` is a statement separator because it is lexed as `Semicolon`.

---

### Practical examples

- Good, with implicit newlines as terminators:
  ```
  LET a = 1
  LET b = 2
  PRINTLN a + b
  ```
- Multiple statements on one line (use `;` or `:`):
  ```
  LET a = 1; LET b = 2; PRINTLN a + b
  PRINT "hello" : PRINTLN " world"
  ```
- `SELECT CASE` single-line arm via `:`:
  ```
  SELECT CASE color$
    CASE "red", "blue": PRINTLN "primary-ish"
    CASE ELSE: PRINTLN "other"
  END SELECT
  ```
- Label on same line as statement:
  ```
  Start: PRINTLN "go"
  GOTO Start
  ```

---

### Edge notes

- Inside parentheses or expressions, you do not use semicolons; they terminate statements, not subexpressions.
- Comments consume to end-of-line; the newline still becomes a virtual semicolon, so you don’t need `;` before a comment
  line break.
- Within `SELECT CASE` parsing, the grammar explicitly allows newlines or `:` between headers and bodies, and ignores
  extra semicolons around `END`/`CASE`.

If you want, I can produce a concise “style guide” snippet you can drop into `docs` explaining these rules with
examples.

---

### See also

- Semicolons and Colons — Quick Style Guide: [./SEMICOLONS_STYLE_GUIDE.md](./SEMICOLONS_STYLE_GUIDE.md)
