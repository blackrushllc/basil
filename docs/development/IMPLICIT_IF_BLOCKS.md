You are working in a Rust-based BASIC interpreter project.
There are **two sibling projects** that are almost identical:

* **Basil** (full-featured): interpreter + compiler + web server.
* **Basic** (stripped-down “naked” BASIC): interpreter only.

This prompt is designed to work in either repo.
When I run it in one repo, treat *that* repo as the active project and ignore the other.

You are working on our BASIC dialect and its Rust-based cousin Basil.

### Design: BEGIN-less IF..THEN..ELSE..END IF blocks (implicit IF blocks)

Status: This document describes the minimal, low-risk parser changes to add multi-line IF blocks without requiring BEGIN..END, while preserving all existing forms and the one-line IF/ELSE.

Goals
- Support classic BASIC style multi-line IF blocks terminated by END [IF] without requiring BEGIN..END, e.g.:
``` 
  IF cond THEN
      PRINT "ok"
  ELSE
      PRINT "not ok"
  END IF
```  
- Keep backward compatibility: existing { … }, THEN BEGIN … END, and one-line IF forms continue to work.
- Keep the language newline model unchanged: the lexer already normalizes both newlines and ':' into Semicolon tokens; we will use those for disambiguation.

Non-goals
- Do not make the parser broadly newline-sensitive beyond recognizing Semicolon tokens that already exist.
- Do not alter runtime semantics or the AST shape for IF; only parsing is extended.

Background (what exists today)
- Lexer maps END-suffixed keywords like ENDIF, END IF, END WHILE, END FUNC, END SUB etc., to TokenKind::End. No new tokens are needed.
- Parser has consume_optional_end_suffix() which tolerates optional suffix identifiers after END (e.g., END IF). We re-use it.
- IF currently supports:
    1) Brace form: IF cond { … } [ELSE { … }]
    2) Classic braced form: IF cond THEN BEGIN … [ELSE BEGIN … END] END [IF]
    3) Single-statement: IF cond THEN <stmt> [ELSE <stmt>]
- WHILE and FOR/FOR EACH already support implicit multi-line bodies via newline/colon, ended by END [WHILE] and NEXT respectively.

Core rule (disambiguation)
- After parsing the IF header (IF <expr> THEN), choose the body form by the next token(s):
    1) If the next token is Begin → parse BEGIN … END (existing).
    2) Else if the next token is LBrace → parse { … } (existing).
    3) Else if the next token is Semicolon (i.e., the header ends with a newline or ':') → parse an implicit multi-statement THEN block that continues until ELSE or END.
    4) Otherwise → parse the existing single-statement THEN body.

ELSE handling (symmetric to THEN)
- When ELSE appears after a THEN block:
    - If immediately followed by IF → parse as ELSE IF … using the existing IF arm (chains work unchanged).
    - Else if followed by Begin → parse BEGIN … END as the ELSE body (existing).
    - Else if followed by LBrace → parse { … } as the ELSE body (existing).
    - Else if ELSE is terminated by Semicolon (newline/colon right after ELSE) → parse an implicit multi-statement ELSE block until END [IF].
    - Otherwise → parse a single-statement ELSE; then require END [IF] immediately afterwards (as we already do today after a single-statement ELSE in the BEGIN form).

Parser touchpoints (basilcore/parser/src/lib.rs)
1) In parse_stmt(), inside the IF arm (already located around the existing IF parsing code):
    - After self.expect(TokenKind::Then)?, check whether the next token is a Semicolon without consuming other forms first. Use a local boolean had_terminator = self.check(TokenKind::Semicolon). If true, consume all consecutive Semicolons with while self.match_k(TokenKind::Semicolon) {}.
    - Precedence of body forms: Begin → LBrace → (if had_terminator) implicit THEN block → single-statement.
    - For the implicit THEN block, collect statements until either:
        - self.check(TokenKind::Else) → stop the THEN body and proceed to parse ELSE branch, or
        - self.check(TokenKind::End) → stop; consume END and optional IF suffix via consume_optional_end_suffix(); there is no ELSE.
        - If EOF is reached first, raise an error: "unterminated IF body (expected END)" with the current line.
    - For the ELSE branch in the implicit mode, mirror the same precedence rule as above using an else_had_term flag that is set when ELSE is immediately followed by Semicolon. For a single-statement ELSE, after parsing that one statement, require END [IF] (via expect_end_any()).

2) Keep the existing brace and BEGIN forms unchanged, including their error messages.

3) Re-use existing helpers:
    - expect_end_any() which expects an END and tolerates optional suffix via consume_optional_end_suffix().
    - parse_stmt() for nested structures; nested implicit IF/WHILE/FOR will work naturally.

Pseudo-structure (illustrative, not exact code)
```
  // After parsing IF <expr>
  self.expect(TokenKind::Then)?;
  let mut had_terminator = false;
  if self.check(TokenKind::Semicolon) {
      had_terminator = true;
      while self.match_k(TokenKind::Semicolon) {}
  }
  if self.match_k(TokenKind::Begin) { /* existing THEN BEGIN … END path */ }
  else if self.match_k(TokenKind::LBrace) { /* existing THEN { … } path */ }
  else if had_terminator {
      // Implicit THEN block until ELSE or END
      let mut then_body = Vec::new();
      loop {
          while self.match_k(TokenKind::Semicolon) {}
          if self.check(TokenKind::Else) || self.check(TokenKind::End) { break; }
          if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated IF body (expected END)", self.peek_line()))); }
          let line = self.peek_line();
          then_body.push(Stmt::Line(line));
          then_body.push(self.parse_stmt()?);
      }
      let then_s = Box::new(Stmt::Block(then_body));
      let else_s = if self.match_k(TokenKind::Else) {
          // ELSE branch precedence
          let mut else_had_term = false;
          if self.check(TokenKind::Semicolon) { else_had_term = true; while self.match_k(TokenKind::Semicolon) {} }
          if self.check(TokenKind::If) { Some(Box::new(self.parse_stmt()?)) } // ELSE IF …
          else if self.match_k(TokenKind::Begin) { /* collect BEGIN…END into Block */ }
          else if self.match_k(TokenKind::LBrace) { /* collect {…} into Block */ }
          else if else_had_term {
              // Implicit ELSE block until END
              let mut else_body = Vec::new();
              loop {
                  while self.match_k(TokenKind::Semicolon) {}
                  if self.check(TokenKind::End) { break; }
                  if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated ELSE body (expected END)", self.peek_line()))); }
                  let line = self.peek_line();
                  else_body.push(Stmt::Line(line));
                  else_body.push(self.parse_stmt()?);
              }
              Some(Box::new(Stmt::Block(else_body)))
          } else {
              // Single-statement ELSE
              let s = self.parse_stmt()?;
              while self.match_k(TokenKind::Semicolon) {}
              self.expect_end_any()?;
              Some(Box::new(s))
          }
      } else {
          while self.match_k(TokenKind::Semicolon) {}
          self.expect_end_any()?;
          None
      };
      return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
  } else {
      // Single-statement THEN (existing)
  }
```
Error handling
- Unterminated implicit THEN body: "parse error at line <n>: unterminated IF body (expected END)".
- Unterminated implicit ELSE body: "parse error at line <n>: unterminated ELSE body (expected END)".
- Existing errors for BEGIN and { forms remain unchanged.

Backward compatibility and precedence
- Explicit block markers take precedence: THEN BEGIN and { } are parsed first.
- Single-statement THEN/ELSE remains the fallback when there is no newline/colon (Semicolon) immediately after THEN or ELSE.
- Newline or ':' immediately after THEN/ELSE signals a multi-statement implicit block.
- Colons work like newlines: IF cond THEN: PRINT A: PRINT B: END IF is accepted. Likewise for ELSE: ELSE: PRINT A: END IF.
- ELSE IF chains are supported transparently by delegating to parse_stmt() after ELSE IF.

Testing plan (matrix)
- Happy paths:
    - Minimal implicit IF without ELSE.
    - Implicit IF with implicit ELSE.
    - Implicit IF with BEGIN ELSE and vice versa (mix-and-match).
    - ELSE IF chains with implicit blocks.
    - Variants using ':' instead of newlines.
- Nesting:
    - Nested implicit IF inside implicit IF.
    - Mixed nesting with WHILE/FOR (which already support implicit bodies).
- Single-line preservation:
    - IF cond THEN PRINT "x" ELSE PRINT "y" still parses as one-liners.
- Error cases:
    - Missing END IF (EOF encountered).
    - ELSE without terminating END IF in implicit path.
    - Ensure BEGIN/{} error messages unchanged.

Classic interpreter parity
- Mirror the same rule in the classic BASIC interpreter: newline/colon after THEN or ELSE implies a multi-line block terminated by END IF. Keep single-line and BEGIN/{} forms.

Examples
1) BEGIN-less IF with ELSE:
```
  IF targetDir$ = "" THEN
      PRINT "No target directory specified. Aborting."
      RETURN
  ELSE
      PRINT "ok"
  END IF
```
2) Colons instead of newlines:
``` 
  IF ok THEN: PRINT "good": PRINT "still good": ELSE: PRINT "bad": END IF
```
3) Single-line (unchanged):
```
IF ok THEN PRINT "one-liner" ELSE PRINT "alt"
```

4) Mixed nesting:
```
   IF outer THEN
      FOR i = 1 TO 3
          IF i = 2 THEN
              PRINT "two"
          END IF
      NEXT i
  END IF
```
Implementation effort
- Changes are localized to the IF arm within parse_stmt(). The code follows the same pattern already used for implicit WHILE and FOR blocks, reducing risk. No changes to the AST or compiler are required.
