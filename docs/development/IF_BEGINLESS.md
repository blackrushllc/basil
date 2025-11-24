Implementing BEGIN-less multi-line IF … THEN … [ELSE …] END IF while preserving single-statement IF

Status: proposal/design for review — no behavior change by default

Context
* Current Basil behavior (see basilcore/parser/src/lib.rs):
  * Single-statement IF is supported: IF cond THEN <stmt> [ELSE <stmt>].
    * Important: the single statement may be on the next line because newlines are tokenized as Semicolon and are skipped before parsing that one statement.
  * Multi-statement THEN/ELSE bodies are supported with either:
    * IF cond { ... } [ELSE { ... }]
    * IF cond THEN BEGIN ... [ELSE BEGIN ... END] END [IF]
  * The lexer already maps ENDIF/ENDWHILE/ENDFUNC/ENDBLOCK to TokenKind::End, and the parser accepts an optional suffix token after END (IF/FUNC/WHILE or the ident BLOCK).
  * WHILE and FOR/FOR EACH already support implicit bodies: WHILE runs until END [WHILE], FOR/FOR EACH run until NEXT.

Goal
* Add an optional BEGIN-less, multi-line IF form that reads a sequence of statements after THEN up to ELSE or END [IF] — while fully preserving current single-statement IF behavior.

Examples (intended when enabled)
1) Multi-line THEN, no ELSE:
   IF ok THEN
       PRINT "yay"
       x = 1
   END IF

2) Multi-line THEN with multi-line ELSE:
   IF ok THEN
       PRINT "then"
   ELSE
       PRINT "else"
       x = x + 1
   END IF

3) Mixed ELSE forms remain valid (explicit block or single statement):
   IF ok THEN
       PRINT "then"
   ELSE PRINT "else-on-one-line"
   END IF

4) Else-if continues to work via existing nested single-statement rule:
   IF a THEN
       PRINT "a"
   ELSE IF b THEN PRINT "b" ELSE PRINT "c"
   END IF

Key constraint and the source of ambiguity
* Today, after IF cond THEN, the parser happily consumes one or more Semicolon tokens (newlines/colons) and then parses a single statement. This means both of these currently parse as single-statement THEN:
  IF c THEN PRINT 1
  IF c THEN
      PRINT 1
* If we naively switch to “read many statements until END IF” whenever we don’t see BEGIN or {, we would break existing code that relies on the second form (THEN newline single statement) because it would now require a closing END and would greedily include subsequent statements.

Design choice: gated feature with conservative disambiguation
* Introduce a parser option flag: implicit_multiline_if: bool.
  * Default: false (no behavior change).
  * When true: After THEN, if we see at least one Semicolon immediately (i.e., an immediate newline/colon separation) and do not see BEGIN or {, we treat the body as an implicit multi-statement block that continues until ELSE or END [IF].
  * Rationale: This is the smallest change that enables the desired form without introducing speculative backtracking or breaking existing programs by default.

Grammar delta (conceptual, when the option is enabled)
* IfStmt := "IF" Expr ( BraceThen | ThenBlock | SingleThen )
* BraceThen := "{" Stmt* "}" [ElsePart]
* ThenBlock := "THEN" nl_or_colon+ ImplicitBody [ElseImplicit] EndIf
* SingleThen := "THEN" nl_or_colon* Stmt [ElseSingle]
* ElseImplicit := "ELSE" nl_or_colon* ( BraceBlock | BeginBlock | ImplicitBody )
* ElseSingle := "ELSE" nl_or_colon* ( BraceBlock | BeginBlock | Stmt ) [EndIf]
* ImplicitBody := Stmt* until ("ELSE" | "END" ["IF"]) (do not consume the terminator here)
* EndIf := "END" ["IF"]

Lexer/AST impact
* Lexer: No changes needed. ENDIF is already tokenized as End; END IF is already accepted via consume_optional_end_suffix().
* AST: No changes. We continue to represent THEN/ELSE bodies as Stmt (often Stmt::Block).

Parser changes (targeted)
1) Add an options struct and constructor (non-breaking):
   * Define a minimal options type and default:
     struct ParserOptions { implicit_multiline_if: bool }
     impl Default for ParserOptions { /* false */ }
   * Extend Parser to store options: struct Parser { ..., opts: ParserOptions }.
   * Add a new entry point parse_with_options(src: &str, opts: ParserOptions) -> Result<Program> that mirrors parse(src) but passes options. Keep parse(src) using Default::default() to preserve current semantics.

2) IF arm disambiguation and new branches (approximate location: basilcore/parser/src/lib.rs, in parse_stmt, around the IF arm):
   * After expect(Then), do not unconditionally consume all Semicolons. Instead, count how many you consume:
     let mut sep_count = 0; while self.match_k(TokenKind::Semicolon) { sep_count += 1; }
   * Branch order and behavior:
     a) If next is LBrace → use existing brace-body branch.
     b) Else if next is Begin → use existing BEGIN…END branch.
     c) Else if opts.implicit_multiline_if && sep_count > 0 → parse implicit THEN body:
        * Collect statements until you see Else or End.
        * Do not consume End/Else here; stop before it.
        * then_branch = Stmt::Block(collected).
        * If ELSE follows:
          • Consume optional Semicolons.
          • If LBrace or Begin, use existing branches.
          • Else if self.check(If), treat as a single-statement ELSE (nested IF as one statement), then require END [IF].
          • Else, parse an implicit ELSE body until END [IF].
        * Finally, require END (accept optional IF) and return Stmt::If.
     d) Else (fallback) → existing single-statement THEN and optional single-statement ELSE path.

3) Helper reuse
* Reuse expect_end_any() to accept either END or END IF.
* Continue to use consume_optional_end_suffix() after consuming END inside multi-branch loops where appropriate.

Pseudocode sketch for the new part (inside IF arm)
  self.expect(Then)?
  let mut sep = 0; while match(Semicolon) { sep += 1; }
  if match(LBrace) { /* existing */ }
  else if match(Begin) { /* existing */ }
  else if self.opts.implicit_multiline_if && sep > 0 {
      // THEN implicit body until ELSE or END
      let then_vec = collect_stmts_until(|p| p.check(Else) || p.check(End))?;
      let then_s = Box::new(Stmt::Block(then_vec));
      let else_s = if match(Else) {
          while match(Semicolon) {}
          if match(LBrace) { /* existing */ }
          else if match(Begin) { /* existing */ }
          else if self.check(If) {
              // ELSE IF … → single-statement ELSE (nested IF as one statement)
              let s = self.parse_stmt()?;
              while match(Semicolon) {}
              self.expect_end_any()?;
              Some(Box::new(s))
          } else {
              // implicit ELSE body until END [IF]
              let else_vec = collect_stmts_until(|p| p.check(End))?;
              Some(Box::new(Stmt::Block(else_vec)))
          }
      } else { None };
      while match(Semicolon) {}
      self.expect_end_any()?;
      return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
  } else {
      // existing single-statement THEN/ELSE path
  }

Function collect_stmts_until (utility suggestion, optional)
* A small local loop that:
  loop {
      while match(Semicolon) {}
      if stop_predicate(self) { break; }
      if check(Eof) { return Err(... unterminated ...); }
      let line = peek_line();
      let s = parse_stmt()?;
      out.push(Stmt::Line(line)); out.push(s);
  }

Backward compatibility
* Default behavior remains identical: parse(src) constructs a Parser with implicit_multiline_if: false.
* When the option is enabled, a program that previously relied on “THEN newline single statement” may now be interpreted as a multi-line IF requiring END [IF]. That is why this is gated. Tooling (CLI flag or per-file pragma) can opt in later.

CLI and tooling (follow-up idea)
* Expose a CLI switch in the frontends to set the option, for example: --implicit-multiline-if.
* Consider a file-level pragma at the top of a source file (e.g., '@implicit_multiline_if on') to enable it; implementing pragmas would need a small driver hook and is out of scope for this change.

Tests to add (once implemented)
1) Single-statement (baseline):
   * IF x THEN PRINT 1
   * IF x THEN\nPRINT 1
   * IF x THEN PRINT 1 ELSE PRINT 2
   These must continue to parse to a single Stmt in the THEN/ELSE branches when the option is false.

2) Implicit multi-line (option true):
   * THEN followed by newline, multiple statements, END IF.
   * THEN implicit, ELSE implicit, END IF.
   * THEN implicit, ELSE single-statement (including ELSE IF), followed by END IF.

3) Error cases:
   * Unterminated THEN implicit body (EOF before END/ELSE).
   * Unterminated ELSE implicit body (EOF before END IF).

4) Nesting:
   * IF (implicit) inside IF (implicit), and mixed with BEGIN and braces.

Documentation updates
* docs/guides/BEGIN_END.md: add a short note that an experimental option exists to allow IF … THEN multi-line bodies without BEGIN, terminated by END [IF]. Clarify that it is off by default for compatibility.
* This document (docs/development/IF_BEGINLESS.md) serves as the detailed design.

Future refinement (optional)
* Backtracking approach: With a small parser snapshot/rollback facility, we could auto-detect multi-line IF without a flag by speculatively parsing one statement, and if the next token is END or ELSE, treat it as single-statement; otherwise, if followed by more statements and eventually END, treat it as implicit multi-line. However, current parser state has side effects (with_depth/catch_depth), so adding safe rollback is non-trivial and is deferred.
