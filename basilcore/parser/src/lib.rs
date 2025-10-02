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
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        // FUNC name(params) block
        if self.match_k(TokenKind::Func) { return self.parse_func(); }

        if self.match_k(TokenKind::Let) {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Assign)?;
            let init = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Let { name, init });
        }

        if self.match_k(TokenKind::Print) {
            let e = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;
            return Ok(Stmt::Print { expr: e });
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
            let then_s = Box::new(self.parse_stmt()?);
            let else_s = if self.match_k(TokenKind::Else) {
                Some(Box::new(self.parse_stmt()?))
            } else { None };
            return Ok(Stmt::If { cond, then_branch: then_s, else_branch: else_s });
        }

        if self.match_k(TokenKind::Begin) {
            let mut inner = Vec::new();
            while !self.match_k(TokenKind::End) {
                if self.check(TokenKind::Eof) { return Err(BasilError("unterminated BEGIN/END".into())); }
                inner.push(self.parse_stmt()?);
            }
            return Ok(Stmt::Block(inner));
        }

        // Fallback: expression statement
        let e = self.parse_expr_bp(0)?;
        self.terminate_stmt()?;
        Ok(Stmt::ExprStmt(e))
    }

    // Accept ';' OR EOF after a statement
    fn terminate_stmt(&mut self) -> Result<()> {
        if self.match_k(TokenKind::Semicolon) { return Ok(()); }
        if self.check(TokenKind::Eof) { return Ok(()); }
        Err(BasilError("expected Semicolon".into()))
    }

    // Pratt parser with postfix call and comparisons
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.parse_prefix()?;

        // postfix calls (highest precedence)
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
        match self.peek_kind() {
            Some(TokenKind::Number) => {
                let t = self.next().unwrap();
                if let Some(Literal::Num(n)) = t.literal { Ok(Expr::Number(n)) } else { Err(BasilError("number literal missing".into())) }
            }
            Some(TokenKind::String) => {
                let t = self.next().unwrap();
                if let Some(Literal::Str(s)) = t.literal { Ok(Expr::Str(s)) } else { Err(BasilError("string literal missing".into())) }
            }
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
            Some(TokenKind::LParen) => { self.next(); let e = self.parse_expr_bp(0)?; self.expect(TokenKind::RParen)?; Ok(e) }
            other => Err(BasilError(format!("unexpected token in expression: {:?}", other))),
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
        // function body is a block: BEGIN ... END
        if !self.match_k(TokenKind::Begin) {
            return Err(BasilError("expected BEGIN after function header".into()));
        }
        let mut body = Vec::new();
        while !self.match_k(TokenKind::End) {
            if self.check(TokenKind::Eof) { return Err(BasilError("unterminated function body".into())); }
            body.push(self.parse_stmt()?);
        }
        Ok(Stmt::Func { name, params, body })
    }

    fn peek_binop_bp(&self) -> Option<(BinOp, u8, u8)> {
        match self.peek_kind()? {
            // comparisons (lower precedence)
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
            _ => None,
        }
    }

    // small helpers
    fn expect(&mut self, k: TokenKind) -> Result<Token> {
        if self.check(k.clone()) { Ok(self.next().unwrap()) } else { Err(BasilError(format!("expected {:?}", k))) }
    }
    fn expect_ident(&mut self) -> Result<String> {
        if self.check(TokenKind::Ident) { Ok(self.next().unwrap().lexeme) } else { Err(BasilError("expected identifier".into())) }
    }
    fn check(&self, k: TokenKind) -> bool { self.peek_kind() == Some(k) }
    fn match_k(&mut self, k: TokenKind) -> bool { if self.check(k) { self.next(); true } else { false } }
    fn peek_kind(&self) -> Option<TokenKind> { self.tokens.get(self.i).map(|t| t.kind.clone()) }
    fn next(&mut self) -> Option<Token> { let t = self.tokens.get(self.i).cloned(); if t.is_some() { self.i+=1; } t }
}
