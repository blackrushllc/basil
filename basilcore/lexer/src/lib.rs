use basil_common::{Result, BasilError, Span};


#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Single-char
    LParen, RParen, Comma, Semicolon,
    Plus, Minus, Star, Slash,
    Lt, Gt, Assign, // '<' '>' '='
    // Two-char
    EqEq, BangEq, LtEq, GtEq,
    // Literals / identifiers
    Ident, Number, String,
    // Keywords
    Func, Return, If, Then, Else, While, Do, Begin, End,
    Let, True, False, Null, And, Or, Not,
    Eof,
}


#[derive(Debug, Clone)]
pub enum Literal { Num(f64), Str(String) }


#[derive(Debug, Clone)]
pub struct Token { pub kind: TokenKind, pub lexeme: String, pub literal: Option<Literal>, pub span: Span }


pub struct Lexer<'a> {
    src: &'a str,
    chars: std::str::Chars<'a>,
    cur: Option<char>,
    pos: usize,
    start: usize,
}


impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut l = Self { src, chars: src.chars(), cur: None, pos: 0, start: 0 };
        l.advance();
        l
    }


    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token()?;
            let eof = t.kind == TokenKind::Eof;
            out.push(t);
            if eof { break; }
        }
        Ok(out)
    }


    fn next_token(&mut self) -> Result<Token> {
        self.skip_ws_and_comments();
        self.start = self.pos;
        let ch = match self.cur { Some(c) => c, None => return Ok(self.make(TokenKind::Eof)) };
        match ch {
            '(' => { self.advance(); Ok(self.make(TokenKind::LParen)) }
            ')' => { self.advance(); Ok(self.make(TokenKind::RParen)) }
            ',' => { self.advance(); Ok(self.make(TokenKind::Comma)) }
            ';' => { self.advance(); Ok(self.make(TokenKind::Semicolon)) }
            '+' => { self.advance(); Ok(self.make(TokenKind::Plus)) }
            '-' => { self.advance(); Ok(self.make(TokenKind::Minus)) }
            '*' => { self.advance(); Ok(self.make(TokenKind::Star)) }
            '/' => {
                self.advance();
                Ok(self.make(TokenKind::Slash))
            }
            '=' => { self.advance(); if self.match_char('=') { Ok(self.make(TokenKind::EqEq)) } else { Ok(self.make(TokenKind::Assign)) } }
            '!' => { self.advance(); if self.match_char('=') { Ok(self.make(TokenKind::BangEq)) } else { Err(BasilError("unexpected '!'".into())) } }
            '<' => { self.advance(); if self.match_char('=') { Ok(self.make(TokenKind::LtEq)) } else { Ok(self.make(TokenKind::Lt)) } }
            '>' => { self.advance(); if self.match_char('=') { Ok(self.make(TokenKind::GtEq)) } else { Ok(self.make(TokenKind::Gt)) } }
            '"' => self.string(),
            c if c.is_ascii_digit() => self.number(),
            c if is_ident_start(c) => self.ident_or_kw(),
            _ => Err(BasilError(format!("unexpected char '{}': pos {}", ch, self.pos)))
        }
    }

    fn string(&mut self) -> Result<Token> {
        // opening quote already in cur at entry
        self.advance();
        let mut s = String::new();
        while let Some(c) = self.cur {
            if c == '"' { self.advance(); break; }
            if c == '\\' { // simple escapes
                self.advance();
                match self.cur { Some('"') => { s.push('"'); self.advance(); }, Some('n') => { s.push('\n'); self.advance(); }, Some('t') => { s.push('\t'); self.advance(); }, Some(c2) => { s.push(c2); self.advance(); }, None => break }
            } else { s.push(c); self.advance(); }
        }
        let mut tok = self.make(TokenKind::String);
        tok.literal = Some(Literal::Str(s));
        Ok(tok)
    }

    fn number(&mut self) -> Result<Token> {
        while self.cur.map_or(false, |c| c.is_ascii_digit()) { self.advance(); }
        if self.cur == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) { self.advance(); while self.cur.map_or(false, |c| c.is_ascii_digit()) { self.advance(); } }
        let lex = &self.src[self.start..self.pos];
        let n: f64 = lex.parse().map_err(|e| BasilError(format!("invalid number '{}': {}", lex, e)))?;
        let mut tok = self.make(TokenKind::Number);
        tok.literal = Some(Literal::Num(n));
        Ok(tok)
    }

    fn ident_or_kw(&mut self) -> Result<Token> {
        while self.cur.map_or(false, |c| is_ident_continue(c)) { self.advance(); }
        let lex = &self.src[self.start..self.pos];
        let kind = match &*lex.to_ascii_uppercase() {
            "FUNC" => TokenKind::Func,
            "RETURN" => TokenKind::Return,
            "IF" => TokenKind::If,
            "THEN" => TokenKind::Then,
            "ELSE" => TokenKind::Else,
            "WHILE" => TokenKind::While,
            "DO" => TokenKind::Do,
            "BEGIN" => TokenKind::Begin,
            "END" => TokenKind::End,
            "LET" => TokenKind::Let,
            "TRUE" => TokenKind::True,
            "FALSE" => TokenKind::False,
            "NULL" => TokenKind::Null,
            "AND" => TokenKind::And,
            "OR" => TokenKind::Or,
            "NOT" => TokenKind::Not,
            _ => TokenKind::Ident,
        };
        Ok(self.make(kind))
    }


    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.cur {
                Some(c) if c.is_whitespace() => { self.advance(); }
                Some('\'') => { // BASIC-style single-quote comment to EOL
                    while let Some(c) = self.cur { if c == '\n' { break; } self.advance(); }
                }
                _ => break,
            }
        }
    }


    fn make(&self, kind: TokenKind) -> Token {
        Token { kind, lexeme: self.src[self.start..self.pos].to_string(), literal: None, span: Span::new(self.start, self.pos) }
    }


    fn advance(&mut self) {
        self.cur = self.chars.next();
        if let Some(c) = self.cur { self.pos += c.len_utf8(); } else { self.pos = self.src.len(); }
    }


    fn match_char(&mut self, want: char) -> bool {
        if self.cur == Some(want) { self.advance(); true } else { false }
    }


    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }
}


fn is_ident_start(c: char) -> bool { c.is_ascii_alphabetic() || c == '_' }
fn is_ident_continue(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' }