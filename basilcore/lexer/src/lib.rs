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
//! Lexer for Basil v0 (fixed start positions + clean string/ident spans)
use basil_common::{Result, BasilError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Single-char
    LParen, RParen, Comma, Semicolon,
    Plus, Minus, Star, Slash,
    Lt, Gt, Assign,        // '<' '>' '='
    // Two-char
    EqEq, BangEq, LtEq, GtEq,
    // Literals / identifiers
    Ident, Number, String,
    // Keywords
    Func, Return, If, Then, Else, While, Do, Begin, End,
    Let, Print, True, False, Null, And, Or, Not,
    Author,
    // New for FOR loop support
    For, To, Step, Next,
    Eof,
}

#[derive(Debug, Clone)]
pub enum Literal { Num(f64), Str(String) }

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub literal: Option<Literal>,
    pub span: Span,
}

pub struct Lexer<'a> {
    src:   &'a str,
    chars: std::str::Chars<'a>,
    cur:   Option<char>,
    pos:   usize, // byte offset *after* `cur`
    start: usize, // byte offset start of current token
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut l = Self { src, chars: src.chars(), cur: None, pos: 0, start: 0 };
        l.advance(); // prime `cur` and `pos`
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

        // If no current char, emit EOF
        let ch = match self.cur {
            Some(c) => c,
            None => return Ok(self.make_with_span(TokenKind::Eof, self.pos, self.pos)),
        };

        // IMPORTANT: set `start` to the *beginning* of the current character
        let clen = ch.len_utf8();
        self.start = self.pos - clen;

        match ch {
            // --- single-char tokens: make FIRST, advance AFTER ---
            '(' => { let tok = self.make(TokenKind::LParen);    self.advance(); Ok(tok) }
            ')' => { let tok = self.make(TokenKind::RParen);    self.advance(); Ok(tok) }
            ',' => { let tok = self.make(TokenKind::Comma);     self.advance(); Ok(tok) }
            ';' => { let tok = self.make(TokenKind::Semicolon); self.advance(); Ok(tok) }
            '+' => { let tok = self.make(TokenKind::Plus);      self.advance(); Ok(tok) }
            '-' => { let tok = self.make(TokenKind::Minus);     self.advance(); Ok(tok) }
            '*' => { let tok = self.make(TokenKind::Star);      self.advance(); Ok(tok) }
            '/' => { let tok = self.make(TokenKind::Slash);     self.advance(); Ok(tok) }

            // --- two-char possibilities: keep existing logic ---
            '=' => {
                self.advance();
                if self.match_char('=') { Ok(self.make(TokenKind::EqEq)) }
                else { Ok(self.make(TokenKind::Assign)) }
            }
            '!' => {
                self.advance();
                if self.match_char('=') { Ok(self.make(TokenKind::BangEq)) }
                else { Err(BasilError("unexpected '!'".into())) }
            }
            '<' => {
                self.advance();
                if self.match_char('=') { Ok(self.make(TokenKind::LtEq)) }
                else { Ok(self.make(TokenKind::Lt)) }
            }
            '>' => {
                self.advance();
                if self.match_char('=') { Ok(self.make(TokenKind::GtEq)) }
                else { Ok(self.make(TokenKind::Gt)) }
            }

            '"' => self.string(),
            c if c.is_ascii_digit() => self.number(),
            c if is_ident_start(c)  => self.ident_or_kw(),
            _ => Err(BasilError(format!("unexpected char '{}': pos {}", ch, self.pos))),
        }
    }


    // Build a token using current self.start..self.pos
    fn make(&self, kind: TokenKind) -> Token {
        self.make_with_span(kind, self.start, self.pos)
    }
    fn make_with_span(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            lexeme: self.src[start..end].to_string(),
            literal: None,
            span: Span::new(start, end),
        }
    }

    fn string(&mut self) -> Result<Token> {
        // Opening quote is at `cur`, and `self.start` points at it.
        let start = self.start;

        // Consume opening quote
        self.advance();

        // Accumulate cooked contents
        let mut s = String::new();
        while let Some(c) = self.cur {
            if c == '"' {
                // IMPORTANT: `self.pos` is already AFTER the '"' right now.
                // So the token's end should be exactly `self.pos` (no +1).
                let end = self.pos;

                // Step past the closing quote so the next token sees the next char (e.g. ';')
                self.advance();

                let mut tok = self.make_with_span(TokenKind::String, start, end);
                tok.literal = Some(Literal::Str(s));
                return Ok(tok);
            }
            if c == '\\' {
                self.advance();
                match self.cur {
                    Some('"') => { s.push('"'); self.advance(); }
                    Some('n') => { s.push('\n'); self.advance(); }
                    Some('t') => { s.push('\t'); self.advance(); }
                    Some(c2)  => { s.push(c2);  self.advance(); }
                    None => break,
                }
            } else {
                s.push(c);
                self.advance();
            }
        }
        Err(BasilError("unterminated string".into()))
    }


    fn number(&mut self) -> Result<Token> {
        let start = self.start;
        // end = byte index just AFTER the last digit (or fractional digit)
        let mut end = self.pos; // currently after the first digit

        // integer part
        while matches!(self.cur, Some(c) if c.is_ascii_digit()) {
            end = self.pos;          // after the current digit
            self.advance();          // move to next char
        }

        // fractional part
        if self.cur == Some('.') && matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            // include the dot
            end = self.pos;          // pos is already after '.'
            self.advance();          // step into first fractional digit
            while matches!(self.cur, Some(c) if c.is_ascii_digit()) {
                end = self.pos;      // after this digit
                self.advance();
            }
        }

        let lex = &self.src[start..end];
        let n: f64 = lex.parse().map_err(|e| BasilError(format!("invalid number '{}': {}", lex, e)))?;
        let mut tok = self.make_with_span(TokenKind::Number, start, end);
        tok.literal = Some(Literal::Num(n));
        Ok(tok)
    }


    fn ident_or_kw(&mut self) -> Result<Token> {
        let start = self.start;
        let mut end = self.pos; // after first ident char
        loop {
            match self.cur {
                Some(c) if is_ident_continue(c) => { end = self.pos; self.advance(); }
                _ => break,
            }
        }
        let lex = &self.src[start..end];
        let kind = match &*lex.to_ascii_uppercase() {
            "FUNC"   => TokenKind::Func,
            "RETURN" => TokenKind::Return,
            "IF"     => TokenKind::If,
            "THEN"   => TokenKind::Then,
            "ELSE"   => TokenKind::Else,
            "WHILE"  => TokenKind::While,
            "DO"     => TokenKind::Do,
            "BEGIN"  => TokenKind::Begin,
            "END"    => TokenKind::End,
            "LET"    => TokenKind::Let,
            "PRINT"  => TokenKind::Print,
            "TRUE"   => TokenKind::True,
            "FALSE"  => TokenKind::False,
            "NULL"   => TokenKind::Null,
            "AND"    => TokenKind::And,
            "OR"     => TokenKind::Or,
            "NOT"    => TokenKind::Not,
            "AUTHOR" => TokenKind::Author,
            "FOR"    => TokenKind::For,
            "TO"     => TokenKind::To,
            "STEP"   => TokenKind::Step,
            "NEXT"   => TokenKind::Next,
            _        => TokenKind::Ident,
        };
        Ok(self.make_with_span(kind, start, end))
    }


    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.cur {
                Some(c) if c.is_whitespace() => { self.advance(); }

                // BASIC-style single-quote comment
                Some('\'') => {
                    while let Some(ch) = self.cur {
                        if ch == '\n' { break; }
                        self.advance();
                    }
                }

                // C++-style line comment: //
                Some('/') if self.peek() == Some('/') => {
                    // consume both slashes
                    self.advance(); // consumed first '/'
                    self.advance(); // consumed second '/'
                    // then consume until newline or EOF
                    while let Some(ch) = self.cur {
                        if ch == '\n' { break; }
                        self.advance();
                    }
                }

                _ => break,
            }
        }
    }


    fn advance(&mut self) {
        self.cur = self.chars.next();
        if let Some(c) = self.cur {
            self.pos += c.len_utf8();
        } else {
            self.pos = self.src.len();
        }
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
