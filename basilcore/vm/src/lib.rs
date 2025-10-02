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

                Op::Add => self.bin_num(|a,b| a+b)?,
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
            60=>Op::Print, 61=>Op::Pop, 62=>Op::ToInt,
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
