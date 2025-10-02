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

use basil_common::{Result, BasilError};
use basil_bytecode::{Program as BCProgram, Chunk, Value, Op};

struct Frame {
    chunk: Rc<Chunk>,
    ip: usize,
    base: usize,
}

pub struct VM {
    frames: Vec<Frame>,
    stack: Vec<Value>,
    globals: Vec<Value>,
}

impl VM {
    pub fn new(p: BCProgram) -> Self {
        let globals = vec![Value::Null; p.globals.len()];
        let top_chunk = Rc::new(p.chunk);
        let frame = Frame { chunk: top_chunk, ip: 0, base: 0 };
        Self { frames: vec![frame], stack: Vec::new(), globals }
    }

    fn cur(&mut self) -> &mut Frame { self.frames.last_mut().expect("no frame") }

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

                Op::Print => { let v = self.pop()?; println!("{}", v); }
                Op::Pop   => { let _ = self.pop()?; }
                Op::ToInt => {
                    let v = self.pop()?;
                    match v {
                        Value::Int(i) => self.stack.push(Value::Int(i)),
                        Value::Num(n) => self.stack.push(Value::Int(n.trunc() as i64)),
                        _ => return Err(BasilError("ToInt expects a numeric value".into())),
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
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Num(n) => *n != 0.0,
        Value::Int(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Func(_) => true,
    }
}
