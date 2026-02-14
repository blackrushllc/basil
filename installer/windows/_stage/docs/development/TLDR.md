### TL;DR
- The Basil frontend already has most of the infrastructure to support classic “END <keyword>” sentinels. The lexer accepts `ENDIF`, `ENDWHILE`, etc. as `End`, and the parser already tolerates optional suffix words after `END` (e.g., `END IF`).
- We can safely add BEGIN-less, multi-line bodies for `WHILE` and `FOR`/`FOR EACH` without breaking existing code.
- Making BEGIN optional for multi-line `IF … THEN` is trickier due to ambiguity with the already-supported single-statement form. I recommend postponing that part or gating it behind a parser flag.
- Surprise: `FUNC` already supports an implicit body that runs until `END [FUNC]` in the current parser — docs should be updated.

---

### What the code already does today
- Lexer (`basilcore/lexer/src/lib.rs`):
  - Keywords like `ENDIF`, `ENDFUNC`, `ENDWHILE`, `ENDBLOCK` are all tokenized as `TokenKind::End` (lines ~553–561). So we don’t need new tokens to accept classic endings; they’re already mapped to `End`.
- Parser (`basilcore/parser/src/lib.rs`):
  - `consume_optional_end_suffix()` accepts an identifier after `END` and will eat `IF`, `FUNC` (also aliases `FUNCTION`/`SUB`), and `WHILE`, or the identifier `BLOCK` (lines ~1445–1456). That means `END IF` and `END WHILE` are already accepted where `END` is expected.
  - `FUNC` bodies already support three forms (lines ~1376–1409):
    1) `BEGIN … END [FUNC]`
    2) `{ … }`
    3) Implicit body until `END [FUNC]` (no `BEGIN` required). This contradicts the current documentation which claims `BEGIN` is mandatory for function bodies.
  - `IF`: supports `{ … }`, `THEN BEGIN … END [IF]`, and single-statement `THEN`/`ELSE` without `BEGIN`. It does not currently support a multi-line implicit block terminated by `END IF` when `BEGIN` is omitted.
  - `WHILE`: requires `BEGIN … END [WHILE]` or `{ … }`. No implicit mode yet.
  - `FOR` and `FOR EACH`: body can be `BEGIN … END`, `{ … }`, or a single statement; currently the parser expects `NEXT` only after it has parsed one of those three forms.

---

### What we can add safely without major surgery
#### 1) WHILE: allow implicit block until `END [WHILE]`
- Rationale: There is no single-statement `WHILE` form today, so allowing a `BEGIN`-less body that runs until `END [WHILE]` is unambiguous and fully backward compatible.
- Sketch change (in `parse_stmt()` under the `WHILE` arm):
```rust
// WHILE <expr> BEGIN ... END  or  WHILE <expr> { ... }  or  WHILE <expr> ... END [WHILE]
if self.match_k(TokenKind::While) {
    let cond = self.parse_expr_bp(0)?;
    let mut body = Vec::new();
    if self.match_k(TokenKind::Begin) {
        // existing BEGIN/END branch...
    } else if self.match_k(TokenKind::LBrace) {
        // existing { } branch...
    } else {
        // NEW: implicit until END [WHILE]
        loop {
            while self.match_k(TokenKind::Semicolon) {}
            if self.check(TokenKind::End) {
                let _ = self.next();
                self.consume_optional_end_suffix(); // (accept END WHILE)
                break;
            }
            if self.check(TokenKind::Eof) {
                return Err(BasilError(format!(
                    "parse error at line {}: unterminated WHILE body: expected 'END'",
                    self.peek_line()
                )));
            }
            let line = self.peek_line();
            let stmt = self.parse_stmt()?;
            body.push(Stmt::Line(line));
            body.push(stmt);
        }
    }
    return Ok(Stmt::While { cond, body: Box::new(Stmt::Block(body)) });
}
```
- This preserves braces and `BEGIN`, and simply adds the classic implicit form.

#### 2) FOR / FOR EACH: treat “no BEGIN/{}” as implicit block until `NEXT`
- Rationale: We already require a `NEXT` sentinel after the loop body; letting the body consume multiple statements up to `NEXT` is backward compatible and matches classic BASIC expectations.
- FOR EACH change (around lines ~812–855):
```rust
// Body: BEGIN..END, {..}, or implicit until NEXT
let body: Stmt = if self.match_k(TokenKind::Begin) {
    // existing
} else if self.match_k(TokenKind::LBrace) {
    // existing
} else {
    // NEW: implicit until NEXT
    let mut inner = Vec::new();
    loop {
        while self.match_k(TokenKind::Semicolon) {}
        if self.check(TokenKind::Next) { break; }
        if self.check(TokenKind::Eof) {
            return Err(BasilError(format!(
                "parse error at line {}: unterminated FOR EACH body: expected 'NEXT'",
                self.peek_line()
            )));
        }
        let line = self.peek_line();
        let s = self.parse_stmt()?;
        inner.push(Stmt::Line(line));
        inner.push(s);
    }
    Stmt::Block(inner)
};
// Expect NEXT [ident] (as today)
while self.match_k(TokenKind::Semicolon) {}
self.expect(TokenKind::Next)?;
if self.check(TokenKind::Ident) { let _ = self.next(); }
let _ = self.terminate_stmt();
return Ok(Stmt::ForEach { var, enumerable, body: Box::new(body) });
```
- Classic FOR change (around lines ~857–905) is analogous: stop the implicit body when you see `NEXT` and leave `NEXT` consumption to the existing code path just below.

---

### The hard part: BEGIN-less multi-line IF
Why it’s tricky:
- Today, after `IF cond THEN` the parser intentionally supports a single-statement THEN (optionally followed by a single-statement ELSE) even if that statement appears on the next line. Newlines are tokenized as `Semicolon`, and the parser will happily consume them before parsing that one statement.
- If we start reading “statements until `END [IF]`” whenever `BEGIN` is omitted, we will break existing code that relies on single-statement THENs split across lines without an `END`.
- Implementing this “smart” requires either:
  - Backtracking/snapshotting parser state after parsing the first THEN statement (unsafe as-is because `parse_stmt()` mutates state like `with_depth`/`catch_depth`), or
  - A new disambiguating rule that does not change existing behavior by default.

Practical options:
- Option A (safe, opt-in): Add a parser flag, e.g., `implicit_multiline_if: bool`.
  - If true, and we see `IF cond THEN` followed immediately by a newline (one or more `Semicolon` tokens before any statement), parse an implicit block until `ELSE` or `END [IF]`.
  - If false (default), keep current behavior: treat it as a single-statement THEN/ELSE unless you explicitly write `BEGIN` or `{`.
  - This preserves backward compatibility and lets us experiment.
- Option B (defer): Leave IF unchanged for now; implement `WHILE` and `FOR` implicit bodies first; update docs; revisit IF with a small parser refactor that supports safe snapshot/rollback during speculative parses.

Suggested skeleton for Option A (gated) inside the `IF` arm after `self.expect(TokenKind::Then)?`:
```rust
// Allow optional semicolons/newlines after THEN
while self.match_k(TokenKind::Semicolon) {}

if self.match_k(TokenKind::Begin) {
    // existing BEGIN/END path
} else if self.match_k(TokenKind::LBrace) {
    // existing { } path
} else if self.implicit_multiline_if && self.check(TokenKind::Semicolon /* or look for NL */) {
    // NEW (opt-in): implicit THEN-block until ELSE or END [IF]
    let mut then_body = Vec::new();
    loop {
        while self.match_k(TokenKind::Semicolon) {}
        if self.check(TokenKind::Else) || self.check(TokenKind::End) { break; }
        if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated IF body", self.peek_line()))); }
        let line = self.peek_line();
        let stmt = self.parse_stmt()?;
        then_body.push(Stmt::Line(line));
        then_body.push(stmt);
    }
    let then_s = Box::new(Stmt::Block(then_body));
    let else_s = if self.match_k(TokenKind::Else) {
        // For ELSE, mirror the same implicit/BEGIN/{ } logic and require END [IF]
        while self.match_k(TokenKind::Semicolon) {}
        if self.match_k(TokenKind::Begin) { /* ... */ }
        else if self.match_k(TokenKind::LBrace) { /* ... */ }
        else {
            // implicit ELSE-block until END [IF]
            let mut else_body = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.check(TokenKind::End) { break; }
                if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated ELSE body", self.peek_line()))); }
                let line = self.peek_line();
                let stmt = self.parse_stmt()?;
                else_body.push(Stmt::Line(line));
                else_body.push(stmt);
            }
            Some(Box::new(Stmt::Block(else_body)))
        }
    } else { None };
    while self.match_k(TokenKind::Semicolon) {}
    self.expect_end_any()?; // accept END or END IF
    return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
} else {
    // existing single-statement THEN/ELSE path (backward compatible default)
}
```
Notes:
- The “detect newline immediately after THEN” part needs a small hook. Today we immediately consume semicolons; to detect a “blank line then body,” check before consuming or record whether there was at least one `Semicolon` right after `THEN`.
- If that makes the parser too newline-sensitive, prefer Option B (defer IF) for now.

---

### Backward compatibility and risks
- WHILE: no behavior change for existing code that already uses `BEGIN` or `{}`. New implicit form only adds functionality.
- FOR / FOR EACH: existing “single statement” bodies still work; they’re just a degenerate case of “implicit until NEXT”. Nested loops and `NEXT i` vs `NEXT` remain as today.
- IF: leave as-is by default. If you implement the opt-in flag, clearly document that multi-line IF without `BEGIN` requires a closing `END [IF]` and is only active when the flag is enabled.

---

### Documentation updates (recommended)
- Update `docs/guides/BEGIN_END.md`:
  - Correct: `FUNC` allows `BEGIN` or `{}`, or an implicit body terminated by `END [FUNC]`.
  - Add: `WHILE` supports three forms: `BEGIN/END`, `{}`, or implicit until `END [WHILE]`.
  - Add: `FOR`/`FOR EACH` supports three forms for the body: `BEGIN/END`, `{}`, or implicit until `NEXT`.
  - Optional: Document the experimental flag (if you add it) for BEGIN-less multi-line `IF`.

---

### Classic BASIC interpreter parity
- Mirror the same changes in the “classic” BASIC interpreter:
  - Accept `END WHILE` as the terminator for WHILE when `BEGIN` is omitted.
  - Treat FOR/FOR EACH bodies as continuing until `NEXT` by default when `BEGIN`/`{}` is not used.
  - Defer the BEGIN-less multi-line IF change or gate it the same way.

---

### Small design note / TODO to leave in code
Place near the IF parser arm:
```rust
// TODO(junie): BEGIN-less multi-line IF (implicit block until END IF)
// - Ambiguity with existing single-statement THEN/ELSE (even across newlines).
// - Safe plan: add Parser option `implicit_multiline_if` (default false).
//   When enabled, and IF is followed by THEN + immediate newline, parse
//   statements until ELSE or END [IF]. Otherwise, keep single-stmt form.
// - Without a parser snapshot/rollback, avoid speculative parsing because
//   parse_stmt() mutates state (with_depth/catch_depth). Revisit when/if
//   we add a small backtracking facility or restructure statement parsing.
```

---

### Quick examples after these changes
- WHILE without BEGIN:
```basic
WHILE i < 10
    PRINT i
    i = i + 1
END WHILE
```
- FOR without BEGIN (body runs until NEXT):
```basic
FOR j = 5 TO 1 STEP -1
    PRINT j;
    FOR i = 1 TO 5
        PRINTLN i;
    NEXT i;
NEXT j;
```
- IF: unchanged by default; still need `BEGIN … END` or `{ … }` for multi-statement bodies unless you enable the experimental flag.

---

### Bottom line
- Implement implicit blocks now for `WHILE` and `FOR`/`FOR EACH` — straightforward and safe.
- Keep `IF` as-is for now; if desired, add an off-by-default feature flag for BEGIN-less multi-line IF that triggers only when `THEN` is followed by an immediate newline and requires a closing `END [IF]`.
- Update docs and examples; note that `FUNC` already supports implicit bodies ending at `END [FUNC]`.
