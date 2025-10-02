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

//! Bytecode + values + function object + helpers (u16 jumps)
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Int(i64),
    Str(String),
    Func(Rc<Function>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Num(a),  Value::Num(b))  => a == b,
            (Value::Int(a),  Value::Int(b))  => a == b,
            (Value::Str(a),  Value::Str(b))  => a == b,
            (Value::Func(a), Value::Func(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for Value {}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Num(n)  => write!(f, "{n}"),
            Value::Int(i)  => write!(f, "{i}"),
            Value::Str(s)  => write!(f, "{s}"),
            Value::Func(fun) => write!(f, "<func {} /{}>", fun.name.as_deref().unwrap_or("_"), fun.arity),
        }
    }
}


#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    // constants / globals
    ConstU8    = 1,
    LoadGlobal = 2,
    StoreGlobal= 3,

    // locals
    LoadLocal  = 11,
    StoreLocal = 12,

    // arithmetic
    Add = 20, Sub = 21, Mul = 22, Div = 23, Neg = 24,

    // comparisons
    Eq = 30, Ne = 31, Lt = 32, Le = 33, Gt = 34, Ge = 35,

    // control flow
    Jump = 40,           // +u16
    JumpIfFalse = 41,    // +u16
    JumpBack = 42,       // +u16 (ip -= off)

    // calls
    Call = 50,           // +u8 (argc)
    Ret  = 51,

    // misc
    Print = 60,
    Pop   = 61,
    ToInt = 62,
    Builtin = 63,       // +u8 (builtin id), +u8 (argc)
    Halt  = 255,
}

#[derive(Debug, Default, Clone)]
pub struct Chunk {
    pub code:   Vec<u8>,
    pub consts: Vec<Value>,
}

impl Chunk {
    pub fn push_op(&mut self, op: Op) { self.code.push(op as u8); }
    pub fn push_u8(&mut self, b: u8)  { self.code.push(b); }
    pub fn add_const(&mut self, v: Value) -> u8 { self.consts.push(v); (self.consts.len() - 1) as u8 }

    // u16 (little-endian) helpers for jumps
    pub fn push_u16(&mut self, n: u16) {
        self.code.push((n & 0x00FF) as u8);
        self.code.push((n >> 8) as u8);
    }
    pub fn emit_u16_placeholder(&mut self) -> usize {
        let at = self.code.len();
        self.code.push(0); self.code.push(0);
        at
    }
    pub fn patch_u16_at(&mut self, at: usize, val: u16) {
        self.code[at]   = (val & 0x00FF) as u8;
        self.code[at+1] = (val >> 8) as u8;
    }
    pub fn here(&self) -> usize { self.code.len() }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub arity: u8,
    pub name: Option<String>,
    pub chunk: Rc<Chunk>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub chunk:   Chunk,        // top-level code
    pub globals: Vec<String>,  // names → indices for global array
}
