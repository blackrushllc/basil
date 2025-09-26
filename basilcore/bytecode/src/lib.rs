//! Bytecode chunk + opcodes + simple Value for Basil v0
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Num(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    ConstU8    = 1,
    LoadGlobal = 2,
    StoreGlobal= 3,
    Add        = 4,
    Sub        = 5,
    Mul        = 6,
    Div        = 7,
    Neg        = 8,
    Print      = 9,
    Pop        = 10,
    Halt       = 255,
}

#[derive(Debug, Default, Clone)]
pub struct Chunk {
    pub code:   Vec<u8>,
    pub consts: Vec<Value>,
}

impl Chunk {
    pub fn push_op(&mut self, op: Op) {
        self.code.push(op as u8);
    }
    pub fn push_u8(&mut self, b: u8) {
        self.code.push(b);
    }
    pub fn add_const(&mut self, v: Value) -> u8 {
        self.consts.push(v);
        (self.consts.len() - 1) as u8
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub chunk:   Chunk,
    pub globals: Vec<String>,
}
