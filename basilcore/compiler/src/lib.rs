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

//! AST → bytecode compiler with functions, calls, returns, if/blocks
use std::collections::HashMap;
use std::rc::Rc;

use basil_common::Result;
use basil_ast::{Program, Stmt, Expr, BinOp};
use basil_bytecode::{Chunk, Program as BCProgram, Value, Op, Function};

pub fn compile(ast: &Program) -> Result<BCProgram> {
    let mut c = C::new();
    for s in ast {
        c.emit_stmt_toplevel(s)?;
    }
    c.chunk.push_op(Op::Halt);
    Ok(BCProgram { chunk: c.chunk, globals: c.globals })
}

struct C {
    chunk: Chunk,
    globals: Vec<String>,
    gmap: HashMap<String, u8>,
}

impl C {
    fn new() -> Self {
        Self { chunk: Chunk::default(), globals: Vec::new(), gmap: HashMap::new() }
    }

    fn gslot(&mut self, name: &str) -> u8 {
        if let Some(&i) = self.gmap.get(name) { return i; }
        let i = self.globals.len() as u8;
        self.globals.push(name.to_string());
        self.gmap.insert(name.to_string(), i);
        i
    }

    fn emit_stmt_toplevel(&mut self, s: &Stmt) -> Result<()> {
        match s {
            // Compile function to a Function value and store into a global.
            Stmt::Func { name, params, body } => {
                let f = self.compile_function(name.clone(), params.clone(), body);
                self.chunk.push_op(Op::ConstU8);
                let idx = self.chunk.add_const(f);
                self.chunk.push_u8(idx);

                let g = self.gslot(name);
                self.chunk.push_op(Op::StoreGlobal);
                self.chunk.push_u8(g);
            }

            // Top-level LET/PRINT/EXPR: move chunk out to avoid &mut self + &mut self.chunk alias.
            Stmt::Let { name, init } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, init, None)?;
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal);
                chunk.push_u8(g);
                self.chunk = chunk;
            }
            Stmt::Print { expr } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, expr, None)?;
                chunk.push_op(Op::Print);
                self.chunk = chunk;
            }
            Stmt::ExprStmt(e) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, e, None)?;
                chunk.push_op(Op::Pop);
                self.chunk = chunk;
            }

            // Not needed for `fib`, but harmless if someone writes a block at top level.
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_toplevel(s2)?; }
            }

            // Ignore RETURN/IF at toplevel for MVP (could be supported later).
            Stmt::Return(_) | Stmt::If { .. } => {}
        }
        Ok(())
    }

    fn compile_function(&mut self, name: String, params: Vec<String>, body: &Vec<Stmt>) -> Value {
        let mut fchunk = Chunk::default();
        let mut env = LocalEnv::new();

        // params occupy local slots 0..arity-1
        for (i, p) in params.iter().enumerate() {
            env.bind(p.to_string(), i as u8);
        }

        // body
        for s in body {
            self.emit_stmt_func(&mut fchunk, s, &mut env).unwrap();
        }

        // implicit return null
        fchunk.push_op(Op::ConstU8);
        let cid = fchunk.add_const(Value::Null);
        fchunk.push_u8(cid);
        fchunk.push_op(Op::Ret);

        Value::Func(Rc::new(Function {
            arity: params.len() as u8,
            name: Some(name),
            chunk: Rc::new(fchunk),
        }))
    }

    fn emit_stmt_func(&mut self, chunk: &mut Chunk, s: &Stmt, env: &mut LocalEnv) -> Result<()> {
        match s {
            Stmt::Let { name, init } => {
                self.emit_expr_in(chunk, init, Some(env))?;
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal);
                chunk.push_u8(slot);
            }
            Stmt::Print { expr } => {
                self.emit_expr_in(chunk, expr, Some(env))?;
                chunk.push_op(Op::Print);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr_in(chunk, e, Some(env))?;
                chunk.push_op(Op::Pop);
            }
            Stmt::Return(eopt) => {
                if let Some(e) = eopt {
                    self.emit_expr_in(chunk, e, Some(env))?;
                } else {
                    chunk.push_op(Op::ConstU8);
                    let cid = chunk.add_const(Value::Null);
                    chunk.push_u8(cid);
                }
                chunk.push_op(Op::Ret);
            }
            Stmt::If { cond, then_branch, else_branch } => {
                self.emit_if_func(chunk, cond, then_branch, else_branch, env)?;
            }
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_func(chunk, s2, env)?; }
            }
            Stmt::Func { .. } => { /* no nested funcs in MVP */ }
        }
        Ok(())
    }

    fn emit_if_func(
        &mut self,
        chunk: &mut Chunk,
        cond: &Expr,
        then_s: &Stmt,
        else_s: &Option<Box<Stmt>>,
        env: &mut LocalEnv,
    ) -> Result<()> {
        self.emit_expr_in(chunk, cond, Some(env))?;
        chunk.push_op(Op::JumpIfFalse);
        let jf = chunk.emit_u16_placeholder();

        self.emit_stmt_func(chunk, then_s, env)?;
        chunk.push_op(Op::Jump);
        let je = chunk.emit_u16_placeholder();

        let after_then = chunk.here();
        let off_then = (after_then - (jf + 2)) as u16;
        chunk.patch_u16_at(jf, off_then);

        if let Some(e) = else_s {
            self.emit_stmt_func(chunk, e, env)?;
        }

        let after_else = chunk.here();
        let off_else = (after_else - (je + 2)) as u16;
        chunk.patch_u16_at(je, off_else);

        Ok(())
    }

    fn emit_expr_in(&mut self, chunk: &mut Chunk, e: &Expr, env: Option<&LocalEnv>) -> Result<()> {
        match e {
            Expr::Number(n) => {
                let idx = chunk.add_const(Value::Num(*n));
                chunk.push_op(Op::ConstU8); chunk.push_u8(idx);
            }
            Expr::Str(s) => {
                let idx = chunk.add_const(Value::Str(s.clone()));
                chunk.push_op(Op::ConstU8); chunk.push_u8(idx);
            }
            Expr::Var(name) => {
                if let Some(env) = env {
                    if let Some(slot) = env.lookup(name) {
                        chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                        return Ok(());
                    }
                }
                let g = self.gslot(name);
                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
            }
            Expr::UnaryNeg(inner) => { self.emit_expr_in(chunk, inner, env)?; chunk.push_op(Op::Neg); }
            Expr::Binary { op, lhs, rhs } => {
                self.emit_expr_in(chunk, lhs, env)?;
                self.emit_expr_in(chunk, rhs, env)?;
                chunk.push_op(match op {
                    BinOp::Add => Op::Add, BinOp::Sub => Op::Sub, BinOp::Mul => Op::Mul, BinOp::Div => Op::Div,
                    BinOp::Eq  => Op::Eq,  BinOp::Ne  => Op::Ne,
                    BinOp::Lt  => Op::Lt,  BinOp::Le  => Op::Le, BinOp::Gt => Op::Gt, BinOp::Ge => Op::Ge,
                });
            }
            Expr::Call { callee, args } => {
                self.emit_expr_in(chunk, callee, env)?;
                for a in args { self.emit_expr_in(chunk, a, env)?; }
                chunk.push_op(Op::Call); chunk.push_u8(args.len() as u8);
            }
        }
        Ok(())
    }
}

// ---- locals env ----
#[derive(Clone)]
struct LocalEnv {
    map: HashMap<String, u8>,
    next: u8,
}
impl LocalEnv {
    fn new() -> Self { Self { map: HashMap::new(), next: 0 } }
    fn bind(&mut self, name: String, slot: u8) { self.map.insert(name, slot); self.next = self.next.max(slot + 1); }
    fn bind_next_if_absent(&mut self, name: String) -> u8 {
        if let Some(&i) = self.map.get(&name) { return i; }
        let i = self.next; self.map.insert(name, i); self.next += 1; i
    }
    fn lookup(&self, name: &str) -> Option<u8> { self.map.get(name).copied() }
}
