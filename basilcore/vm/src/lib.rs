//! Minimal stack VM for Basil v0
use basil_common::{Result, BasilError};
use basil_bytecode::{Program as BCProgram, Chunk, Value, Op};

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    globals: Vec<Value>,
}

impl VM {
    pub fn new(p: BCProgram) -> Self {
        let globals = vec![Value::Null; p.globals.len()];
        Self { chunk: p.chunk, ip: 0, stack: Vec::new(), globals }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let op = self.read_op()?;
            match op {
                Op::ConstU8 => {
                    let i = self.read_u8()? as usize;
                    let v = self.chunk.consts[i].clone();
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
                Op::Add => self.bin_num(|a, b| a + b)?,
                Op::Sub => self.bin_num(|a, b| a - b)?,
                Op::Mul => self.bin_num(|a, b| a * b)?,
                Op::Div => self.bin_num(|a, b| a / b)?,
                Op::Neg => {
                    let a = {
                        let val = self.pop()?;
                        self.as_num(val)?
                    };
                    self.stack.push(Value::Num(-a));
                }
                Op::Print => {
                    let v = self.pop()?;
                    println!("{}", v);
                }
                Op::Pop => {
                    let _ = self.pop()?;
                }
                Op::Halt => break,
            }
        }
        Ok(())
    }

    fn read_op(&mut self) -> Result<Op> {
        let byte = *self
            .chunk
            .code
            .get(self.ip)
            .ok_or_else(|| BasilError("ip out of range".into()))?;
        self.ip += 1;
        let op = match byte {
            1 => Op::ConstU8,
            2 => Op::LoadGlobal,
            3 => Op::StoreGlobal,
            4 => Op::Add,
            5 => Op::Sub,
            6 => Op::Mul,
            7 => Op::Div,
            8 => Op::Neg,
            9 => Op::Print,
            10 => Op::Pop,
            255 => Op::Halt,
            _ => return Err(BasilError(format!("bad opcode {}", byte))),
        };
        Ok(op)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let b = *self
            .chunk
            .code
            .get(self.ip)
            .ok_or_else(|| BasilError("ip out of range".into()))?;
        self.ip += 1;
        Ok(b)
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| BasilError("stack underflow".into()))
    }

    fn as_num(&self, v: Value) -> Result<f64> {
        if let Value::Num(n) = v {
            Ok(n)
        } else {
            Err(BasilError("expected number".into()))
        }
    }

    fn bin_num<F: Fn(f64, f64) -> f64>(&mut self, f: F) -> Result<()> {
        let b = {
            let val = self.pop()?;
            self.as_num(val)?
        };
        let a = {
            let val = self.pop()?;
            self.as_num(val)?
        };
        self.stack.push(Value::Num(f(a, b)));
        Ok(())
    }
}
