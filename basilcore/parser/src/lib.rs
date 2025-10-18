/*

 ▄▄▄▄    ██▓    ▄▄▄       ▄████▄   ██ ▄█▀ ██▀███   █    ██   ██████  ██░ ██
▓█████▄ ▓██▒   ▒████▄    ▒██▀ ▀█   ██▄█▒ ▓██ ▒ ██▒ ██  ▓██▒▒██    ▒ ▓██░ ██▒
▒██▒ ▄██▒██░   ▒██  ▀█▄  ▒▓█    ▄ ▓███▄░ ▓██ ░▄█ ▒▓██  ▒██░░ ▓██▄   ▒██▀▀██░
▒██░█▀  ▒██░   ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄ ▒██▀▀█▄  ▓▓█  ░██░  ▒   ██▒░▓█ ░██
░▓█  ▀█▓░██████▒▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄░██▓ ▒██▒▒▒█████▓ ▒██████▒▒░▓█▒░██▓
░▒▓███▀▒░ ▒░▓  ░▒▒   ▓▒█░░ ░▒ ▒  ░▒ ▒▒ ▓▒░ ▒▓ ░▒▓░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░ ▒ ░░▒░▒
▒░▒   ░ ░ ░ ▒  ░ ▒   ▒▒ ░  ░  ▒   ░ ░▒ ▒░  ░▒ ░ ▒░░░▒░ ░ ░ ░ ░▒  ░ ░ ▒ ░▒░ ░
 ░    ░   ░ ░    ░   ▒   ░        ░ ░░ ░   ░░   ░  ░░░ ░ ░ ░  ░  ░   ░  ░░ ░
 ░          ░  ░     ░  ░░ ░      ░  ░      ░        ░           ░   ░  ░  ░
      ░                  ░
Copyright (C) 2026, Blackrush LLC
Created by Erik Olson, Tarpon Springs, Florida
For more information, visit BlackrushDrive.com

MIT License

Copyright (c) 2026 Erik Lee Olson for Blackrush, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

*/

//! Pratt parser with functions, calls, return, if, blocks, comparisons
use basil_common::{Result, BasilError};
use basil_lexer::{Lexer, Token, TokenKind, Literal};
use basil_ast::{Expr, Stmt, BinOp, Program};

pub fn parse(src: &str) -> Result<Program> {
    let mut lx = Lexer::new(src);
    let tokens = lx.tokenize()?;
    Parser::new(tokens).parse_program()
}

struct Parser { tokens: Vec<Token>, i: usize, with_depth: usize, catch_depth: usize }

impl Parser {
    fn new(tokens: Vec<Token>) -> Self { Self { tokens, i: 0, with_depth: 0, catch_depth: 0 } }

    fn parse_program(&mut self) -> Result<Program> {
        let mut stmts = Vec::new();
        while !self.check(TokenKind::Eof) {
            // Skip any stray semicolons (e.g., from newline insertion)
            while self.match_k(TokenKind::Semicolon) {}
            if self.check(TokenKind::Eof) { break; }
            let line = self.peek_line();
            let s = self.parse_stmt()?;
            stmts.push(Stmt::Line(line));
            stmts.push(s);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        // Skip any leading semicolons (useful with newline-as-semicolon)
        while self.match_k(TokenKind::Semicolon) {}

        // SELECT CASE <expr> ... END [SELECT]
        if self.match_k(TokenKind::Select) {
            self.expect(TokenKind::Case)?;
            let selector = self.parse_expr_bp(0)?;
            // Accept newline or ':' (both as Semicolon) after header
            while self.match_k(TokenKind::Semicolon) {}
            // Optional brace-delimited form: SELECT CASE <expr> { ... }
            if self.match_k(TokenKind::LBrace) {
                let mut arms: Vec<basil_ast::CaseArm> = Vec::new();
                let mut else_body: Option<Vec<Stmt>> = None;
                let mut saw_else = false;
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError("Expected '}' to terminate SELECT CASE body.".into())); }
                    if self.match_k(TokenKind::Case) {
                        if self.match_k(TokenKind::Else) {
                            if saw_else { return Err(BasilError("Only one CASE ELSE is allowed.".into())); }
                            saw_else = true;
                            while self.match_k(TokenKind::Semicolon) {}
                            let mut body: Vec<Stmt> = Vec::new();
                            loop {
                                while self.match_k(TokenKind::Semicolon) {}
                                if self.check(TokenKind::RBrace) { break; }
                                if self.check(TokenKind::Case) { break; }
                                if self.check(TokenKind::Eof) { return Err(BasilError("Expected '}' to terminate SELECT CASE body.".into())); }
                                let line = self.peek_line();
                                let s = self.parse_stmt()?;
                                body.push(Stmt::Line(line));
                                body.push(s);
                            }
                            else_body = Some(body);
                            continue;
                        }
                        // Parse one or more patterns separated by commas
                        let mut patterns: Vec<basil_ast::CasePattern> = Vec::new();
                        loop {
                            if self.match_k(TokenKind::Is) {
                                let op = match self.peek_kind() {
                                    Some(TokenKind::EqEq) | Some(TokenKind::Assign) => { let _ = self.next(); BinOp::Eq },
                                    Some(TokenKind::BangEq) => { let _ = self.next(); BinOp::Ne },
                                    Some(TokenKind::Lt) => { let _ = self.next(); BinOp::Lt },
                                    Some(TokenKind::LtEq) => { let _ = self.next(); BinOp::Le },
                                    Some(TokenKind::Gt) => { let _ = self.next(); BinOp::Gt },
                                    Some(TokenKind::GtEq) => { let _ = self.next(); BinOp::Ge },
                                    _ => return Err(BasilError("Use 'CASE IS <op> <expr>' with one comparator operator.".into())),
                                };
                                let rhs = self.parse_expr_bp(0)?;
                                patterns.push(basil_ast::CasePattern::Compare { op, rhs });
                            } else {
                                let first = self.parse_expr_bp(0)?;
                                if self.match_k(TokenKind::To) {
                                    let hi = self.parse_expr_bp(0)?;
                                    patterns.push(basil_ast::CasePattern::Range { lo: first, hi });
                                } else {
                                    patterns.push(basil_ast::CasePattern::Value(first));
                                }
                            }
                            if self.match_k(TokenKind::Comma) { continue; }
                            break;
                        }
                        if patterns.is_empty() {
                            return Err(BasilError("CASE requires at least one value, range, or comparator.".into()));
                        }
                        while self.match_k(TokenKind::Semicolon) {}
                        let mut body: Vec<Stmt> = Vec::new();
                        loop {
                            while self.match_k(TokenKind::Semicolon) {}
                            if self.check(TokenKind::Case) || self.check(TokenKind::RBrace) { break; }
                            if self.check(TokenKind::Eof) { return Err(BasilError("Expected '}' to terminate SELECT CASE body.".into())); }
                            let line = self.peek_line();
                            let s = self.parse_stmt()?;
                            body.push(Stmt::Line(line));
                            body.push(s);
                        }
                        arms.push(basil_ast::CaseArm { patterns, body });
                        continue;
                    }
                    return Err(BasilError("Expected 'CASE' or '}' inside SELECT CASE.".into()));
                }
                return Ok(Stmt::SelectCase { selector, arms, else_body });
            }
            let mut arms: Vec<basil_ast::CaseArm> = Vec::new();
            let mut else_body: Option<Vec<Stmt>> = None;
            let mut saw_else = false;
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.check(TokenKind::End) {
                    // consume END and optional SELECT suffix
                    let _ = self.next();
                    if self.check(TokenKind::Select) { let _ = self.next(); }
                    break;
                }
                if self.check(TokenKind::Eof) {
                    return Err(BasilError("Expected 'END' or 'END SELECT' to terminate SELECT CASE block.".into()));
                }
                if self.match_k(TokenKind::Case) {
                    if self.match_k(TokenKind::Else) {
                        if saw_else { return Err(BasilError("Only one CASE ELSE is allowed.".into())); }
                        saw_else = true;
                        // Accept nl_or_colon, then collect body until END or next CASE
                        while self.match_k(TokenKind::Semicolon) {}
                        let mut body: Vec<Stmt> = Vec::new();
                        loop {
                            while self.match_k(TokenKind::Semicolon) {}
                            if self.check(TokenKind::End) || self.check(TokenKind::Case) { break; }
                            if self.check(TokenKind::Eof) { return Err(BasilError("Expected 'END' or 'END SELECT' to terminate SELECT CASE block.".into())); }
                            let line = self.peek_line();
                            let s = self.parse_stmt()?;
                            body.push(Stmt::Line(line));
                            body.push(s);
                        }
                        else_body = Some(body);
                        continue;
                    }
                    // Parse one or more patterns separated by commas
                    let mut patterns: Vec<basil_ast::CasePattern> = Vec::new();
                    loop {
                        // 'IS' comparator form
                        if self.match_k(TokenKind::Is) {
                            let op = match self.peek_kind() {
                                Some(TokenKind::EqEq) | Some(TokenKind::Assign) => { let _ = self.next(); BinOp::Eq },
                                Some(TokenKind::BangEq) => { let _ = self.next(); BinOp::Ne },
                                Some(TokenKind::Lt) => { let _ = self.next(); BinOp::Lt },
                                Some(TokenKind::LtEq) => { let _ = self.next(); BinOp::Le },
                                Some(TokenKind::Gt) => { let _ = self.next(); BinOp::Gt },
                                Some(TokenKind::GtEq) => { let _ = self.next(); BinOp::Ge },
                                _ => return Err(BasilError("Use 'CASE IS <op> <expr>' with one comparator operator.".into())),
                            };
                            let rhs = self.parse_expr_bp(0)?;
                            patterns.push(basil_ast::CasePattern::Compare { op, rhs });
                        } else {
                            // Value or range form
                            let first = self.parse_expr_bp(0)?;
                            if self.match_k(TokenKind::To) {
                                // Range must be 'expr TO expr'
                                let hi = self.parse_expr_bp(0)?;
                                patterns.push(basil_ast::CasePattern::Range { lo: first, hi });
                            } else {
                                patterns.push(basil_ast::CasePattern::Value(first));
                            }
                        }
                        if self.match_k(TokenKind::Comma) { continue; }
                        break;
                    }
                    if patterns.is_empty() {
                        return Err(BasilError("CASE requires at least one value, range, or comparator.".into()));
                    }
                    // Accept nl_or_colon, then parse body until next CASE or END
                    while self.match_k(TokenKind::Semicolon) {}
                    let mut body: Vec<Stmt> = Vec::new();
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.check(TokenKind::End) || self.check(TokenKind::Case) { break; }
                        if self.check(TokenKind::Eof) { return Err(BasilError("Expected 'END' or 'END SELECT' to terminate SELECT CASE block.".into())); }
                        let line = self.peek_line();
                        let s = self.parse_stmt()?;
                        body.push(Stmt::Line(line));
                        body.push(s);
                    }
                    arms.push(basil_ast::CaseArm { patterns, body });
                    continue;
                }
                // If we reached here, we expected either CASE or END
                return Err(BasilError("Expected 'END' or 'END SELECT' to terminate SELECT CASE block.".into()));
            }
            return Ok(Stmt::SelectCase { selector, arms, else_body });
        }

        // WITH <expr> ... END WITH
        if self.match_k(TokenKind::With) {
            let target = self.parse_expr_bp(0)?;
            // Accept newline or ':' before body
            while self.match_k(TokenKind::Semicolon) {}
            // Enter WITH scope
            self.with_depth += 1;
            let mut body: Vec<Stmt> = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.check(TokenKind::End) {
                    let _ = self.next(); // consume END
                    if self.match_k(TokenKind::With) {
                        break;
                    } else {
                        return Err(BasilError("Expected 'END WITH' to terminate WITH block.".into()));
                    }
                }
                if self.check(TokenKind::Eof) {
                    return Err(BasilError("Expected 'END WITH' to terminate WITH block.".into()));
                }
                let line = self.peek_line();
                let s = self.parse_stmt()?;
                body.push(Stmt::Line(line));
                body.push(s);
            }
            // Exit WITH scope
            self.with_depth -= 1;
            return Ok(Stmt::With { target, body });
        }

        // TRY ... [CATCH [err$] ...] [FINALLY ...] END TRY
        if self.match_k(TokenKind::Try) {
            // Accept newline or ':' before body
            while self.match_k(TokenKind::Semicolon) {}
            let mut try_body: Vec<Stmt> = Vec::new();
            // Collect try-body until CATCH/FINALLY/END
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.check(TokenKind::Catch) || self.check(TokenKind::Finally) || self.check(TokenKind::End) { break; }
                if self.check(TokenKind::Eof) { return Err(BasilError("Expected 'END TRY' to terminate TRY block.".into())); }
                let line = self.peek_line();
                let s = self.parse_stmt()?;
                try_body.push(Stmt::Line(line));
                try_body.push(s);
            }
            let mut saw_catch = false;
            let mut saw_finally = false;
            let mut catch_var: Option<String> = None;
            let mut catch_body: Option<Vec<Stmt>> = None;
            let mut finally_body: Option<Vec<Stmt>> = None;

            // Parse optional CATCH/FINALLY sections in any order, at most one each
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.match_k(TokenKind::Catch) {
                    if saw_catch { return Err(BasilError("Only one CATCH block is allowed per TRY.".into())); }
                    saw_catch = true;
                    // Optional ident for error var
                    if self.check(TokenKind::Ident) {
                        let name = self.expect_ident()?;
                        if !name.ends_with('$') { return Err(BasilError("CATCH variable must be a string (use '$' suffix).".into())); }
                        catch_var = Some(name);
                    }
                    // Accept nl_or_colon before body
                    while self.match_k(TokenKind::Semicolon) {}
                    let mut body: Vec<Stmt> = Vec::new();
                    // Enter CATCH context
                    self.catch_depth += 1;
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.check(TokenKind::Finally) || self.check(TokenKind::End) { break; }
                        if self.check(TokenKind::Eof) { self.catch_depth -= 1; return Err(BasilError("Expected 'END TRY' to terminate TRY block.".into())); }
                        let line = self.peek_line();
                        let s = self.parse_stmt()?;
                        body.push(Stmt::Line(line));
                        body.push(s);
                    }
                    self.catch_depth -= 1;
                    catch_body = Some(body);
                    continue;
                }
                if self.match_k(TokenKind::Finally) {
                    if saw_finally { return Err(BasilError("Only one FINALLY block is allowed per TRY.".into())); }
                    saw_finally = true;
                    // Accept nl_or_colon before body
                    while self.match_k(TokenKind::Semicolon) {}
                    let mut body: Vec<Stmt> = Vec::new();
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.check(TokenKind::Catch) || self.check(TokenKind::End) { break; }
                        if self.check(TokenKind::Eof) { return Err(BasilError("Expected 'END TRY' to terminate TRY block.".into())); }
                        let line = self.peek_line();
                        let s = self.parse_stmt()?;
                        body.push(Stmt::Line(line));
                        body.push(s);
                    }
                    finally_body = Some(body);
                    continue;
                }
                break;
            }
            if !saw_catch && !saw_finally { return Err(BasilError("TRY must contain a CATCH or FINALLY block.".into())); }
            // Expect END TRY
            self.expect(TokenKind::End)?;
            while self.match_k(TokenKind::Semicolon) {}
            if !self.match_k(TokenKind::Try) {
                return Err(BasilError("Expected 'END TRY' to terminate TRY block.".into()));
            }
            return Ok(Stmt::Try { try_body, catch_var, catch_body, finally_body });
        }

        // FUNC/SUB name(params) block
        if self.check(TokenKind::Func) {
            let kw = self.next().unwrap();
            let kind = if kw.lexeme.eq_ignore_ascii_case("SUB") { basil_ast::FuncKind::Sub } else { basil_ast::FuncKind::Func };
            return self.parse_func(kind);
        }

        // LABEL name  or  IDENT:  (colon-form)
        if self.check(TokenKind::Label) {
            // consume the Label token first
            let tok = self.next().unwrap();
            // If the next token is an identifier, this is the keyword form: LABEL name
            let name = if self.check(TokenKind::Ident) {
                self.expect_ident()?
            } else {
                // colon-form: the label name is carried in the Label token's lexeme
                tok.lexeme
            };
            self.terminate_stmt()?;
            return Ok(Stmt::Label(name));
        }
        // GOTO name
        if self.match_k(TokenKind::Goto) {
            let name = self.expect_ident()?;
            self.terminate_stmt()?;
            return Ok(Stmt::Goto(name));
        }
        // GOSUB name
        if self.match_k(TokenKind::Gosub) {
            let name = self.expect_ident()?;
            self.terminate_stmt()?;
            return Ok(Stmt::Gosub(name));
        }

        // SETENV name = expr
        if self.match_k(TokenKind::Setenv) {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Assign)?;
            let value = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::SetEnv { name, value, export: false });
        }
        // EXPORTENV name = expr
        if self.match_k(TokenKind::Exportenv) {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Assign)?;
            let value = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::SetEnv { name, value, export: true });
        }
        // SHELL expr
        if self.match_k(TokenKind::Shell) {
            let cmd = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Shell { cmd });
        }
        // EXIT [expr]
        if self.match_k(TokenKind::Exit) {
            let expr = if self.check(TokenKind::Semicolon) || self.check(TokenKind::Eof) { None } else { Some(self.parse_expr_bp(0)?) };
            self.terminate_stmt()?;
            return Ok(Stmt::Exit(expr));
        }
        // STOP
        if self.match_k(TokenKind::Stop) {
            self.terminate_stmt()?;
            return Ok(Stmt::Stop);
        }

        // RAISE [expr]
        if self.match_k(TokenKind::Raise) {
            let expr_opt = if self.check(TokenKind::Semicolon) || self.check(TokenKind::Eof) { None } else { Some(self.parse_expr_bp(0)?) };
            if expr_opt.is_none() && self.catch_depth == 0 {
                return Err(BasilError("RAISE without an expression is only valid inside CATCH.".into()));
            }
            self.terminate_stmt()?;
            return Ok(Stmt::Raise(expr_opt));
        }

        if self.match_k(TokenKind::Let) {
            // Support two forms:
            // 1) LET name[(indices...)] = expr
            // 2) LET obj.Member = expr
            // If we see Ident '.' after LET, parse as member property set.
            if self.check(TokenKind::Ident) {
                let save_i = self.i;
                let obj_name = self.expect_ident()?;
                if self.match_k(TokenKind::Dot) {
                    let prop = self.expect_ident()?;
                    self.expect(TokenKind::Assign)?;
                    let value = self.parse_expr_bp(0)?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::SetProp { target: Expr::Var(obj_name), prop, value });
                } else {
                    // revert and handle standard LET name[...] = expr
                    self.i = save_i;
                }
            }

            let name = self.expect_ident()?;
            // Optional square-bracket indexing for list/dict: LET name '[' expr ']' = value
            if self.match_k(TokenKind::LBracket) {
                let idx = self.parse_expr_bp(0)?;
                self.expect(TokenKind::RBracket)?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expr_bp(0)?;
                self.terminate_stmt()?;
                return Ok(Stmt::SetIndexSquare { target: Expr::Var(name), index: idx, value });
            }
            // Optional indices for array element assignment: name '(' exprlist ')'
            let indices = if self.match_k(TokenKind::LParen) {
                let mut idxs = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        idxs.push(self.parse_expr_bp(0)?);
                        if !self.match_k(TokenKind::Comma) { break; }
                    }
                }
                self.expect(TokenKind::RParen)?;
                // Support LET arr(i).Prop = expr by detecting a following '.'
                if self.match_k(TokenKind::Dot) {
                    let prop = self.expect_member_name()?;
                    self.expect(TokenKind::Assign)?;
                    let value = self.parse_expr_bp(0)?;
                    self.terminate_stmt()?;
                    let call = Expr::Call { callee: Box::new(Expr::Var(name)), args: idxs };
                    return Ok(Stmt::SetProp { target: call, prop, value });
                }
                Some(idxs)
            } else { None };
            self.expect(TokenKind::Assign)?;
            let init = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Let { name, indices, init });
        }

        if self.match_k(TokenKind::Print) {
            // Support PRINT with comma-separated expressions joined by TABs
            let mut e = self.parse_expr_bp(0)?;
            while self.match_k(TokenKind::Comma) {
                let next = self.parse_expr_bp(0)?;
                // e = e + "\t" + next
                e = Expr::Binary { op: BinOp::Add, lhs: Box::new(e), rhs: Box::new(Expr::Str("\t".to_string())) };
                e = Expr::Binary { op: BinOp::Add, lhs: Box::new(e), rhs: Box::new(next) };
            }
            self.terminate_stmt()?;
            return Ok(Stmt::Print { expr: e });
        }

        if self.match_k(TokenKind::Println) {
            // PRINTLN works like PRINT but always appends a newline
            let mut e = self.parse_expr_bp(0)?;
            while self.match_k(TokenKind::Comma) {
                let next = self.parse_expr_bp(0)?;
                e = Expr::Binary { op: BinOp::Add, lhs: Box::new(e), rhs: Box::new(Expr::Str("\t".to_string())) };
                e = Expr::Binary { op: BinOp::Add, lhs: Box::new(e), rhs: Box::new(next) };
            }
            // append newline
            e = Expr::Binary { op: BinOp::Add, lhs: Box::new(e), rhs: Box::new(Expr::Str("\n".to_string())) };
            self.terminate_stmt()?;
            return Ok(Stmt::Print { expr: e });
        }

        if self.match_k(TokenKind::Describe) {
            let target = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Describe { target });
        }

        // EXEC(code$)
        if self.match_k(TokenKind::Exec) {
            self.expect(TokenKind::LParen)?;
            let code = self.parse_expr_bp(0)?;
            self.expect(TokenKind::RParen)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Exec { code });
        }

        if self.match_k(TokenKind::Return) {
            // Distinguish GOSUB-return forms and function-return
            // RETURN TO <label> ;
            if self.match_k(TokenKind::To) {
                let label = self.expect_ident()?;
                self.terminate_stmt()?;
                return Ok(Stmt::ReturnFromGosub(Some(label)));
            }
            // Bare RETURN; → GOSUB return
            if self.check(TokenKind::Semicolon) || self.check(TokenKind::Eof) {
                self.terminate_stmt()?;
                return Ok(Stmt::ReturnFromGosub(None));
            }
            // Otherwise: RETURN <expr> → function return
            let expr = Some(self.parse_expr_bp(0)?);
            self.terminate_stmt()?;
            return Ok(Stmt::Return(expr));
        }

        if self.match_k(TokenKind::If) {
            let cond = self.parse_expr_bp(0)?;
            // Brace form: IF <cond> { ... } [ELSE ...]
            if self.match_k(TokenKind::LBrace) {
                let mut then_body = Vec::new();
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated IF {{ ... }}", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    then_body.push(Stmt::Line(line));
                    then_body.push(stmt);
                }
                let then_s = Box::new(Stmt::Block(then_body));
                let else_s = if self.match_k(TokenKind::Else) {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::If) {
                        let s = self.parse_stmt()?;
                        Some(Box::new(s))
                    } else if self.match_k(TokenKind::LBrace) {
                        let mut else_body = Vec::new();
                        loop {
                            while self.match_k(TokenKind::Semicolon) {}
                            if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                            if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated ELSE {{ ... }}", self.peek_line()))); }
                            let line = self.peek_line();
                            let stmt = self.parse_stmt()?;
                            else_body.push(Stmt::Line(line));
                            else_body.push(stmt);
                        }
                        Some(Box::new(Stmt::Block(else_body)))
                    } else if self.match_k(TokenKind::Begin) {
                        let mut else_body = Vec::new();
                        loop {
                            while self.match_k(TokenKind::Semicolon) {}
                            if self.match_k(TokenKind::End) { self.consume_optional_end_suffix(); break; }
                            if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated ELSE BEGIN/END", self.peek_line()))); }
                            let line = self.peek_line();
                            let stmt = self.parse_stmt()?;
                            else_body.push(Stmt::Line(line));
                            else_body.push(stmt);
                        }
                        Some(Box::new(Stmt::Block(else_body)))
                    } else {
                        let s = self.parse_stmt()?;
                        Some(Box::new(s))
                    }
                } else { None };
                return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
            }

            // Classic forms: require THEN
            self.expect(TokenKind::Then)?;
            // Allow optional semicolons/newlines before BEGIN
            while self.match_k(TokenKind::Semicolon) {}
            // Support both single-statement and block IF forms.
            // Block form: IF <cond> THEN BEGIN ... [ELSE ...] END
            if self.match_k(TokenKind::Begin) {
                // collect THEN block until ELSE or END
                let mut then_body = Vec::new();
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::Else) || self.check(TokenKind::End) { break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated IF THEN BEGIN ...", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    then_body.push(Stmt::Line(line));
                    then_body.push(stmt);
                }
                let then_s = Box::new(Stmt::Block(then_body));
                let else_s = if self.match_k(TokenKind::Else) {
                    // Allow optional semicolons/newlines before BEGIN
                    while self.match_k(TokenKind::Semicolon) {}
                    // Else can be BEGIN ... END or a single statement before END
                    if self.match_k(TokenKind::Begin) {
                        let mut else_body = Vec::new();
                        loop {
                            while self.match_k(TokenKind::Semicolon) {}
                            if self.match_k(TokenKind::End) { self.consume_optional_end_suffix(); break; }
                            if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated ELSE BEGIN/END", self.peek_line()))); }
                            let line = self.peek_line();
                            let stmt = self.parse_stmt()?;
                            else_body.push(Stmt::Line(line));
                            else_body.push(stmt);
                        }
                        Some(Box::new(Stmt::Block(else_body)))
                    } else {
                        let s = self.parse_stmt()?;
                        // After a single-statement ELSE, require END to close the IF
                        while self.match_k(TokenKind::Semicolon) {}
                        self.expect_end_any()?;
                        Some(Box::new(s))
                    }
                } else {
                    // No ELSE: require END to close the IF
                    while self.match_k(TokenKind::Semicolon) {}
                    self.expect_end_any()?;
                    None
                };
                return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
            } else {
                // Simple form: single statements for THEN and optional ELSE
                let then_line = self.peek_line();
                let then_stmt = self.parse_stmt()?;
                let then_s = Box::new(Stmt::Block(vec![Stmt::Line(then_line), then_stmt]));
                let else_s = if self.match_k(TokenKind::Else) {
                    let else_line = self.peek_line();
                    let es = self.parse_stmt()?;
                    Some(Box::new(Stmt::Block(vec![Stmt::Line(else_line), es])))
                } else { None };
                return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
            }
        }

        // WHILE <expr> BEGIN ... END  or  WHILE <expr> { ... }
        if self.match_k(TokenKind::While) {
            let cond = self.parse_expr_bp(0)?;
            let mut body = Vec::new();
            if self.match_k(TokenKind::Begin) {
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.match_k(TokenKind::End) { self.consume_optional_end_suffix(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated WHILE BEGIN/END", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    body.push(Stmt::Line(line));
                    body.push(stmt);
                }
            } else if self.match_k(TokenKind::LBrace) {
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated WHILE {{ ... }}", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    body.push(Stmt::Line(line));
                    body.push(stmt);
                }
            } else {
                return Err(BasilError("Expected 'BEGIN' or '{' after WHILE condition".into()));
            }
            return Ok(Stmt::While { cond, body: Box::new(Stmt::Block(body)) });
        }

        if self.match_k(TokenKind::Break) { self.terminate_stmt()?; return Ok(Stmt::Break); }
        if self.match_k(TokenKind::Continue) { self.terminate_stmt()?; return Ok(Stmt::Continue); }

        if self.match_k(TokenKind::LBrace) {
            let mut inner = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated brace block", self.peek_line()))); }
                let line = self.peek_line();
                let stmt = self.parse_stmt()?;
                inner.push(Stmt::Line(line));
                inner.push(stmt);
            }
            return Ok(Stmt::Block(inner));
        }

        if self.match_k(TokenKind::Begin) {
            let mut inner = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.match_k(TokenKind::End) { self.consume_optional_end_suffix(); break; }
                if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated BEGIN/END", self.peek_line()))); }
                let line = self.peek_line();
                let stmt = self.parse_stmt()?;
                inner.push(Stmt::Line(line));
                inner.push(stmt);
            }
            return Ok(Stmt::Block(inner));
        }

        // TYPE ... END TYPE (struct definition) or TYPE Name { ... }
        if self.match_k(TokenKind::Type) {
            let type_name = self.expect_ident()?;
            // Optional brace body
            let mut fields: Vec<basil_ast::StructField> = Vec::new();
            if self.match_k(TokenKind::LBrace) {
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated TYPE {{ ... }}", self.peek_line()))); }
                    // Expect field declaration starting with DIM
                    if !self.match_k(TokenKind::Dim) { return Err(BasilError(format!("parse error at line {}: expected DIM in TYPE body", self.peek_line()))); }
                    let (fname, fkind) = self.parse_struct_field()?;
                    fields.push(basil_ast::StructField { name: fname, kind: fkind });
                }
            } else {
                // Classic TYPE ... END TYPE form
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::End) {
                        let _ = self.next(); // consume END
                        if !self.match_k(TokenKind::Type) { return Err(BasilError(format!("parse error at line {}: expected 'END TYPE'", self.peek_line()))); }
                        break;
                    }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated TYPE ... END TYPE", self.peek_line()))); }
                    if !self.match_k(TokenKind::Dim) { return Err(BasilError(format!("parse error at line {}: expected DIM in TYPE body", self.peek_line()))); }
                    let (fname, fkind) = self.parse_struct_field()?;
                    fields.push(basil_ast::StructField { name: fname, kind: fkind });
                }
            }
            self.terminate_stmt().ok(); // tolerate optional terminator
            return Ok(Stmt::TypeDef { name: type_name, fields });
        }

        if self.match_k(TokenKind::For) {
            // Check FOR EACH form first
            if self.match_k(TokenKind::Each) {
                // FOR EACH ident IN expr <body> NEXT [ident]
                let var = self.expect_ident()?;
                self.expect(TokenKind::In)?;
                let enumerable = self.parse_expr_bp(0)?;
                // Body: BEGIN..END, {..}, or single statement
                let body: Stmt = if self.match_k(TokenKind::Begin) {
                    let mut inner = Vec::new();
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.match_k(TokenKind::End) { break; }
                        if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated FOR EACH BEGIN/END", self.peek_line()))); }
                        let line = self.peek_line();
                        let s = self.parse_stmt()?;
                        inner.push(Stmt::Line(line));
                        inner.push(s);
                    }
                    Stmt::Block(inner)
                } else if self.match_k(TokenKind::LBrace) {
                    let mut inner = Vec::new();
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                        if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated FOR EACH {{ ... }}", self.peek_line()))); }
                        let line = self.peek_line();
                        let s = self.parse_stmt()?;
                        inner.push(Stmt::Line(line));
                        inner.push(s);
                    }
                    Stmt::Block(inner)
                } else {
                    let line = self.peek_line();
                    let s = self.parse_stmt()?;
                    Stmt::Block(vec![Stmt::Line(line), s])
                };
                // Expect NEXT [ident]
                while self.match_k(TokenKind::Semicolon) {}
                self.expect(TokenKind::Next)?;
                if self.check(TokenKind::Ident) { let _ = self.next(); }
                let _ = self.terminate_stmt();
                return Ok(Stmt::ForEach { var, enumerable, body: Box::new(body) });
            }

            // Classic FOR var = start TO end [STEP step] <stmt-or-block> NEXT [var]
            let var = self.expect_ident()?;
            self.expect(TokenKind::Assign)?;
            let start = self.parse_expr_bp(0)?;
            self.expect(TokenKind::To)?;
            let end = self.parse_expr_bp(0)?;
            let step = if self.match_k(TokenKind::Step) { Some(self.parse_expr_bp(0)?) } else { None };

            // Body: BEGIN..END, {..}, or single statement
            let body: Stmt = if self.match_k(TokenKind::Begin) {
                let mut inner = Vec::new();
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.match_k(TokenKind::End) { break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated FOR BEGIN/END", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    inner.push(Stmt::Line(line));
                    inner.push(stmt);
                }
                Stmt::Block(inner)
            } else if self.match_k(TokenKind::LBrace) {
                let mut inner = Vec::new();
                loop {
                    while self.match_k(TokenKind::Semicolon) {}
                    if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
                    if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated FOR {{ ... }}", self.peek_line()))); }
                    let line = self.peek_line();
                    let stmt = self.parse_stmt()?;
                    inner.push(Stmt::Line(line));
                    inner.push(stmt);
                }
                Stmt::Block(inner)
            } else {
                // Single statement body
                let line = self.peek_line();
                let s = self.parse_stmt()?;
                Stmt::Block(vec![Stmt::Line(line), s])
            };

            // Expect NEXT [ident]
            while self.match_k(TokenKind::Semicolon) {}
            self.expect(TokenKind::Next)?;
            if self.check(TokenKind::Ident) { let _ = self.next(); }
            // Optional terminator after NEXT
            let _ = self.terminate_stmt();

            return Ok(Stmt::For { var, start, end, step, body: Box::new(body) });
        }

        if self.match_k(TokenKind::Dim) {
            let name = self.expect_ident()?;
            // Fixed-length string bracket form: DIM name$[N]
            if name.ends_with('$') && self.match_k(TokenKind::LBracket) {
                // Expect integer literal for N
                let n_tok = self.expect(TokenKind::Number)?;
                let n = if let Some(basil_lexer::Literal::Num(v)) = n_tok.literal { v as usize } else { 0usize };
                self.expect(TokenKind::RBracket)?;
                self.terminate_stmt()?;
                return Ok(Stmt::DimFixedStr { name, len: n });
            }
            if self.match_k(TokenKind::LParen) {
                let mut dims = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        dims.push(self.parse_expr_bp(0)?);
                        if !self.match_k(TokenKind::Comma) { break; }
                    }
                }
                self.expect(TokenKind::RParen)?;
                // Optional: AS Type for object arrays
                if self.match_k(TokenKind::As) {
                    let tname = self.expect_ident()?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::DimObjectArray { name, dims, type_name: Some(tname) });
                } else {
                    // If name ends with '@', treat as untyped object array
                    if name.ends_with('@') {
                        self.terminate_stmt()?;
                        return Ok(Stmt::DimObjectArray { name, dims, type_name: None });
                    } else {
                        self.terminate_stmt()?;
                        return Ok(Stmt::Dim { name, dims });
                    }
                }
            } else if self.match_k(TokenKind::As) {
                // Support: DIM name@ AS CLASS(filename)
                if self.match_k(TokenKind::Class) {
                    self.expect(TokenKind::LParen)?;
                    let fname = self.parse_expr_bp(0)?;
                    self.expect(TokenKind::RParen)?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::Let { name, indices: None, init: Expr::NewClass { filename: Box::new(fname) } });
                }
                // Support: DIM name$ AS STRING * N  (fixed-length string)
                if self.check(TokenKind::Ident) {
                    // Peek without consuming to check for STRING
                    let t = self.tokens.get(self.i).unwrap();
                    if t.lexeme.eq_ignore_ascii_case("STRING") {
                        let _ = self.next(); // consume IDENT("STRING")
                        if self.match_k(TokenKind::Star) {
                            let n_tok = self.expect(TokenKind::Number)?;
                            let n = if let Some(basil_lexer::Literal::Num(v)) = n_tok.literal { v as usize } else { 0usize };
                            self.terminate_stmt()?;
                            return Ok(Stmt::DimFixedStr { name, len: n });
                        } else {
                            return Err(BasilError(format!("parse error at line {}: expected '*' and length after STRING", self.peek_line())));
                        }
                    }
                }
                // Support: DIM name AS TYPE TypeName
                if self.match_k(TokenKind::Type) {
                    let tname = self.expect_ident()?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::DimObject { name, type_name: tname, args: Vec::new() });
                }
                // Default: DIM name AS TypeName [(args)] — object/struct scalar
                let tname = self.expect_ident()?;
                let mut args = Vec::new();
                if self.match_k(TokenKind::LParen) {
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr_bp(0)?);
                            if !self.match_k(TokenKind::Comma) { break; }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                }
                self.terminate_stmt()?;
                return Ok(Stmt::DimObject { name, type_name: tname, args });
            } else if self.match_k(TokenKind::Assign) {
                // Support: DIM name = expr
                let init_expr = self.parse_expr_bp(0)?;
                self.terminate_stmt()?;
                match init_expr {
                    Expr::List(items) => {
                        // If variable is a primitive-typed array (name suffix '%' or '$'),
                        // desugar to: DIM name(upper=n) + element assignments name(1..n) = items.
                        // Otherwise (e.g., name ends with '@' or no suffix), treat as simple LET of a dynamic list.
                        if name.ends_with('%') || name.ends_with('$') {
                            let n = items.len();
                            let mut stmts: Vec<Stmt> = Vec::new();
                            stmts.push(Stmt::Dim { name: name.clone(), dims: vec![Expr::Number(n as f64)] });
                            for (i, it) in items.into_iter().enumerate() {
                                let idx_expr = Expr::Number((i as f64) + 1.0);
                                stmts.push(Stmt::Let { name: name.clone(), indices: Some(vec![idx_expr]), init: it });
                            }
                            return Ok(Stmt::Block(stmts));
                        } else {
                            return Ok(Stmt::Let { name, indices: None, init: Expr::List(items) });
                        }
                    }
                    other => {
                        // Fallback: treat as LET name = expr
                        return Ok(Stmt::Let { name, indices: None, init: other });
                    }
                }
            } else {
                return Err(BasilError(format!("parse error at line {}: expected '(' or AS after DIM name", self.peek_line())));
            }
        }

        // Special-case: SLEEP expr or SLEEP(expr) as a statement without requiring parentheses
        if self.check(TokenKind::Ident) {
            let save_i = self.i;
            let name = self.expect_ident()?;
            if name.eq_ignore_ascii_case("SLEEP") {
                // Accept either SLEEP(expr) or SLEEP expr
                let arg = if self.match_k(TokenKind::LParen) {
                    let e = self.parse_expr_bp(0)?;
                    self.expect(TokenKind::RParen)?;
                    e
                } else {
                    self.parse_expr_bp(0)?
                };
                self.terminate_stmt()?;
                let call = Expr::Call { callee: Box::new(Expr::Var("SLEEP".to_string())), args: vec![arg] };
                return Ok(Stmt::ExprStmt(call));
            } else {
                // Support zero-arg terminal commands as bare statements without parentheses
                // e.g., CLS; HOME; CLEAR; COLOR_RESET; ATTR_RESET; CURSOR_SAVE; CURSOR_RESTORE; CURSOR_HIDE; CURSOR_SHOW;
                let uname = name.to_ascii_uppercase();
                const ZERO_ARG_TERMINAL_CMDS: [&str; 9] = [
                    "CLS", "CLEAR", "HOME",
                    "COLOR_RESET", "ATTR_RESET",
                    "CURSOR_SAVE", "CURSOR_RESTORE",
                    "CURSOR_HIDE", "CURSOR_SHOW",
                ];
                if ZERO_ARG_TERMINAL_CMDS.contains(&uname.as_str()) {
                    // Optionally accept empty parentheses: NAME or NAME()
                    if self.match_k(TokenKind::LParen) {
                        // For these commands, only empty parens are allowed in statement form
                        self.expect(TokenKind::RParen)?;
                    }
                    self.terminate_stmt()?;
                    let call = Expr::Call { callee: Box::new(Expr::Var(name)), args: vec![] };
                    return Ok(Stmt::ExprStmt(call));
                }
                // Not a special-case; rewind and continue with regular parsing
                self.i = save_i;
            }
        }

        // Fallback: detect assignment-like forms first to enforce LET for variable assignment,
        // while still allowing obj.Prop = expr without LET. Otherwise, parse an expression statement.
        let save_i = self.i;
        // Probe a potential left-hand chain: prefix + postfix (calls and member access only)
        let lhs_probe = (|| {
            let mut lhs = self.parse_prefix()?;
            loop {
                if self.match_k(TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr_bp(0)?);
                            if !self.match_k(TokenKind::Comma) { break; }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    lhs = Expr::Call { callee: Box::new(lhs), args };
                    continue;
                }
                if self.match_k(TokenKind::Dot) {
                    let name = self.expect_member_name()?;
                    if self.match_k(TokenKind::LParen) {
                        let mut args = Vec::new();
                        if !self.check(TokenKind::RParen) {
                            loop {
                                args.push(self.parse_expr_bp(0)?);
                                if !self.match_k(TokenKind::Comma) { break; }
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        lhs = Expr::MemberCall { target: Box::new(lhs), method: name, args };
                    } else {
                        lhs = Expr::MemberGet { target: Box::new(lhs), name };
                    }
                    continue;
                }
                if self.match_k(TokenKind::LBracket) {
                    let idx = self.parse_expr_bp(0)?;
                    self.expect(TokenKind::RBracket)?;
                    lhs = Expr::IndexSquare { target: Box::new(lhs), index: Box::new(idx) };
                    continue;
                }
                break;
            }
            Ok::<Expr, BasilError>(lhs)
        })();
        if let Ok(lhs) = lhs_probe {
            if self.check(TokenKind::Assign) {
                // Allow assignment without LET for member property targets: obj.Prop = expr
                // and for list/dict square-bracket indexing: obj[expr] = value
                if let Expr::MemberGet { target, name } = lhs {
                    let _ = self.next(); // consume '='
                    let value = self.parse_expr_bp(0)?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::SetProp { target: *target, prop: name, value });
                } else if let Expr::IndexSquare { target, index } = lhs {
                    let _ = self.next(); // consume '='
                    let value = self.parse_expr_bp(0)?;
                    self.terminate_stmt()?;
                    return Ok(Stmt::SetIndexSquare { target: *target, index: *index, value });
                } else {
                    return Err(BasilError("Use LET for assignment; '=' in expressions tests equality.".into()));
                }
            }
            // Not an assignment pattern; reset before parsing general expression
            self.i = save_i;
        } else {
            // If probe failed, reset and proceed to parse expression normally (will error if invalid)
            self.i = save_i;
        }
        let e = self.parse_expr_bp(0)?;
        self.terminate_stmt()?;
        Ok(Stmt::ExprStmt(e))
    }

    // Accept ';' OR EOF after a statement
    fn terminate_stmt(&mut self) -> Result<()> {
        if self.match_k(TokenKind::Semicolon) { return Ok(()); }
        if self.check(TokenKind::Eof) { return Ok(()); }
        Err(BasilError(format!("parse error at line {}: expected Semicolon or Colon", self.peek_line())))
    }

    // Pratt parser with postfix call and comparisons
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.parse_prefix()?;

        // postfix calls, member access, and square-bracket indexing (highest precedence)
        loop {
            if self.match_k(TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr_bp(0)?);
                        if !self.match_k(TokenKind::Comma) { break; }
                    }
                }
                self.expect(TokenKind::RParen)?;
                lhs = Expr::Call { callee: Box::new(lhs), args };
                continue;
            }
            if self.match_k(TokenKind::LBracket) {
                let idx = self.parse_expr_bp(0)?;
                self.expect(TokenKind::RBracket)?;
                lhs = Expr::IndexSquare { target: Box::new(lhs), index: Box::new(idx) };
                continue;
            }
            if self.match_k(TokenKind::Dot) {
                let name = self.expect_member_name()?;
                if self.match_k(TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr_bp(0)?);
                            if !self.match_k(TokenKind::Comma) { break; }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    lhs = Expr::MemberCall { target: Box::new(lhs), method: name, args };
                } else {
                    lhs = Expr::MemberGet { target: Box::new(lhs), name };
                }
                continue;
            }
            break;
        }

        loop {
            // binary operator?
            let (op, lbp, rbp) = if let Some((op, lb, rb)) = self.peek_binop_bp() { (op, lb, rb) } else { break };
            if lbp < min_bp { break; }
            self.next(); // consume operator
            let rhs = self.parse_expr_bp(rbp)?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr> {
        if self.match_k(TokenKind::Minus) {
            let e = self.parse_expr_bp(80)?;
            return Ok(Expr::UnaryNeg(Box::new(e)));
        }
        if self.match_k(TokenKind::Not) {
            let e = self.parse_expr_bp(80)?;
            return Ok(Expr::UnaryNot(Box::new(e)));
        }
        match self.peek_kind() {
            Some(TokenKind::Dot) => {
                // Leading '.' parsed as ImplicitThis member access; validity (WITH scope) is enforced during compilation.
                // This avoids false parse errors when newline continuation or formatting places '.' at line start.
                let _ = self.next(); // consume '.'
                let name = self.expect_member_name()?;
                if self.match_k(TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr_bp(0)?);
                            if !self.match_k(TokenKind::Comma) { break; }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::MemberCall { target: Box::new(Expr::ImplicitThis), method: name, args })
                } else {
                    Ok(Expr::MemberGet { target: Box::new(Expr::ImplicitThis), name })
                }
            }
            Some(TokenKind::Number) => {
                let t = self.next().unwrap();
                if let Some(Literal::Num(n)) = t.literal { Ok(Expr::Number(n)) } else { Err(BasilError(format!("parse error at line {}: number literal missing", t.line))) }
            }
            Some(TokenKind::String) => {
                let t = self.next().unwrap();
                if let Some(Literal::Str(s)) = t.literal { Ok(Expr::Str(s)) } else { Err(BasilError(format!("parse error at line {}: string literal missing", t.line))) }
            }
            Some(TokenKind::True) => { let _ = self.next().unwrap(); Ok(Expr::Bool(true)) }
            Some(TokenKind::False) => { let _ = self.next().unwrap(); Ok(Expr::Bool(false)) }
            Some(TokenKind::Author) => {
                // Consume AUTHOR token
                let _ = self.next().unwrap();
                // Allow optional empty parentheses: AUTHOR or AUTHOR()
                if self.match_k(TokenKind::LParen) {
                    self.expect(TokenKind::RParen)?;
                }
                Ok(Expr::Str("Erik Olson".to_string()))
            }
            Some(TokenKind::Ident) => Ok(Expr::Var(self.next().unwrap().lexeme)),
            Some(TokenKind::New) => {
                // NEW Type(args)
                let _ = self.next().unwrap();
                let type_name = self.expect_ident()?;
                self.expect(TokenKind::LParen)?;
                let mut args = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr_bp(0)?);
                        if !self.match_k(TokenKind::Comma) { break; }
                    }
                }
                self.expect(TokenKind::RParen)?;
                Ok(Expr::NewObject { type_name, args })
            }
            Some(TokenKind::Class) => {
                // CLASS(filename)
                let _ = self.next().unwrap();
                self.expect(TokenKind::LParen)?;
                let fname = self.parse_expr_bp(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::NewClass { filename: Box::new(fname) })
            }
            Some(TokenKind::Eval) => {
                // EVAL(expr)
                let _ = self.next().unwrap();
                self.expect(TokenKind::LParen)?;
                let inner = self.parse_expr_bp(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Eval(Box::new(inner)))
            }
            Some(TokenKind::LBracket) => {
                // List literal: [ expr (, expr)* ,? ] with optional newlines/semicolons between elements
                let _ = self.next(); // consume '['
                let mut items: Vec<Expr> = Vec::new();
                // allow stray semicolons/newlines
                while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                if !self.check(TokenKind::RBracket) {
                    loop {
                        while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                        items.push(self.parse_expr_bp(0)?);
                        while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                        if self.match_k(TokenKind::Comma) { continue; }
                        break;
                    }
                    // optional trailing comma
                    let _ = self.match_k(TokenKind::Comma);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::List(items))
            }
            Some(TokenKind::LBrace) => {
                // Dict literal: { "key": expr (, "key": expr)* ,? }
                let _ = self.next(); // consume '{'
                let mut entries: Vec<(String, Expr)> = Vec::new();
                // allow stray semicolons/newlines
                while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                if !self.check(TokenKind::RBrace) {
                    loop {
                        while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                        // key must be string literal
                        let key_tok = self.expect(TokenKind::String)?;
                        let key = if let Some(basil_lexer::Literal::Str(s)) = key_tok.literal { s } else { return Err(BasilError("Dictionary key must be a quoted string literal".into())); };
                        // colon separator
                        self.expect(TokenKind::Colon)?;
                        let value = self.parse_expr_bp(0)?;
                        entries.push((key, value));
                        while self.check(TokenKind::Semicolon) { let _ = self.next(); }
                        if self.match_k(TokenKind::Comma) { continue; }
                        break;
                    }
                    // optional trailing comma
                    let _ = self.match_k(TokenKind::Comma);
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::Dict(entries))
            }
            Some(TokenKind::LParen) => { self.next(); let e = self.parse_expr_bp(0)?; self.expect(TokenKind::RParen)?; Ok(e) }
            other => Err(BasilError(format!("parse error at line {}: unexpected token in expression: {:?}", self.peek_line(), other))),
        }
    }

    fn parse_func(&mut self, kind: basil_ast::FuncKind) -> Result<Stmt> {
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                params.push(self.expect_ident()?);
                if !self.match_k(TokenKind::Comma) { break; }
            }
        }
        self.expect(TokenKind::RParen)?;
        // allow optional semicolons/newlines before body
        while self.match_k(TokenKind::Semicolon) {}
        // Function body forms supported:
        // 1) BEGIN ... END [FUNC]
        // 2) { ... }
        // 3) Implicit body terminated by END [FUNC]
        let is_brace_body = if self.match_k(TokenKind::LBrace) { true } else { false };
        let has_begin = if !is_brace_body && self.match_k(TokenKind::Begin) { true } else { false };
        let mut body = Vec::new();
        loop {
            while self.match_k(TokenKind::Semicolon) {}
            if is_brace_body {
                if self.check(TokenKind::RBrace) { let _ = self.next(); break; }
            } else if has_begin {
                if self.match_k(TokenKind::End) { self.consume_optional_end_suffix(); break; }
            } else {
                if self.check(TokenKind::End) {
                    let _ = self.next(); // consume END
                    // optional FUNC (includes FUNCTION/SUB) after END
                    if self.check(TokenKind::Func) { let _ = self.next(); }
                    break;
                }
            }
            if self.check(TokenKind::Eof) {
                return Err(BasilError(match (is_brace_body, has_begin) {
                    (true, _) => format!("parse error at line {}: unterminated function body: expected '}}'", self.peek_line()),
                    (_, true) => format!("parse error at line {}: unterminated function body: expected 'END'", self.peek_line()),
                    _ => format!("parse error at line {}: unterminated function body", self.peek_line()),
                }));
            }
            let line = self.peek_line();
            let stmt = self.parse_stmt()?;
            body.push(Stmt::Line(line));
            body.push(stmt);
        }
        Ok(Stmt::Func { kind, name, params, body })
    }

    fn peek_binop_bp(&self) -> Option<(BinOp, u8, u8)> {
        match self.peek_kind()? {
            // logical (lowest precedence)
            TokenKind::Or => Some((BinOp::Or, 20, 21)),
            TokenKind::And => Some((BinOp::And, 30, 31)),
            // comparisons (allow '=' as alias of '==')
            TokenKind::EqEq | TokenKind::Assign => Some((BinOp::Eq, 40, 41)),
            TokenKind::BangEq => Some((BinOp::Ne, 40, 41)),
            TokenKind::Lt => Some((BinOp::Lt, 50, 51)),
            TokenKind::LtEq => Some((BinOp::Le, 50, 51)),
            TokenKind::Gt => Some((BinOp::Gt, 50, 51)),
            TokenKind::GtEq => Some((BinOp::Ge, 50, 51)),
            // additive
            TokenKind::Plus => Some((BinOp::Add, 60, 61)),
            TokenKind::Minus => Some((BinOp::Sub, 60, 61)),
            // multiplicative
            TokenKind::Star => Some((BinOp::Mul, 70, 71)),
            TokenKind::Slash => Some((BinOp::Div, 70, 71)),
            TokenKind::Mod => Some((BinOp::Mod, 70, 71)),
            _ => None,
        }
    }

    // small helpers
    fn expect(&mut self, k: TokenKind) -> Result<Token> {
        if self.check(k.clone()) { Ok(self.next().unwrap()) } else { Err(BasilError(format!("parse error at line {}: expected {:?}", self.peek_line(), k))) }
    }
    // Expect END and consume optional alias suffix words like IF/FUNC/FUNCTION/SUB/WHILE/BLOCK
    fn expect_end_any(&mut self) -> Result<()> {
        let _ = self.expect(TokenKind::End)?;
        self.consume_optional_end_suffix();
        Ok(())
    }
    fn consume_optional_end_suffix(&mut self) {
        // Accept a following token that is either IF/FUNC/WHILE or an identifier 'BLOCK'
        match self.peek_kind() {
            Some(TokenKind::If) | Some(TokenKind::Func) | Some(TokenKind::While) => { let _ = self.next(); }
            Some(TokenKind::Ident) => {
                // Allow END BLOCK (Ident form)
                let t = self.tokens.get(self.i).unwrap();
                if t.lexeme.eq_ignore_ascii_case("BLOCK") { let _ = self.next(); }
            }
            _ => {}
        }
    }
    fn expect_ident(&mut self) -> Result<String> {
        if self.check(TokenKind::Ident) { Ok(self.next().unwrap().lexeme) } else { Err(BasilError(format!("parse error at line {}: expected identifier", self.peek_line()))) }
    }
    // Accept an identifier or a keyword token as a member name after '.'
    fn expect_member_name(&mut self) -> Result<String> {
        match self.peek_kind() {
            Some(TokenKind::Ident)
            | Some(TokenKind::Func)
            | Some(TokenKind::Return)
            | Some(TokenKind::If)
            | Some(TokenKind::Then)
            | Some(TokenKind::Else)
            | Some(TokenKind::While)
            | Some(TokenKind::Do)
            | Some(TokenKind::Begin)
            | Some(TokenKind::End)
            | Some(TokenKind::With)
            | Some(TokenKind::Break)
            | Some(TokenKind::Continue)
            | Some(TokenKind::Let)
            | Some(TokenKind::Print)
            | Some(TokenKind::Println)
            | Some(TokenKind::True)
            | Some(TokenKind::False)
            | Some(TokenKind::Null)
            | Some(TokenKind::And)
            | Some(TokenKind::Or)
            | Some(TokenKind::Not)
            | Some(TokenKind::Author)
            | Some(TokenKind::For)
            | Some(TokenKind::To)
            | Some(TokenKind::Step)
            | Some(TokenKind::Next)
            | Some(TokenKind::Each)
            | Some(TokenKind::In)
            | Some(TokenKind::Foreach)
            | Some(TokenKind::Endfor)
            | Some(TokenKind::Dim)
            | Some(TokenKind::As)
            | Some(TokenKind::Describe)
            | Some(TokenKind::New)
            | Some(TokenKind::Class)
            | Some(TokenKind::Setenv)
            | Some(TokenKind::Exportenv)
            | Some(TokenKind::Shell)
            | Some(TokenKind::Exit)
            | Some(TokenKind::Label)
            | Some(TokenKind::Goto)
            | Some(TokenKind::Gosub)
            | Some(TokenKind::Mod)
            | Some(TokenKind::Exec)
            | Some(TokenKind::Eval) => {
                Ok(self.next().unwrap().lexeme)
            }
            _ => Err(BasilError(format!("parse error at line {}: expected identifier", self.peek_line()))),
        }
    }
    fn check(&self, k: TokenKind) -> bool { self.peek_kind() == Some(k) }
    fn match_k(&mut self, k: TokenKind) -> bool {
        if self.check(k.clone()) { self.next(); true }
        else if matches!(k, TokenKind::Semicolon) && self.check(TokenKind::Colon) { self.next(); true }
        else { false }
    }
    fn peek_kind(&self) -> Option<TokenKind> { self.tokens.get(self.i).map(|t| t.kind.clone()) }
    fn peek_line(&self) -> u32 { self.tokens.get(self.i).map(|t| t.line).unwrap_or(0) }

    // Parse a single struct field declaration after 'DIM' in a TYPE body.
    // Returns (field_name, field_kind).
    fn parse_struct_field(&mut self) -> Result<(String, basil_ast::StructFieldKind)> {
        use basil_ast::StructFieldKind as SFK;
        let fname = self.expect_ident()?;
        // Optional classic array dims for fields not supported in this minimal pass
        if self.match_k(TokenKind::LParen) {
            return Err(BasilError(format!("parse error at line {}: array fields in TYPE not supported yet", self.peek_line())));
        }
        // Type clause or infer from suffix
        let kind = if self.match_k(TokenKind::As) {
            if self.check(TokenKind::Ident) {
                let t = self.tokens.get(self.i).unwrap();
                if t.lexeme.eq_ignore_ascii_case("STRING") {
                    let _ = self.next(); // consume IDENT("STRING")
                    if self.match_k(TokenKind::Star) {
                        let n_tok = self.expect(TokenKind::Number)?;
                        let n = if let Some(basil_lexer::Literal::Num(v)) = n_tok.literal { v as usize } else { 0usize };
                        SFK::FixedString(n)
                    } else {
                        SFK::VarString
                    }
                } else {
                    // AS <TypeName>
                    let tname = self.expect_ident()?;
                    SFK::Struct(tname)
                }
            } else if self.match_k(TokenKind::Type) {
                let tname = self.expect_ident()?;
                SFK::Struct(tname)
            } else {
                return Err(BasilError(format!("parse error at line {}: expected type after AS", self.peek_line())));
            }
        } else {
            if fname.ends_with('%') { SFK::Int32 }
            else if fname.ends_with('$') { SFK::VarString }
            else { SFK::Float64 }
        };
        // Consume optional statement terminator here if present; caller may also handle.
        while self.match_k(TokenKind::Semicolon) {}
        Ok((fname, kind))
    }

    fn next(&mut self) -> Option<Token> { let t = self.tokens.get(self.i).cloned(); if t.is_some() { self.i+=1; } t }
}
