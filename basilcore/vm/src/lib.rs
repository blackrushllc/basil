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

//! Frame-based VM with calls, locals, jumps, comparisons
use std::rc::Rc;
use std::io::{self, Write, Read};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use std::env;
use std::time::Duration;
use crossterm::event::{read, Event, KeyEvent, KeyCode};
use crossterm::event::poll;

use basil_common::{Result, BasilError};
use basil_bytecode::{Program as BCProgram, Chunk, Value, Op, ElemType, ArrayObj};
use basil_objects::{Registry, register_objects};

struct Frame {
    chunk: Rc<Chunk>,
    ip: usize,
    base: usize,
}

struct ArrEnum {
    arr: Rc<ArrayObj>,
    cur: isize,    // -1 before first element
    total: usize,  // total elements
}

pub struct VM {
    frames: Vec<Frame>,
    stack: Vec<Value>,
    globals: Vec<Value>,
    registry: Registry,
    enums: Vec<ArrEnum>,
    // Caches for CGI params
    get_params_cache: Option<Vec<String>>,    // name=value pairs from QUERY_STRING
    post_params_cache: Option<Vec<String>>,   // name=value pairs from stdin (x-www-form-urlencoded)
}

impl VM {
    pub fn new(p: BCProgram) -> Self {
        let globals = vec![Value::Null; p.globals.len()];
        let top_chunk = Rc::new(p.chunk);
        let frame = Frame { chunk: top_chunk, ip: 0, base: 0 };
        let mut registry = Registry::new();
        register_objects(&mut registry);
        Self { frames: vec![frame], stack: Vec::new(), globals, registry, enums: Vec::new(), get_params_cache: None, post_params_cache: None }
    }

    fn cur(&mut self) -> &mut Frame { self.frames.last_mut().expect("no frame") }

    // --- CGI param helpers ---
    fn url_decode_form(&self, s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = String::with_capacity(bytes.len());
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => { out.push(' '); i += 1; }
                b'%' if i + 2 < bytes.len() => {
                    let h1 = bytes[i+1] as char; let h2 = bytes[i+2] as char;
                    let hex = [h1, h2];
                    let hv = u8::from_str_radix(&hex.iter().collect::<String>(), 16).ok();
                    if let Some(b) = hv { out.push(b as char); i += 3; } else { out.push('%'); i += 1; }
                }
                b => { out.push(b as char); i += 1; }
            }
        }
        out
    }
    fn parse_pairs(&self, s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for part in s.split('&') {
            if part.is_empty() { continue; }
            let mut it = part.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            let kd = self.url_decode_form(k);
            let vd = self.url_decode_form(v);
            out.push(format!("{}={}", kd, vd));
        }
        out
    }
    fn ensure_get_params(&mut self) {
        if self.get_params_cache.is_none() {
            let q = env::var("QUERY_STRING").unwrap_or_default();
            let v = self.parse_pairs(&q);
            self.get_params_cache = Some(v);
        }
    }
    fn ensure_post_params(&mut self) {
        if self.post_params_cache.is_some() { return; }
        let clen: usize = env::var("CONTENT_LENGTH").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        if clen == 0 { self.post_params_cache = Some(Vec::new()); return; }
        let ctype = env::var("CONTENT_TYPE").unwrap_or_default();
        if !ctype.to_ascii_lowercase().starts_with("application/x-www-form-urlencoded") {
            // unsupported type for now
            self.post_params_cache = Some(Vec::new());
            return;
        }
        let mut body = Vec::with_capacity(clen);
        let _ = io::stdin().take(clen as u64).read_to_end(&mut body);
        let s = String::from_utf8_lossy(&body).to_string();
        let v = self.parse_pairs(&s);
        self.post_params_cache = Some(v);
    }
    fn make_string_array(vals: Vec<String>) -> Value {
        use std::cell::RefCell;
        let dims = vec![vals.len()];
        let data: Vec<Value> = vals.into_iter().map(Value::Str).collect();
        let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
        Value::Array(arr)
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let op = self.read_op()?;
            match op {
                Op::ConstU8 => {
                    let i = self.read_u8()? as usize;
                    let v = self.cur().chunk.consts[i].clone();
                    self.stack.push(v);
                }
                Op::LoadGlobal => {
                    let i = self.read_u8()? as usize;
                    let v = self.globals[i].clone();
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let i = self.read_u8()? as usize;
                    let v = self.pop()?;
                    self.globals[i] = v;
                }

                Op::LoadLocal => {
                    let i = self.read_u8()? as usize;
                    let base = self.cur().base;
                    let v = self.stack[base + i].clone();
                    self.stack.push(v);
                }
                Op::StoreLocal => {
                    let i = self.read_u8()? as usize;
                    let v = self.pop()?;
                    let base = self.cur().base;
                    while self.stack.len() <= base + i { self.stack.push(Value::Null); }
                    self.stack[base + i] = v;
                }

                Op::Add => {
                    let rb = self.pop()?;
                    let lb = self.pop()?;
                    match (&lb, &rb) {
                        (Value::Str(_), _) | (_, Value::Str(_)) => {
                            let ls = format!("{}", lb);
                            let rs = format!("{}", rb);
                            self.stack.push(Value::Str(format!("{}{}", ls, rs)));
                        }
                        _ => {
                            // numeric addition (use existing numeric coercion)
                            let a = self.as_num(lb)?;
                            let b = self.as_num(rb)?;
                            self.stack.push(Value::Num(a + b));
                        }
                    }
                },
                Op::Sub => self.bin_num(|a,b| a-b)?,
                Op::Mul => self.bin_num(|a,b| a*b)?,
                Op::Div => self.bin_num(|a,b| a/b)?,
                Op::Neg => {
                    let v = self.pop()?;
                    let n = self.as_num(v)?;
                    self.stack.push(Value::Num(-n));
                }

                Op::Eq => self.bin_cmp(|a,b| a==b)?,
                Op::Ne => self.bin_cmp(|a,b| a!=b)?,
                Op::Lt => self.bin_num_cmp(|a,b| a<b)?,
                Op::Le => self.bin_num_cmp(|a,b| a<=b)?,
                Op::Gt => self.bin_num_cmp(|a,b| a>b)?,
                Op::Ge => self.bin_num_cmp(|a,b| a>=b)?,

                Op::Jump => {
                    let off = self.read_u16()? as usize;
                    self.cur().ip += off;
                }
                Op::JumpIfFalse => {
                    let off = self.read_u16()? as usize;
                    let cond = self.pop()?;
                    if !is_truthy(&cond) { self.cur().ip += off; }
                }
                Op::JumpBack => {
                    let off = self.read_u16()? as usize;
                    self.cur().ip -= off;
                }

                Op::Call => {
                    let argc = self.read_u8()? as usize;
                    let callee_idx = self.stack.len() - 1 - argc;
                    let callee = self.stack.remove(callee_idx);
                    let base = callee_idx;
                    match callee {
                        Value::Func(f) => {
                            if f.arity as usize != argc {
                                return Err(BasilError(format!("arity mismatch: expected {}, got {}", f.arity, argc)));
                            }
                            let frame = Frame { chunk: f.chunk.clone(), ip: 0, base };
                            self.frames.push(frame);
                        }
                        _ => return Err(BasilError("CALL target is not a function".into())),
                    }
                }

                Op::Ret => {
                    let retv = self.pop().unwrap_or(Value::Null);
                    let frame = self.frames.pop().ok_or_else(|| BasilError("RET with no frame".into()))?;
                    self.stack.truncate(frame.base);
                    self.stack.push(retv);
                    if self.frames.is_empty() { break; }
                }

                Op::Print => { let v = self.pop()?; print!("{}", v); }
                Op::Pop   => { let _ = self.pop()?; }
                Op::ToInt => {
                    let v = self.pop()?;
                    match v {
                        Value::Int(i) => self.stack.push(Value::Int(i)),
                        Value::Num(n) => self.stack.push(Value::Int(n.trunc() as i64)),
                        _ => return Err(BasilError("ToInt expects a numeric value".into())),
                    }
                }

                Op::ArrMake => {
                    let rank = self.read_u8()? as usize;
                    let et_code = self.read_u8()? as u8;
                    let type_cidx = self.read_u8()? as usize; // may be 255 if not applicable
                    let elem = match et_code {
                        0 => ElemType::Num,
                        1 => ElemType::Int,
                        2 => ElemType::Str,
                        3 => {
                            if type_cidx == 255 {
                                ElemType::Obj(None)
                            } else {
                                let tn_v = self.cur().chunk.consts[type_cidx].clone();
                                let tn = match tn_v { Value::Str(s) => s, _ => return Err(BasilError("ArrMake type expects string const".into())) };
                                ElemType::Obj(Some(tn))
                            }
                        }
                        _ => return Err(BasilError("bad elem type".into())),
                    };
                    if rank == 0 || rank > 4 { return Err(BasilError("array rank must be 1..4".into())); }
                    let mut uppers: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array dimension must be numeric".into())) };
                        uppers.push(n);
                    }
                    uppers.reverse();
                    let mut dims: Vec<usize> = Vec::with_capacity(rank);
                    let mut total: usize = 1;
                    for u in uppers {
                        if u < 0 { return Err(BasilError("array dimension upper bound must be >= 0".into())); }
                        let len = (u as usize) + 1;
                        dims.push(len);
                        total = total.saturating_mul(len);
                    }
                    let defv = match &elem {
                        ElemType::Num => Value::Num(0.0),
                        ElemType::Int => Value::Int(0),
                        ElemType::Str => Value::Str(String::new()),
                        ElemType::Obj(_) => Value::Null,
                    };
                    let mut data = Vec::with_capacity(total);
                    data.resize(total, defv);
                    let arr = Rc::new(ArrayObj { elem, dims, data: std::cell::RefCell::new(data) });
                    self.stack.push(Value::Array(arr));
                }

                Op::ArrGet => {
                    let rank = self.read_u8()? as usize;
                    let mut idxs: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array index must be numeric".into())) };
                        idxs.push(n);
                    }
                    idxs.reverse();
                    let arr_v = self.pop()?;
                    let arr_rc = match arr_v { Value::Array(rc) => rc, _ => return Err(BasilError("array access on non-array or not DIMed".into())) };
                    let arr = arr_rc.as_ref();
                    if idxs.len() != arr.dims.len() { return Err(BasilError("array rank mismatch".into())); }
                    for (dim_len, idx) in arr.dims.iter().zip(&idxs) {
                        if *idx < 0 || (*idx as usize) >= *dim_len { return Err(BasilError("array index out of bounds".into())); }
                    }
                    // compute linear index (row-major)
                    let mut lin: usize = 0;
                    let mut stride: usize = 1;
                    for d in 0..arr.dims.len() {
                        let len = arr.dims[arr.dims.len() - 1 - d];
                        let idx = idxs[arr.dims.len() - 1 - d] as usize;
                        if d == 0 { lin = idx; stride = len; } else { lin += idx * stride; stride *= len; }
                    }
                    let val = arr.data.borrow()[lin].clone();
                    self.stack.push(val);
                }

                Op::ArrSet => {
                    let rank = self.read_u8()? as usize;
                    let val = self.pop()?;
                    let mut idxs: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array index must be numeric".into())) };
                        idxs.push(n);
                    }
                    idxs.reverse();
                    let arr_v = self.pop()?;
                    let arr_rc = match arr_v { Value::Array(rc) => rc, _ => return Err(BasilError("array write on non-array or not DIMed".into())) };
                    let arr = arr_rc.as_ref();
                    if idxs.len() != arr.dims.len() { return Err(BasilError("array rank mismatch".into())); }
                    for (dim_len, idx) in arr.dims.iter().zip(&idxs) {
                        if *idx < 0 || (*idx as usize) >= *dim_len { return Err(BasilError("array index out of bounds".into())); }
                    }
                    let mut lin: usize = 0;
                    let mut stride: usize = 1;
                    for d in 0..arr.dims.len() {
                        let len = arr.dims[arr.dims.len() - 1 - d];
                        let idx = idxs[arr.dims.len() - 1 - d] as usize;
                        if d == 0 { lin = idx; stride = len; } else { lin += idx * stride; stride *= len; }
                    }
                    let coerced = match &arr.elem {
                        ElemType::Num => match val { Value::Num(n)=>Value::Num(n), Value::Int(i)=>Value::Num(i as f64), other=>return Err(BasilError(format!("cannot store non-numeric {:?} into numeric array", other))) },
                        ElemType::Int => match val { Value::Int(i)=>Value::Int(i), Value::Num(n)=>Value::Int(n.trunc() as i64), other=>return Err(BasilError(format!("cannot store non-numeric {:?} into integer array", other))) },
                        ElemType::Str => match val { Value::Str(s)=>Value::Str(s), other=>Value::Str(format!("{}", other)) },
                        ElemType::Obj(Some(tname)) => match val {
                            Value::Object(rc) => {
                                let got = rc.borrow().type_name().to_string();
                                if got.eq_ignore_ascii_case(tname) { Value::Object(rc) }
                                else { return Err(BasilError(format!("Expected {} in typed object array, got {}.", tname, got))); }
                            }
                            Value::Null => Value::Null,
                            other => return Err(BasilError(format!("cannot store non-object {:?} into typed OBJECT[] array", other))),
                        },
                        ElemType::Obj(None) => match val {
                            Value::Object(_) | Value::Null => val,
                            other => return Err(BasilError(format!("cannot store non-object {:?} into OBJECT[] array", other))),
                        },
                    };
                    arr.data.borrow_mut()[lin] = coerced;
                }

                // enumeration over arrays
                Op::EnumNew => {
                    let it = self.pop()?;
                    match it {
                        Value::Array(rc) => {
                            let total = rc.dims.iter().copied().fold(1usize, |acc, d| acc.saturating_mul(d));
                            let handle = self.enums.len();
                            self.enums.push(ArrEnum { arr: rc, cur: -1, total });
                            self.stack.push(Value::Int(handle as i64));
                        }
                        Value::Object(_) => {
                            let ty = self.type_of(&it);
                            return Err(BasilError(format!("FOR EACH expects an array or iterable object after IN (got TYPE={}).", ty)));
                        }
                        other => {
                            let ty = self.type_of(&other);
                            return Err(BasilError(format!("FOR EACH expects an array or iterable object after IN (got TYPE={}).", ty)));
                        }
                    }
                }
                Op::EnumMoveNext => {
                    let handle = match self.stack.last() {
                        Some(Value::Int(i)) => *i as usize,
                        _ => return Err(BasilError("ENUM_MOVENEXT requires enumerator handle on stack".into())),
                    };
                    let e = self.enums.get_mut(handle).ok_or_else(|| BasilError("bad enumerator handle".into()))?;
                    if (e.cur + 1) < e.total as isize { e.cur += 1; self.stack.push(Value::Bool(true)); }
                    else { self.stack.push(Value::Bool(false)); }
                }
                Op::EnumCurrent => {
                    let handle = match self.stack.last() {
                        Some(Value::Int(i)) => *i as usize,
                        _ => return Err(BasilError("ENUM_CURRENT requires enumerator handle on stack".into())),
                    };
                    let e = self.enums.get(handle).ok_or_else(|| BasilError("bad enumerator handle".into()))?;
                    if e.cur < 0 { return Err(BasilError("ENUM_CURRENT before first element".into())); }
                    let lin = e.cur as usize;
                    let val = e.arr.data.borrow()[lin].clone();
                    self.stack.push(val);
                }
                Op::EnumDispose => {
                    let h = self.pop()?;
                    match h {
                        Value::Int(_i) => { /* no-op; freed with VM */ }
                        _ => return Err(BasilError("ENUM_DISPOSE expects enumerator handle".into())),
                    }
                }

                // --- Objects ---
                Op::NewObj => {
                    let type_cidx = self.read_u8()? as usize;
                    let argc = self.read_u8()? as usize;
                    let tname_v = self.cur().chunk.consts[type_cidx].clone();
                    let type_name = match tname_v { Value::Str(s) => s, _ => return Err(BasilError("NEW_OBJ expects type name string const".into())) };
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let obj = self.registry.make(&type_name, &args)?;
                    self.stack.push(Value::Object(obj));
                }
                Op::GetProp => {
                    let prop_cidx = self.read_u8()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("GETPROP expects property name string const".into())) };
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow().get_prop(&prop)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("GETPROP on non-object".into())),
                    }
                }
                Op::SetProp => {
                    let prop_cidx = self.read_u8()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("SETPROP expects property name string const".into())) };
                    let val = self.pop()?;
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            rc.borrow_mut().set_prop(&prop, val)?;
                        }
                        _ => return Err(BasilError("SETPROP on non-object".into())),
                    }
                }
                Op::CallMethod => {
                    let meth_cidx = self.read_u8()? as usize;
                    let argc = self.read_u8()? as usize;
                    let mname_v = self.cur().chunk.consts[meth_cidx].clone();
                    let method = match mname_v { Value::Str(s)=>s, _=>return Err(BasilError("CALLMETHOD expects method name string const".into())) };
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow_mut().call(&method, &args)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("CALLMETHOD on non-object".into())),
                    }
                }
                Op::DescribeObj => {
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let desc = rc.borrow().descriptor();
                            // simple formatting
                            let mut s = String::new();
                            s.push_str(&format!("{} — v{}\n{}\n", desc.type_name, desc.version, desc.summary));
                            if !desc.properties.is_empty() {
                                s.push_str("Properties:\n");
                                for p in desc.properties { s.push_str(&format!("  {} : {} {}{}\n", p.name, p.type_name, if p.readable {"R"} else {""}, if p.writable {"W"} else {""})); }
                            }
                            if !desc.methods.is_empty() {
                                s.push_str("Methods:\n");
                                for m in desc.methods { s.push_str(&format!("  {}({}) -> {}\n", m.name, m.arg_names.join(", "), m.return_type)); }
                            }
                            self.stack.push(Value::Str(s));
                        }
                        Value::Array(arr_rc) => {
                            let arr = arr_rc.as_ref();
                            let elem = match &arr.elem {
                                ElemType::Num => "FLOAT".to_string(),
                                ElemType::Int => "INTEGER".to_string(),
                                ElemType::Str => "STRING".to_string(),
                                ElemType::Obj(Some(t)) => t.clone(),
                                ElemType::Obj(None) => "OBJECT".to_string(),
                            };
                            let mut total: usize = 1;
                            for d in &arr.dims { total = total.saturating_mul(*d); }
                            let dims = if arr.dims.is_empty() { "0".to_string() } else { arr.dims.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("x") };
                            let s = format!("Array — elem={}, dims={}, size={} (row-major)", elem, dims, total);
                            self.stack.push(Value::Str(s));
                        }
                        other => return Err(BasilError(format!("DESCRIBE on unsupported value: {}", self.type_of(&other)))),
                    }
                }

                Op::Builtin => {
                    let bid = self.read_u8()? as u8;
                    let argc = self.read_u8()? as usize;
                    // pop args in reverse then reverse to preserve call order
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();

                    match bid {
                        1 => { // LEN(arg)
                            if argc != 1 { return Err(BasilError("LEN expects 1 argument".into())); }
                            match &args[0] {
                                Value::Str(s) => {
                                    let n = s.chars().count() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                                Value::Array(arr_rc) => {
                                    let arr = arr_rc.as_ref();
                                    let mut total: usize = 1;
                                    for d in &arr.dims { total = total.saturating_mul(*d); }
                                    self.stack.push(Value::Int(total as i64));
                                }
                                other => {
                                    // Fallback: coerce to string via Display and count chars
                                    let s = format!("{}", other);
                                    let n = s.chars().count() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                            }
                        }
                        2 => { // MID$(s, start [,len]) -- start is 1-based
                            if !(argc == 2 || argc == 3) { return Err(BasilError("MID$ expects 2 or 3 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("MID$ arg 1 must be string".into())) };
                            // convert numeric args to i64 via truncation
                            let start_i = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("MID$ start must be numeric".into())),
                            };
                            let start_idx0 = if start_i <= 1 { 0usize } else { (start_i as usize) - 1 };
                            let mut iter = s.chars();
                            // drop start_idx0 chars
                            for _ in 0..start_idx0 { if iter.next().is_none() { break; } }
                            let res = if argc == 2 {
                                iter.collect::<String>()
                            } else {
                                let len_i = match &args[2] {
                                    Value::Int(i) => *i,
                                    Value::Num(n) => n.trunc() as i64,
                                    _ => return Err(BasilError("MID$ length must be numeric".into())),
                                };
                                if len_i <= 0 { String::new() } else { iter.take(len_i as usize).collect::<String>() }
                            };
                            self.stack.push(Value::Str(res));
                        }
                        3 => { // LEFT$(s, n)
                            if argc != 2 { return Err(BasilError("LEFT$ expects 2 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("LEFT$ arg 1 must be string".into())) };
                            let n = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("LEFT$ count must be numeric".into())),
                            };
                            if n <= 0 { self.stack.push(Value::Str(String::new())); }
                            else {
                                let res: String = s.chars().take(n as usize).collect();
                                self.stack.push(Value::Str(res));
                            }
                        }
                        4 => { // RIGHT$(s, n)
                            if argc != 2 { return Err(BasilError("RIGHT$ expects 2 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("RIGHT$ arg 1 must be string".into())) };
                            let n = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("RIGHT$ count must be numeric".into())),
                            };
                            if n <= 0 { self.stack.push(Value::Str(String::new())); }
                            else {
                                let total = s.chars().count() as i64;
                                let take_n = if n > total { total } else { n } as usize;
                                let skip_n = (total - take_n as i64) as usize;
                                let res: String = s.chars().skip(skip_n).take(take_n).collect();
                                self.stack.push(Value::Str(res));
                            }
                        }
                        5 => { // INSTR(hay, needle [,start]) -- returns 0-based index or 0 if not found
                            if !(argc == 2 || argc == 3) { return Err(BasilError("INSTR expects 2 or 3 arguments".into())); }
                            let hay = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("INSTR arg 1 must be string".into())) };
                            let needle = match &args[1] { Value::Str(s) => s.clone(), _ => return Err(BasilError("INSTR arg 2 must be string".into())) };
                            let start = if argc == 3 {
                                match &args[2] { Value::Int(i) => *i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("INSTR start must be numeric".into())) }
                            } else { 0 };
                            if needle.is_empty() {
                                // empty needle: return start (already 0-based)
                                let start_nonneg = if start < 0 { 0 } else { start as usize };
                                let total_chars = hay.chars().count();
                                let idx = if start_nonneg > total_chars { total_chars } else { start_nonneg };
                                self.stack.push(Value::Int(idx as i64));
                            } else {
                                // map start (char) to byte index
                                let start_nonneg = if start < 0 { 0 } else { start as usize };
                                let mut byte_idx = 0usize;
                                let mut seen = 0usize;
                                for (bidx, _) in hay.char_indices() {
                                    if seen == start_nonneg { byte_idx = bidx; break; }
                                    seen += 1;
                                    byte_idx = hay.len();
                                }
                                if start_nonneg == 0 { byte_idx = 0; }
                                if start_nonneg > hay.chars().count() { self.stack.push(Value::Int(0)); }
                                else {
                                    let slice = &hay[byte_idx..];
                                    if let Some(rel) = slice.find(&needle) {
                                        let abs_byte = byte_idx + rel;
                                        // convert abs_byte to char index
                                        let idx = hay[..abs_byte].chars().count();
                                        self.stack.push(Value::Int(idx as i64));
                                    } else {
                                        self.stack.push(Value::Int(0));
                                    }
                                }
                            }
                        }
                        6 => { // INPUT$([prompt])
                            if !(argc == 0 || argc == 1) { return Err(BasilError("INPUT$ expects 0 or 1 argument".into())); }
                            if argc == 1 {
                                let prompt = match &args[0] { Value::Str(s) => s.clone(), other => format!("{}", other) };
                                print!("{}", prompt);
                                let _ = io::stdout().flush();
                            }
                            let mut input = String::new();
                            io::stdin().read_line(&mut input).map_err(|e| BasilError(format!("INPUT$ read error: {}", e)))?;
                            // Trim trailing CR/LF (Windows, Unix)
                            while input.ends_with('\n') || input.ends_with('\r') { input.pop(); }
                            self.stack.push(Value::Str(input));
                        }
                        7 => { // INKEY$() — non-blocking, returns "" if no key available
                            if argc != 0 { return Err(BasilError("INKEY$ expects 0 arguments".into())); }
                            enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                            let s = if poll(Duration::from_millis(0)).map_err(|e| BasilError(format!("poll: {}", e)))? {
                                match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                    Event::Key(KeyEvent { code, .. }) => match code {
                                        KeyCode::Char(c) => c.to_string(),
                                        KeyCode::Enter => "\r".to_string(),
                                        KeyCode::Backspace => "\u{0008}".to_string(),
                                        KeyCode::Tab => "\t".to_string(),
                                        KeyCode::Esc => "\u{001B}".to_string(),
                                        _ => String::new(),
                                    },
                                    _ => String::new(),
                                }
                            } else { String::new() };
                            let _ = disable_raw_mode();
                            self.stack.push(Value::Str(s));
                        }
                        8 => { // INKEY%() — non-blocking, returns 0 if no key available
                            if argc != 0 { return Err(BasilError("INKEY% expects 0 arguments".into())); }
                            enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                            let code_i: i64 = if poll(Duration::from_millis(0)).map_err(|e| BasilError(format!("poll: {}", e)))? {
                                match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                    Event::Key(KeyEvent { code, .. }) => {
                                        match code {
                                            KeyCode::Char(c) => c as i64,
                                            KeyCode::Enter => 13,
                                            KeyCode::Backspace => 8,
                                            KeyCode::Tab => 9,
                                            KeyCode::Esc => 27,
                                            KeyCode::Up => 1000,
                                            KeyCode::Down => 1001,
                                            KeyCode::Left => 1002,
                                            KeyCode::Right => 1003,
                                            KeyCode::Home => 1004,
                                            KeyCode::End => 1005,
                                            KeyCode::PageUp => 1006,
                                            KeyCode::PageDown => 1007,
                                            KeyCode::Insert => 1008,
                                            KeyCode::Delete => 1009,
                                            KeyCode::F(n) => 1100 + n as i64,
                                            _ => 0,
                                        }
                                    }
                                    _ => 0,
                                }
                            } else { 0 };
                            let _ = disable_raw_mode();
                            self.stack.push(Value::Int(code_i));
                        }
                        9 => { // TYPE$(value)
                            if argc != 1 { return Err(BasilError("TYPE$ expects 1 argument".into())); }
                            let s = self.type_of(&args[0]);
                            self.stack.push(Value::Str(s));
                        }
                        10 => { // HTML/HTML$(x)
                            if argc != 1 { return Err(BasilError("HTML expects 1 argument".into())); }
                            let s = format!("{}", args[0]);
                            let mut out = String::with_capacity(s.len());
                            for ch in s.chars() {
                                match ch {
                                    '&' => out.push_str("&amp;"),
                                    '<' => out.push_str("&lt;"),
                                    '>' => out.push_str("&gt;"),
                                    '"' => out.push_str("&quot;"),
                                    '\'' => out.push_str("&#39;"),
                                    _ => out.push(ch),
                                }
                            }
                            self.stack.push(Value::Str(out));
                        },
                        11 => { // GET$()
                            if argc != 0 { return Err(BasilError("GET$ expects 0 arguments".into())); }
                            self.ensure_get_params();
                            let vals = self.get_params_cache.clone().unwrap_or_default();
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        12 => { // POST$()
                            if argc != 0 { return Err(BasilError("POST$ expects 0 arguments".into())); }
                            self.ensure_post_params();
                            let vals = self.post_params_cache.clone().unwrap_or_default();
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        13 => { // REQUEST$()
                            if argc != 0 { return Err(BasilError("REQUEST$ expects 0 arguments".into())); }
                            self.ensure_get_params();
                            self.ensure_post_params();
                            let mut vals = self.get_params_cache.clone().unwrap_or_default();
                            if let Some(mut p) = self.post_params_cache.clone() { vals.append(&mut p); }
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        14 => { // UCASE$(s)
                            if argc != 1 { return Err(BasilError("UCASE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("UCASE$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.to_uppercase()));
                        }
                        15 => { // LCASE$(s)
                            if argc != 1 { return Err(BasilError("LCASE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("LCASE$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.to_lowercase()));
                        }
                        16 => { // TRIM$(s)
                            if argc != 1 { return Err(BasilError("TRIM$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("TRIM$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.trim().to_string()));
                        }
                        17 => { // CHR$(n)
                            if argc != 1 { return Err(BasilError("CHR$ expects 1 argument".into())); }
                            let n = match &args[0] {
                                Value::Int(i) => *i,
                                Value::Num(f) => f.trunc() as i64,
                                _ => return Err(BasilError("CHR$ arg must be numeric".into())),
                            };
                            let out = if n < 0 || n > 0x10FFFF { String::new() } else { std::char::from_u32(n as u32).map(|c| c.to_string()).unwrap_or_default() };
                            self.stack.push(Value::Str(out));
                        }
                        18 => { // ASC%(s)
                            if argc != 1 { return Err(BasilError("ASC% expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s, _ => return Err(BasilError("ASC% arg must be string".into())) };
                            let code: i64 = s.chars().next().map(|c| c as u32 as i64).unwrap_or(0);
                            self.stack.push(Value::Int(code));
                        }
                        19 => { // INPUTC$()
                            if argc != 0 { return Err(BasilError("INPUTC$ expects 0 arguments".into())); }
                            enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                            let s = loop {
                                match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                    Event::Key(KeyEvent { code, .. }) => {
                                        let out = match code {
                                            KeyCode::Char(c) if c.is_ascii() => c.to_string(),
                                            _ => String::new(),
                                        };
                                        break out;
                                    }
                                    _ => { /* ignore other events */ }
                                }
                            };
                            let _ = disable_raw_mode();
                            self.stack.push(Value::Str(s));
                        }
                        _ => return Err(BasilError(format!("unknown builtin id {}", bid))),
                    }
                }

                Op::Halt => {
                    if self.frames.len() == 1 { break; }
                    else { return Err(BasilError("HALT inside function".into())); }
                }
               // other => { return Err(BasilError(format!("unhandled opcode {:?}", other))); }
            }
        }
        Ok(())
    }

    // --- helpers ---

    fn read_op(&mut self) -> Result<Op> {
        let f = self.cur();
        let byte = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))?;
        f.ip += 1;
        let op = match byte {
            1=>Op::ConstU8, 2=>Op::LoadGlobal, 3=>Op::StoreGlobal,
            11=>Op::LoadLocal, 12=>Op::StoreLocal,
            20=>Op::Add, 21=>Op::Sub, 22=>Op::Mul, 23=>Op::Div, 24=>Op::Neg,
            30=>Op::Eq, 31=>Op::Ne, 32=>Op::Lt, 33=>Op::Le, 34=>Op::Gt, 35=>Op::Ge,
            40=>Op::Jump, 41=>Op::JumpIfFalse, 42=>Op::JumpBack,
            50=>Op::Call, 51=>Op::Ret,
            60=>Op::Print, 61=>Op::Pop, 62=>Op::ToInt, 63=>Op::Builtin,
            70=>Op::ArrMake, 71=>Op::ArrGet, 72=>Op::ArrSet,
            80=>Op::NewObj, 81=>Op::GetProp, 82=>Op::SetProp, 83=>Op::CallMethod, 84=>Op::DescribeObj,
            90=>Op::EnumNew, 91=>Op::EnumMoveNext, 92=>Op::EnumCurrent, 93=>Op::EnumDispose,
            255=>Op::Halt,
            _ => return Err(BasilError(format!("bad opcode {}", byte))),
        };
        Ok(op)
    }
    fn read_u8(&mut self) -> Result<u8> {
        let f = self.cur();
        let b = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))?;
        f.ip += 1; Ok(b)
    }
    fn read_u16(&mut self) -> Result<u16> {
        let f = self.cur();
        let lo = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))? as u16;
        let hi = *f.chunk.code.get(f.ip+1).ok_or_else(|| BasilError("ip out of range".into()))? as u16;
        f.ip += 2; Ok(lo | (hi<<8))
    }
    fn pop(&mut self) -> Result<Value> { self.stack.pop().ok_or_else(|| BasilError("stack underflow".into())) }

    fn as_num(&self, v: Value) -> Result<f64> {
        match v {
            Value::Num(n) => Ok(n),
            Value::Int(i) => Ok(i as f64),
            Value::Bool(b) => Ok(if b { 1.0 } else { 0.0 }),
            _ => Err(BasilError("expected number".into())),
        }
    }
    fn bin_num<F: Fn(f64,f64)->f64>(&mut self, f: F) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        let b = self.as_num(b)?; let a = self.as_num(a)?;
        self.stack.push(Value::Num(f(a,b))); Ok(())
    }
    fn bin_cmp<F: Fn(&Value,&Value)->bool>(&mut self, f: F) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        self.stack.push(Value::Bool(f(&a,&b))); Ok(())
    }
    fn bin_num_cmp<F: Fn(f64,f64)->bool>(&mut self, f: F) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        let b = self.as_num(b)?; let a = self.as_num(a)?;
        self.stack.push(Value::Bool(f(a,b))); Ok(())
    }

    fn type_of(&self, v: &Value) -> String {
        match v {
            Value::Null => "NULL".to_string(),
            Value::Bool(_) => "BOOL".to_string(),
            Value::Num(_) => "FLOAT".to_string(),
            Value::Int(_) => "INTEGER".to_string(),
            Value::Str(_) => "STRING".to_string(),
            Value::Func(_) => "FUNCTION".to_string(),
            Value::Array(arr_rc) => {
                let arr = arr_rc.as_ref();
                let base = match &arr.elem {
                    ElemType::Num => "FLOAT".to_string(),
                    ElemType::Int => "INTEGER".to_string(),
                    ElemType::Str => "STRING".to_string(),
                    ElemType::Obj(Some(tn)) => tn.clone(),
                    ElemType::Obj(None) => "OBJECT".to_string(),
                };
                format!("{}[]", base)
            }
            Value::Object(rc) => rc.borrow().type_name().to_string(),
        }
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Num(n) => *n != 0.0,
        Value::Int(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Func(_) => true,
        Value::Array(_) => true,
        Value::Object(_) => true,
    }
}
