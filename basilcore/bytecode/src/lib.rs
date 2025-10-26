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

//! Bytecode + values + function object + helpers (u16 jumps)
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use basil_common::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElemType { Num, Int, Str, Obj(Option<String>) }

#[derive(Debug)]
pub struct ArrayObj {
    pub elem: ElemType,
    pub dims: Vec<usize>, // lengths per dimension (0-based indices, inclusive upper bound yields length upper+1)
    pub data: RefCell<Vec<Value>>,
}

// --- Object system core types ---

#[derive(Debug, Clone)]
pub struct PropDesc {
    pub name: String,
    pub type_name: String,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Debug, Clone)]
pub struct MethodDesc {
    pub name: String,
    pub arity: u8,
    pub arg_names: Vec<String>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct ObjectDescriptor {
    pub type_name: String,
    pub version: String,
    pub summary: String,
    pub properties: Vec<PropDesc>,
    pub methods: Vec<MethodDesc>,
    pub examples: Vec<String>,
}

pub trait BasicObject {
    fn type_name(&self) -> &str;
    fn get_prop(&self, name: &str) -> Result<Value>;
    fn set_prop(&mut self, name: &str, v: Value) -> Result<()>;
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value>;
    fn descriptor(&self) -> ObjectDescriptor;
}

pub type ObjectRef = Rc<RefCell<dyn BasicObject>>;

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Int(i64),
    Str(String),
    Func(Rc<Function>),
    Array(Rc<ArrayObj>),
    Object(ObjectRef),
    // Dynamic containers
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    // Special runtime-only value: 2-D string array, row-major order.
    // Used as RHS for whole-array assignment (auto-redimensioning target array).
    StrArray2D { rows: usize, cols: usize, data: Vec<String> },
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
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::List(a), Value::List(b)) => {
                let av = a.borrow();
                let bv = b.borrow();
                if av.len() != bv.len() { return false; }
                for (i, v) in av.iter().enumerate() { if *v != bv[i] { return false; } }
                true
            }
            (Value::Dict(a), Value::Dict(b)) => {
                let av = a.borrow();
                let bv = b.borrow();
                if av.len() != bv.len() { return false; }
                av.iter().all(|(k, v)| bv.get(k).map(|w| v == w).unwrap_or(false))
            }
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
            Value::Array(arr_rc) => {
                // show like <array Num 10x20>
                let arr = arr_rc.as_ref();
                let et = match &arr.elem { ElemType::Num => "Num".to_string(), ElemType::Int => "Int".to_string(), ElemType::Str => "Str".to_string(), ElemType::Obj(Some(t)) => t.clone(), ElemType::Obj(None) => "OBJECT".to_string() };
                let dims = if arr.dims.is_empty() { "".to_string() } else { arr.dims.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("x") };
                write!(f, "<array {} {}>", et, dims)
            }
            Value::Object(obj_rc) => {
                let obj = obj_rc.borrow();
                write!(f, "<{}>", obj.type_name())
            }
            Value::List(items) => {
                write!(f, "[")?;
                let v = items.borrow();
                for (i, it) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", it)?;
                }
                write!(f, "]")
            }
            Value::Dict(map) => {
                write!(f, "{{")?;
                let mut first = true;
                let m = map.borrow();
                for (k, v) in m.iter() {
                    if !first { write!(f, ", ")?; } else { first = false; }
                    // keys are strings; print quoted for clarity
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::StrArray2D { rows, cols, .. } => {
                write!(f, "<StrArray2D {}x{}>", rows, cols)
            }
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Num(n) => write!(f, "Num({n})"),
            Value::Int(i) => write!(f, "Int({i})"),
            Value::Str(s) => write!(f, "Str(\"{}\")", s),
            Value::Func(fun) => write!(f, "Func(name={:?}, arity={})", fun.name, fun.arity),
            Value::Array(_) => write!(f, "Array(..)"),
            Value::Object(obj_rc) => {
                let obj = obj_rc.borrow();
                write!(f, "Object(<{}>)", obj.type_name())
            }
            Value::List(items) => {
                write!(f, "List(")?;
                let v = items.borrow();
                for (i, it) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{:?}", it)?;
                }
                write!(f, ")")
            }
            Value::Dict(map) => {
                write!(f, "Dict{{")?;
                let mut first = true;
                let m = map.borrow();
                for (k, v) in m.iter() {
                    if !first { write!(f, ", ")?; } else { first = false; }
                    write!(f, "{}: {:?}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::StrArray2D { rows, cols, data } => {
                write!(f, "StrArray2D(rows={}, cols={}, data_len={})", rows, cols, data.len())
            }
        }
    }
}


#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    // constants / globals
    Const    = 1,
    LoadGlobal = 2,
    StoreGlobal= 3,

    // locals
    LoadLocal  = 11,
    StoreLocal = 12,

    // arithmetic
    Add = 20, Sub = 21, Mul = 22, Div = 23, Neg = 24, Mod = 25,

    // comparisons
    Eq = 30, Ne = 31, Lt = 32, Le = 33, Gt = 34, Ge = 35,

    // control flow
    Jump = 40,           // +u16
    JumpIfFalse = 41,    // +u16
    JumpBack = 42,       // +u16 (ip -= off)
    // gosub control flow
    Gosub = 110,         // +u16 (push return ip; ip += off or ip -= off depending on opcode variant)
    GosubBack = 111,     // +u16 (ip -= off; push return ip)
    GosubRet = 112,      // pop return ip into ip; error if empty
    GosubPop = 113,      // pop and discard return ip; error if empty

    // calls
    Call = 50,           // +u8 (argc)
    Ret  = 51,

    // misc
    Print = 60,
    Pop   = 61,
    ToInt = 62,
    Builtin = 63,       // +u8 (builtin id), +u8 (argc)
    SetLine = 64,       // +u16 (line number)

    // arrays
    ArrMake = 70,       // +u8 (rank), +u8 (elemType: 0=Num,1=Int,2=Str,3=Object), +u8 (type-name const idx or 255 if none); then pops rank dims (upper bounds)
    ArrGet  = 71,       // +u8 (rank) -- stack: [..., array, i0, i1, ...] -> push elem
    ArrSet  = 72,       // +u8 (rank) -- stack: [..., array, i0, i1, ..., value] -> (store) no push

    // objects (string-based slow path for names/types)
    NewObj      = 80,   // +u8 (const index of type name), +u8 (argc). Stack: [..., args...] -> push object
    GetProp     = 81,   // +u8 (const index of property name). Stack: [..., obj] -> push value
    SetProp     = 82,   // +u8 (const index of property name). Stack: [..., obj, value] -> (store)
    CallMethod  = 83,   // +u8 (const index of method name), +u8 (argc). Stack: [..., obj, args...] -> push ret
    DescribeObj = 84,   // no extra. Stack: [..., obj or array] -> push string

    // classes
    NewClass        = 100, // pop filename (string) → push instance (object)
    GetMember       = 101, // alias of GetProp
    SetMember       = 102, // alias of SetProp
    CallMember      = 103, // alias of CallMethod
    DestroyInstance = 104, // hint GC/no-op for now

    // dynamic code execution
    ExecString      = 105, // pop string: Basil statements; parse+compile+run (no value pushed)
    EvalString      = 106, // pop string: Basil expression; parse+compile+run; push value

    // enumeration
    EnumNew      = 90,  // expects iterable (array or object) on stack; pushes enumerator handle (object) or error
    EnumMoveNext = 91,  // moves enumerator; pushes Bool
    EnumCurrent  = 92,  // pushes current element Value
    EnumDispose  = 93,  // best-effort cleanup

    // exceptions
    TryPush = 120,      // +u16 (handler off), +u16 (finally off or 0)
    TryPop  = 121,      // no extra
    Raise   = 122,      // expects message (any value) on stack; converts to string and raises
    Reraise = 123,      // rethrow current exception

    // suspension
    Stop   = 124,       // suspend execution

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
    pub fn add_const(&mut self, v: Value) -> u16 { self.consts.push(v); (self.consts.len() - 1) as u16 }

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

// --- Simple (de)serializer for Program used by .basilx cache ---
pub fn serialize_program(p: &Program) -> Vec<u8> {
    fn w_u8(b: &mut Vec<u8>, v: u8) { b.push(v); }
    fn w_u32(b: &mut Vec<u8>, v: u32) { b.extend_from_slice(&v.to_le_bytes()); }
    fn w_str(b: &mut Vec<u8>, s: &str) { w_u32(b, s.len() as u32); b.extend_from_slice(s.as_bytes()); }
    fn ser_value(b: &mut Vec<u8>, v: &Value) {
        match v {
            Value::Null => { w_u8(b,0); }
            Value::Bool(x) => { w_u8(b,1); w_u8(b, if *x {1} else {0}); }
            Value::Num(n) => { w_u8(b,2); b.extend_from_slice(&n.to_le_bytes()); }
            Value::Int(i) => { w_u8(b,3); b.extend_from_slice(&i.to_le_bytes()); }
            Value::Str(s) => { w_u8(b,4); w_str(b, s); }
            Value::Func(f) => {
                w_u8(b,5);
                w_u8(b, f.arity);
                match &f.name { Some(n)=>{ w_u8(b,1); w_str(b,n); }, None=>{ w_u8(b,0); } }
                ser_chunk(b, &f.chunk);
            }
            Value::Array(_) => { w_u8(b,250); } // unsupported in consts
            Value::Object(_) => { w_u8(b,251); }
            Value::StrArray2D { .. } => { w_u8(b,252); }
            Value::List(_) => { w_u8(b,253); }
            Value::Dict(_) => { w_u8(b,254); }
        }
    }
    fn ser_chunk(b: &mut Vec<u8>, c: &Chunk) {
        w_u32(b, c.code.len() as u32); b.extend_from_slice(&c.code);
        w_u32(b, c.consts.len() as u32);
        for v in &c.consts { ser_value(b, v); }
    }
    let mut b = Vec::new();
    ser_chunk(&mut b, &p.chunk);
    w_u32(&mut b, p.globals.len() as u32);
    for g in &p.globals { w_str(&mut b, g); }
    b
}

pub fn deserialize_program(data: &[u8]) -> basil_common::Result<Program> {
    use basil_common::{Result, BasilError};
    fn r_u8(p: &mut usize, data: &[u8]) -> Result<u8> { if *p >= data.len() { return Err(BasilError("eof".into())); } let v=data[*p]; *p+=1; Ok(v) }
    fn r_u32(p: &mut usize, data: &[u8]) -> Result<u32> { if *p+4>data.len(){return Err(BasilError("eof".into()));} let v = u32::from_le_bytes([data[*p],data[*p+1],data[*p+2],data[*p+3]]); *p+=4; Ok(v) }
    fn r_f64(p: &mut usize, data: &[u8]) -> Result<f64> { if *p+8>data.len(){return Err(BasilError("eof".into()));} let mut a=[0u8;8]; a.copy_from_slice(&data[*p..*p+8]); *p+=8; Ok(f64::from_le_bytes(a)) }
    fn r_i64(p: &mut usize, data: &[u8]) -> Result<i64> { if *p+8>data.len(){return Err(BasilError("eof".into()));} let mut a=[0u8;8]; a.copy_from_slice(&data[*p..*p+8]); *p+=8; Ok(i64::from_le_bytes(a)) }
    fn r_str(p: &mut usize, data: &[u8]) -> Result<String> { let n = r_u32(p,data)? as usize; if *p+n>data.len(){return Err(BasilError("eof".into()));} let s = String::from_utf8(data[*p..*p+n].to_vec()).map_err(|e| BasilError(format!("utf8: {}", e)))?; *p+=n; Ok(s) }
    fn de_chunk(p: &mut usize, data: &[u8]) -> Result<Chunk> {
        let code_len = r_u32(p,data)? as usize; if *p+code_len>data.len(){return Err(BasilError("eof".into()));}
        let code = data[*p..*p+code_len].to_vec(); *p+=code_len;
        let nconst = r_u32(p,data)? as usize; let mut consts = Vec::with_capacity(nconst);
        for _ in 0..nconst { consts.push(de_value(p,data)?); }
        Ok(Chunk { code, consts })
    }
    fn de_value(p: &mut usize, data: &[u8]) -> Result<Value> {
        use std::rc::Rc;
        let tag = r_u8(p,data)?;
        Ok(match tag {
            0 => Value::Null,
            1 => { let b = r_u8(p,data)? != 0; Value::Bool(b) },
            2 => { let n = r_f64(p,data)?; Value::Num(n) },
            3 => { let i = r_i64(p,data)?; Value::Int(i) },
            4 => { let s = r_str(p,data)?; Value::Str(s) },
            5 => {
                let ar = r_u8(p,data)?;
                let has = r_u8(p,data)? != 0;
                let name = if has { Some(r_str(p,data)?) } else { None };
                let chunk = de_chunk(p,data)?;
                Value::Func(Rc::new(Function { arity: ar, name, chunk: std::rc::Rc::new(chunk) }))
            }
            250|251|252 => Value::Null, // placeholder for unsupported in consts
            253|254 => Value::Null,
            _ => return Err(BasilError("bad const tag".into())),
        })
    }
    let mut p = 0usize;
    let chunk = de_chunk(&mut p, data)?;
    let n = r_u32(&mut p,data)? as usize; let mut globals = Vec::with_capacity(n);
    for _ in 0..n { globals.push(r_str(&mut p,data)?); }
    Ok(Program { chunk, globals })
}
