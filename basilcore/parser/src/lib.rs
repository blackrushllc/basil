//! Pratt parser for a tiny Basil subset: LET, PRINT, expressions (+,-,*,/, parentheses, unary -)
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
        while !self.check(TokenKind::Eof) { stmts.push(self.parse_stmt()?); }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        if self.match_k(TokenKind::Let) {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Assign)?;
            let init = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;                  // <- changed
            return Ok(Stmt::Let { name, init });
        }
        if self.match_k(TokenKind::Print) {
            let e = self.parse_expr_bp(0)?;
            self.terminate_stmt()?;                  // <- changed
            return Ok(Stmt::Print { expr: e });
        }
        let e = self.parse_expr_bp(0)?;
        self.terminate_stmt()?;                      // <- changed
        Ok(Stmt::ExprStmt(e))
    }

    // Accept ';' OR end-of-file after a statement
    fn terminate_stmt(&mut self) -> Result<()> {
        if self.match_k(TokenKind::Semicolon) { return Ok(()); }
        if self.check(TokenKind::Eof) { return Ok(()); }
        Err(BasilError("expected Semicolon".into()))
    }



    // Pratt parser
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.parse_prefix()?;
        loop {
            let op = if let Some(op) = self.peek_binop() { op } else { break };
            let (lbp, rbp) = infix_bp(op);
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
            Some(TokenKind::Ident) => Ok(Expr::Var(self.next().unwrap().lexeme)),
            Some(TokenKind::LParen) => { self.next(); let e = self.parse_expr_bp(0)?; self.expect(TokenKind::RParen)?; Ok(e) }
            other => Err(BasilError(format!("unexpected token in expression: {:?}", other))),
        }
    }

    fn peek_binop(&self) -> Option<BinOp> {
        match self.peek_kind()? {
            TokenKind::Plus  => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star  => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            _ => None,
        }
    }

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

fn infix_bp(op: BinOp) -> (u8,u8) {
    match op {
        BinOp::Add | BinOp::Sub => (60, 61),
        BinOp::Mul | BinOp::Div => (70, 71),
    }
}
