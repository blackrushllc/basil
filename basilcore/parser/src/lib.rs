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
Copyright (C) 2026, Blackrush LLC, All Rights Reserved
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

struct Parser { tokens: Vec<Token>, i: usize }

impl Parser {
    fn new(tokens: Vec<Token>) -> Self { Self { tokens, i: 0 } }

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
        // FUNC name(params) block
        if self.match_k(TokenKind::Func) { return self.parse_func(); }

        // LABEL name
        if self.match_k(TokenKind::Label) {
            let name = self.expect_ident()?;
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

        if self.match_k(TokenKind::Return) {
            // optional expression before terminator
            let expr = if self.check(TokenKind::Semicolon) || self.check(TokenKind::Eof) {
                None
            } else {
                Some(self.parse_expr_bp(0)?)
            };
            self.terminate_stmt()?;
            return Ok(Stmt::Return(expr));
        }

        if self.match_k(TokenKind::If) {
            let cond = self.parse_expr_bp(0)?;
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
                            if self.match_k(TokenKind::End) { break; }
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
                        self.expect(TokenKind::End)?;
                        Some(Box::new(s))
                    }
                } else {
                    // No ELSE: require END to close the IF
                    while self.match_k(TokenKind::Semicolon) {}
                    self.expect(TokenKind::End)?;
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

        // WHILE <expr> BEGIN ... END
        if self.match_k(TokenKind::While) {
            let cond = self.parse_expr_bp(0)?;
            self.expect(TokenKind::Begin)?;
            let mut body = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.match_k(TokenKind::End) { break; }
                if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated WHILE BEGIN/END", self.peek_line()))); }
                let line = self.peek_line();
                let stmt = self.parse_stmt()?;
                body.push(Stmt::Line(line));
                body.push(stmt);
            }
            return Ok(Stmt::While { cond, body: Box::new(Stmt::Block(body)) });
        }

        if self.match_k(TokenKind::Break) { self.terminate_stmt()?; return Ok(Stmt::Break); }
        if self.match_k(TokenKind::Continue) { self.terminate_stmt()?; return Ok(Stmt::Continue); }

        if self.match_k(TokenKind::Begin) {
            let mut inner = Vec::new();
            loop {
                while self.match_k(TokenKind::Semicolon) {}
                if self.match_k(TokenKind::End) { break; }
                if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated BEGIN/END", self.peek_line()))); }
                let line = self.peek_line();
                let stmt = self.parse_stmt()?;
                inner.push(Stmt::Line(line));
                inner.push(stmt);
            }
            return Ok(Stmt::Block(inner));
        }

        if self.match_k(TokenKind::For) {
            // Check FOR EACH form first
            if self.match_k(TokenKind::Each) {
                // FOR EACH ident IN expr <body> NEXT [ident]
                let var = self.expect_ident()?;
                self.expect(TokenKind::In)?;
                let enumerable = self.parse_expr_bp(0)?;
                // Body: either BEGIN..END or single statement
                let body: Stmt = if self.match_k(TokenKind::Begin) {
                    let mut inner = Vec::new();
                    loop {
                        while self.match_k(TokenKind::Semicolon) {}
                        if self.match_k(TokenKind::End) { break; }
                        if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated FOR EACH BEGIN/END", self.peek_line()))); }
                        inner.push(self.parse_stmt()?);
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

            // Body: either BEGIN..END or single statement
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

        // Fallback: either member property assignment (without LET) or expression statement
        let e = self.parse_expr_bp(0)?;
        if self.check(TokenKind::Assign) {
            // Only allow assignment without LET for member property targets: obj.Prop = expr
            if let Expr::MemberGet { target, name } = e.clone() {
                let _ = self.next(); // consume '='
                let value = self.parse_expr_bp(0)?;
                self.terminate_stmt()?;
                return Ok(Stmt::SetProp { target: *target, prop: name, value });
            } else {
                return Err(BasilError(format!("parse error at line {}: assignment without LET is only allowed for object properties (obj.Prop = expr)", self.peek_line())));
            }
        }
        self.terminate_stmt()?;
        Ok(Stmt::ExprStmt(e))
    }

    // Accept ';' OR EOF after a statement
    fn terminate_stmt(&mut self) -> Result<()> {
        if self.match_k(TokenKind::Semicolon) { return Ok(()); }
        if self.check(TokenKind::Eof) { return Ok(()); }
        Err(BasilError(format!("parse error at line {}: expected Semicolon", self.peek_line())))
    }

    // Pratt parser with postfix call and comparisons
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.parse_prefix()?;

        // postfix calls and member access (highest precedence)
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
            Some(TokenKind::LParen) => { self.next(); let e = self.parse_expr_bp(0)?; self.expect(TokenKind::RParen)?; Ok(e) }
            other => Err(BasilError(format!("parse error at line {}: unexpected token in expression: {:?}", self.peek_line(), other))),
        }
    }

    fn parse_func(&mut self) -> Result<Stmt> {
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
        // Function body: either BEGIN ... END, or an implicit block terminated by END [FUNC]
        let has_begin = self.match_k(TokenKind::Begin);
        let mut body = Vec::new();
        loop {
            while self.match_k(TokenKind::Semicolon) {}
            if has_begin {
                if self.match_k(TokenKind::End) { break; }
            } else {
                if self.check(TokenKind::End) {
                    let _ = self.next(); // consume END
                    // optional FUNC after END
                    if self.check(TokenKind::Func) { let _ = self.next(); }
                    break;
                }
            }
            if self.check(TokenKind::Eof) { return Err(BasilError(format!("parse error at line {}: unterminated function body", self.peek_line()))); }
            let line = self.peek_line();
            let stmt = self.parse_stmt()?;
            body.push(Stmt::Line(line));
            body.push(stmt);
        }
        Ok(Stmt::Func { name, params, body })
    }

    fn peek_binop_bp(&self) -> Option<(BinOp, u8, u8)> {
        match self.peek_kind()? {
            // logical (lowest precedence)
            TokenKind::Or => Some((BinOp::Or, 20, 21)),
            TokenKind::And => Some((BinOp::And, 30, 31)),
            // comparisons
            TokenKind::EqEq => Some((BinOp::Eq, 40, 41)),
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
            | Some(TokenKind::Mod) => {
                Ok(self.next().unwrap().lexeme)
            }
            _ => Err(BasilError(format!("parse error at line {}: expected identifier", self.peek_line()))),
        }
    }
    fn check(&self, k: TokenKind) -> bool { self.peek_kind() == Some(k) }
    fn match_k(&mut self, k: TokenKind) -> bool { if self.check(k) { self.next(); true } else { false } }
    fn peek_kind(&self) -> Option<TokenKind> { self.tokens.get(self.i).map(|t| t.kind.clone()) }
    fn peek_line(&self) -> u32 { self.tokens.get(self.i).map(|t| t.line).unwrap_or(0) }
    fn next(&mut self) -> Option<Token> { let t = self.tokens.get(self.i).cloned(); if t.is_some() { self.i+=1; } t }
}
