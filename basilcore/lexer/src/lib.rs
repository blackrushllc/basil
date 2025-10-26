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
//! Lexer for Basil v0 (fixed start positions + clean string/ident spans)
use basil_common::{Result, BasilError, Span};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Single-char
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Semicolon, Colon,
    Plus, Minus, Star, Slash,
    Dot,
    Mod,
    Lt, Gt, Assign,        // '<' '>' '='
    // Two-char
    EqEq, BangEq, LtEq, GtEq,
    // Literals / identifiers
    Ident, Number, String,
    // Keywords
    Func, Return, If, Then, Else, While, Do, Begin, End, With,
    Break, Continue,
    Let, Print, Println, True, False, Null, And, Or, Not,
    Author,
    // New for FOR loop support
    For, To, Step, Next,
    Each, In, Foreach, Endfor,
    Dim,
    As,
    Describe,
    New,
    Class,
    Type, // TYPE ... END TYPE definitions
    // New for SELECT CASE
    Select, Case, Is,
    // Exceptions
    Try, Catch, Finally, Raise,
    // Env and process control
    Setenv, Exportenv, Shell, Exit, Stop,
    // Unstructured control flow
    Label, Goto, Gosub,
    // Dynamic code execution
    Exec, Eval,
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
    pub line: u32,
}

pub struct Lexer<'a> {
    src:   &'a str,
    chars: std::str::Chars<'a>,
    cur:   Option<char>,
    pos:   usize, // byte offset *after* `cur`
    start: usize, // byte offset start of current token
    line:  usize, // 1-based current line number
    tok_line: usize, // line number at start of current token
    pending_nl_semi: bool, // if true, emit a Semicolon token before next real token
    pending: VecDeque<Token>, // injected tokens (e.g., for string interpolation lowering)
    // --- line continuation state ---
    paren_depth: i32,
    last_was_continuation: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut l = Self {
            src,
            chars: src.chars(),
            cur: None,
            pos: 0,
            start: 0,
            line: 1,
            tok_line: 1,
            pending_nl_semi: false,
            pending: VecDeque::new(),
            paren_depth: 0,
            last_was_continuation: false,
        };
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
        // If we have injected tokens (e.g., from string interpolation), serve them first
        if let Some(tok) = self.pending.pop_front() {
            self.post_emit_adjust(&tok);
            return Ok(tok);
        }

        self.skip_ws_and_comments();

        // Record the line number at the start of the token (or EOF)
        self.tok_line = self.line;

        // If we saw a newline earlier, emit a virtual semicolon (unless at EOF)
        if self.pending_nl_semi {
            if self.cur.is_some() {
                self.pending_nl_semi = false;
                let tok = self.make_with_span(TokenKind::Semicolon, self.pos, self.pos);
                self.post_emit_adjust(&tok);
                return Ok(tok);
            } else {
                // At EOF, don't bother emitting a trailing semicolon
                self.pending_nl_semi = false;
            }
        }

        // If no current char, emit EOF
        let ch = match self.cur {
            Some(c) => c,
            None => {
                let tok = self.make_with_span(TokenKind::Eof, self.pos, self.pos);
                // No need to adjust state for EOF
                return Ok(tok);
            }
        };

        // IMPORTANT: set `start` to the *beginning* of the current character
        let clen = ch.len_utf8();
        self.start = self.pos - clen;

        let tok = match ch {
            // --- single-char tokens: make FIRST, advance AFTER ---
            '(' => { let tok = self.make(TokenKind::LParen);    self.advance(); tok }
            ')' => { let tok = self.make(TokenKind::RParen);    self.advance(); tok }
            '{' => { let tok = self.make(TokenKind::LBrace);    self.advance(); tok }
            '}' => { let tok = self.make(TokenKind::RBrace);    self.advance(); tok }
            '[' => { let tok = self.make(TokenKind::LBracket);  self.advance(); tok }
            ']' => { let tok = self.make(TokenKind::RBracket);  self.advance(); tok }
            ',' => { let tok = self.make(TokenKind::Comma);     self.advance(); tok }
            ';' => { let tok = self.make(TokenKind::Semicolon); self.advance(); tok }
            ':' => { let tok = self.make(TokenKind::Colon);     self.advance(); tok }
            '+' => { let tok = self.make(TokenKind::Plus);      self.advance(); tok }
            '-' => { let tok = self.make(TokenKind::Minus);     self.advance(); tok }
            '*' => { let tok = self.make(TokenKind::Star);      self.advance(); tok }
            '/' => { let tok = self.make(TokenKind::Slash);     self.advance(); tok }
            '.' => { let tok = self.make(TokenKind::Dot);       self.advance(); tok }

            // --- two-char possibilities: keep existing logic ---
            '=' => {
                self.advance();
                if self.match_char('=') { self.make(TokenKind::EqEq) }
                else { self.make(TokenKind::Assign) }
            }
            '!' => {
                self.advance();
                if self.match_char('=') { self.make(TokenKind::BangEq) }
                else { return Err(BasilError("unexpected '!'".into())); }
            }
            '<' => {
                self.advance();
                if self.match_char('=') { self.make(TokenKind::LtEq) }
                else if self.match_char('>') { self.make(TokenKind::BangEq) }
                else { self.make(TokenKind::Lt) }
            }
            '>' => {
                self.advance();
                if self.match_char('=') { self.make(TokenKind::GtEq) }
                else { self.make(TokenKind::Gt) }
            }

            '"' => self.string()?,
            c if c.is_ascii_digit() => self.number()?,
            c if is_ident_start(c)  => self.ident_or_kw()?,
            _ => return Err(BasilError(format!("unexpected char '{}': pos {}", ch, self.pos))),
        };

        self.post_emit_adjust(&tok);
        Ok(tok)
    }


    // Adjust lexer state after emitting a token (track paren depth and continuation contexts)
    fn post_emit_adjust(&mut self, tok: &Token) {
        use TokenKind::*;
        match tok.kind {
            LParen => { self.paren_depth += 1; self.last_was_continuation = true; },
            RParen => { if self.paren_depth > 0 { self.paren_depth -= 1; } self.last_was_continuation = false; },
            // Tokens that require a right operand or continuation
            Plus | Minus | Star | Slash | Dot | Assign | EqEq | BangEq | Lt | LtEq | Gt | GtEq | And | Or | Comma | Mod | To | Step => {
                self.last_was_continuation = true;
            }
            Semicolon | Eof => { self.last_was_continuation = false; }
            _ => { self.last_was_continuation = false; }
        }
    }

    // Look ahead after a newline to see if the next non-space char is a continuation operator.
    // Does not consume any input.
    fn next_after_nl_is_cont_op(&self) -> bool {
        let mut it = self.chars.clone(); // starts AFTER current char
        // skip spaces/tabs/CR
        while let Some(ch) = it.next() {
            match ch {
                ' ' | '\t' | '\r' => continue,
                // If the line starts with a comment, don't treat as continuation
                '\'' => return false,
                '/' => {
                    if let Some('/') = it.clone().next() { return false; }
                    return matches!(ch, '+' | '-' | '*' | '/' | '.' | ',');
                }
                '#' => return false,
                '+' | '-' | '*' | '.' | ',' => return true,
                _ => return false,
            }
        }
        false
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
            line: self.tok_line as u32,
        }
    }

    // Consume whitespace and optional comment to the end of line after a lone '_' ident.
    // Returns true if a newline (or EOF) was consumed, indicating an explicit continuation.
    fn consume_explicit_continuation_after_underscore(&mut self) -> bool {
        // Only allow spaces/tabs/CR before newline or start of a line comment (#, ', //)
        // Save current position; we are currently positioned just after the '_' and before any trailing spaces.
        let mut it = self.chars.clone();
        let mut saw_comment = false;
        // Check ahead without consuming
        loop {
            match it.next() {
                Some(' ') | Some('\t') | Some('\r') => continue,
                Some('\n') => break, // ok
                Some('\'') => { saw_comment = true; break; }
                Some('#') => { saw_comment = true; break; }
                Some('/') => {
                    if let Some('/') = it.next() { saw_comment = true; break; }
                    // a solitary '/' means next line starts with '/'; treat as not a valid explicit continuation context
                    return false;
                }
                Some(_) => return false, // other non-space content => not explicit continuation
                None => break, // EOF is acceptable (treat like newline)
            }
        }
        // Now actually consume until newline (and the newline itself if present)
        loop {
            match self.cur {
                Some(' ') | Some('\t') | Some('\r') => { self.advance(); }
                Some('\'') | Some('#') if saw_comment => {
                    // consume to newline
                    while let Some(ch) = self.cur { if ch == '\n' { break; } self.advance(); }
                }
                Some('/') if saw_comment && self.peek() == Some('/') => {
                    // we're at first '/', consume both then to newline
                    self.advance(); self.advance();
                    while let Some(ch) = self.cur { if ch == '\n' { break; } self.advance(); }
                }
                Some('\n') => { self.advance(); break; }
                None => break,
                _ => { // For safety, if non-space content appears, abort (shouldn't happen due to precheck)
                    break;
                }
            }
        }
        true
    }

    fn string(&mut self) -> Result<Token> {
        // Two-pass approach to avoid leaking outside the string when parsing interpolation.
        // 1) Capture the raw content between quotes without interpreting escapes.
        // 2) Parse that raw content into literal and expression parts; cook escapes only in literals.
        let tok_line = self.tok_line as u32;
        let outer_start = self.start;
        // Record the byte index just AFTER the opening quote
        let content_start = self.pos;
        // Step into the first character (if any)
        self.advance();
        // scan raw until the matching closing quote (respecting escapes)
        let content_end = loop {
            let ch = match self.cur {
                Some(c) => c,
                None => return Err(BasilError("unterminated string".into())),
            };
            if ch == '"' {
                // end should EXCLUDE the closing quote
                let end = self.pos - '"'.len_utf8();
                self.advance();     // step past closing quote
                break end;
            }
            if ch == '\\' {
                // skip escaped char
                self.advance();
                if self.cur.is_some() { self.advance(); }
                continue;
            }
            self.advance();
        };
        let raw = &self.src[content_start..content_end];
        // Helper to decode escapes in literal segments (" \ n t r, \#{ → "#{", \} → '}', and generic \x → x)
        fn push_escape(dst: &mut String, next: Option<char>, iter: &mut std::str::CharIndices) {
            if let Some(nc) = next {
                match nc {
                    '"' => dst.push('"'),
                    'n' => dst.push('\n'),
                    't' => dst.push('\t'),
                    'r' => dst.push('\r'),
                    '#' => {
                        // support \#{ -> literal "#{"
                        dst.push('#');
                        if let Some((_, c3)) = iter.clone().next() {
                            if c3 == '{' {
                                // consume '{'
                                let _ = iter.next();
                                dst.push('{');
                            }
                        }
                    }
                    '}' => dst.push('}'),
                    c => dst.push(c),
                }
            }
        }
        let mut literal_buf = String::new();
        let mut built: Vec<Token> = Vec::new();
        let mut saw_interpolation = false;
        let mut need_plus = false;

        let mut i = 0usize; // byte index into raw
        while i < raw.len() {
            // read next char
            let (ci, ch) = {
                let mut it = raw[ i.. ].char_indices();
                let (off, c) = it.next().unwrap();
                (i + off, c)
            };
            if ch == '\\' {
                // decode escape into literal buffer
                let mut it = raw[ ci + ch.len_utf8() .. ].char_indices();
                let next = it.next().map(|(_,c)| c);
                push_escape(&mut literal_buf, next, &mut it);
                // advance i past backslash and the consumed char(s)
                // Compute advancement: one for '\\' and one for the immediate next char; optional third for '{' if present
                let mut adv = ch.len_utf8();
                if let Some(nc) = next { adv += nc.len_utf8(); if nc == '#' { if raw[ ci + adv .. ].starts_with("{") { adv += '{'.len_utf8(); } } }
                i = ci + adv;
                continue;
            }
            if ch == '#' {
                // possible interpolation start
                let after_hash = ci + ch.len_utf8();
                if raw[after_hash..].starts_with("{") {
                    // start of interpolation
                    saw_interpolation = true;
                    // flush current literal
                    if need_plus { built.push(Token { kind: TokenKind::Plus, lexeme: "+".into(), literal: None, span: Span::new(outer_start, self.pos), line: tok_line }); }
                    need_plus = true;
                    let lit = std::mem::take(&mut literal_buf);
                    built.push(Token { kind: TokenKind::String, lexeme: lit.clone(), literal: Some(Literal::Str(lit)), span: Span::new(outer_start, self.pos), line: tok_line });
                    built.push(Token { kind: TokenKind::Plus, lexeme: "+".into(), literal: None, span: Span::new(outer_start, self.pos), line: tok_line });

                    // scan inner expression in raw starting at after '{'
                    let mut j = after_hash + '{'.len_utf8();
                    let mut depth: usize = 1;
                    let mut in_str = false;
                    let mut expr_end_opt: Option<usize> = None;
                    while j < raw.len() {
                        let (cj, ch2) = {
                            let mut it2 = raw[j..].char_indices();
                            let (off, c) = it2.next().unwrap();
                            (j + off, c)
                        };
                        if in_str {
                            if ch2 == '\\' {
                                // skip escaped char inside inner string
                                let mut it3 = raw[cj + ch2.len_utf8() ..].char_indices();
                                if let Some((_, _)) = it3.next() { /* skip next char */ }
                                j = cj + ch2.len_utf8();
                                if let Some((off,_)) = raw[j..].char_indices().next() { j += off; }
                                continue;
                            } else if ch2 == '"' {
                                in_str = false;
                                j = cj + ch2.len_utf8();
                                continue;
                            } else {
                                j = cj + ch2.len_utf8();
                                continue;
                            }
                        } else {
                            match ch2 {
                                '"' => { in_str = true; j = cj + ch2.len_utf8(); }
                                '{' => { depth += 1; j = cj + ch2.len_utf8(); }
                                '}' => {
                                    depth -= 1;
                                    if depth == 0 { expr_end_opt = Some(cj); j = cj + ch2.len_utf8(); break; }
                                    j = cj + ch2.len_utf8();
                                }
                                _ => { j = cj + ch2.len_utf8(); }
                            }
                        }
                    }
                    let expr_end = match expr_end_opt { Some(p) => p, None => {
                        return Err(BasilError(format!("Unterminated interpolation: missing '}}' after '#{{' at line {}.", tok_line)));
                    } };
                    let expr_src = &raw[ after_hash + '{'.len_utf8() .. expr_end ];
                    if expr_src.trim().is_empty() {
                        return Err(BasilError(format!("Empty interpolation not allowed: expected expression after '#{{' at line {}.", tok_line)));
                    }
                    // Tokenize inner expression and wrap in parentheses
                    let mut sub = Lexer::new(expr_src);
                    let mut inner = sub.tokenize()?;
                    inner.retain(|t| t.kind != TokenKind::Eof && t.kind != TokenKind::Semicolon);
                    built.push(Token { kind: TokenKind::LParen, lexeme: "(".into(), literal: None, span: Span::new(outer_start, self.pos), line: tok_line });
                    for mut t in inner { t.line = tok_line; built.push(t); }
                    built.push(Token { kind: TokenKind::RParen, lexeme: ")".into(), literal: None, span: Span::new(outer_start, self.pos), line: tok_line });
                    // advance i to j (position just after the closing '}')
                    i = j;
                    continue;
                }
            }
            // regular char for literal
            literal_buf.push(ch);
            i = ci + ch.len_utf8();
        }

        if saw_interpolation {
            // flush tail literal
            if need_plus { built.push(Token { kind: TokenKind::Plus, lexeme: "+".into(), literal: None, span: Span::new(outer_start, self.pos), line: tok_line }); }
            let tail = std::mem::take(&mut literal_buf);
            built.push(Token { kind: TokenKind::String, lexeme: tail.clone(), literal: Some(Literal::Str(tail)), span: Span::new(outer_start, self.pos), line: tok_line });
            for t in built.drain(..) { self.pending.push_back(t); }
            if let Some(tok) = self.pending.pop_front() { return Ok(tok); }
            unreachable!("pending should have at least one token");
        } else {
            // simple string token (no interpolation)
            let tok = Token { kind: TokenKind::String, lexeme: self.src[outer_start..self.pos].to_string(), literal: Some(Literal::Str(literal_buf)), span: Span::new(outer_start, self.pos), line: tok_line };
            return Ok(tok);
        }
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
            "FUNCTION" => TokenKind::Func,
            "SUB"    => TokenKind::Func,
            "RETURN" => TokenKind::Return,
            "IF"     => TokenKind::If,
            "THEN"   => TokenKind::Then,
            "ELSE"   => TokenKind::Else,
            "WHILE"  => TokenKind::While,
            "DO"     => TokenKind::Do,
            "BEGIN"  => TokenKind::Begin,
            "END"    => TokenKind::End,
            "ENDIF"  => TokenKind::End,
            "ENDFUNC" => TokenKind::End,
            "ENDFUNCTION" => TokenKind::End,
            "ENDSUB" => TokenKind::End,
            "ENDWHILE" => TokenKind::End,
            "ENDBLOCK" => TokenKind::End,
            "SELECT" => TokenKind::Select,
            "CASE"   => TokenKind::Case,
            "IS"     => TokenKind::Is,
            "BREAK"  => TokenKind::Break,
            "CONTINUE" => TokenKind::Continue,
            "LET"    => TokenKind::Let,
            "PRINT"  => TokenKind::Print,
            "PRINTLN"=> TokenKind::Println,
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
            "EACH"   => TokenKind::Each,
            "IN"     => TokenKind::In,
            "FOREACH"=> TokenKind::Foreach,
            "ENDFOR" => TokenKind::Endfor,
            "DIM"    => TokenKind::Dim,
            "AS"     => TokenKind::As,
            "DESCRIBE" => TokenKind::Describe,
            "NEW"    => TokenKind::New,
            "CLASS"  => TokenKind::Class,
            "WITH"   => TokenKind::With,
            "TRY"    => TokenKind::Try,
            "CATCH"  => TokenKind::Catch,
            "FINALLY"=> TokenKind::Finally,
            "RAISE"  => TokenKind::Raise,
            "SETENV" => TokenKind::Setenv,
            "EXPORTENV" => TokenKind::Exportenv,
            "SHELL"  => TokenKind::Shell,
            "EXIT"   => TokenKind::Exit,
            "STOP"   => TokenKind::Stop,
            "LABEL"  => TokenKind::Label,
            "GOTO"   => TokenKind::Goto,
            "GOSUB"  => TokenKind::Gosub,
            "MOD"    => TokenKind::Mod,
            "EXEC"   => TokenKind::Exec,
            "EVAL"   => TokenKind::Eval,
            "TYPE"   => TokenKind::Type,
            _        => TokenKind::Ident,
        };

        // Explicit line continuation: a single '_' followed by optional spaces/comments to end-of-line
        if matches!(kind, TokenKind::Ident) && lex == "_" {
            if self.consume_explicit_continuation_after_underscore() {
                // Return the next real token instead of the '_' token
                return self.next_token();
            }
        }

        // Support colon-form labels: IDENT ':' -> Label token with ident as lexeme
        if matches!(kind, TokenKind::Ident) && self.cur == Some(':') {
            // consume ':' and emit a Label token whose lexeme is the identifier
            self.advance();
            return Ok(self.make_with_span(TokenKind::Label, start, end));
        }
        Ok(self.make_with_span(kind, start, end))
    }


    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.cur {
                Some(c) if c.is_whitespace() => {
                    if c == '\n' {
                        // implicit continuation rules
                        let suppress = self.paren_depth > 0
                            || self.last_was_continuation
                            || self.next_after_nl_is_cont_op();
                        if !suppress { self.pending_nl_semi = true; }
                    }
                    self.advance();
                }

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

                // Preprocessor-like directives starting with '#': treat as comment line (e.g., #USE ...)
                Some('#') => {
                    while let Some(ch) = self.cur {
                        if ch == '\n' { break; }
                        self.advance();
                    }
                }

                // BASIC-style REM comment (case-insensitive): skip 'REM' and rest of line
                Some('R') | Some('r') => {
                    let mut it = self.chars.clone();
                    let n1 = it.next();
                    let n2 = it.next();
                    if matches!(n1, Some('E') | Some('e')) && matches!(n2, Some('M') | Some('m')) {
                        // consume R E M
                        self.advance(); self.advance(); self.advance();
                        while let Some(ch) = self.cur {
                            if ch == '\n' { break; }
                            self.advance();
                        }
                    } else {
                        break;
                    }
                }

                _ => break,
            }
        }
    }


    fn advance(&mut self) {
        self.cur = self.chars.next();
        if let Some(c) = self.cur {
            if c == '\n' { self.line += 1; }
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
fn is_ident_continue(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '%' || c == '@' || c == '&' }
