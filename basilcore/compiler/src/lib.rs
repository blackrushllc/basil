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

            // FOR at toplevel
            Stmt::For { var, start, end, step, body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // init: var = start
                self.emit_expr_in(&mut chunk, start, None)?;
                let g = self.gslot(var);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);

                // loop start label
                let loop_start = chunk.here();

                // step >= 0 ?
                match step {
                    Some(e) => { self.emit_expr_in(&mut chunk, e, None)?; }
                    None => {
                        let idx = chunk.add_const(Value::Num(1.0));
                        chunk.push_op(Op::ConstU8); chunk.push_u8(idx);
                    }
                }
                let idx0 = chunk.add_const(Value::Num(0.0));
                chunk.push_op(Op::ConstU8); chunk.push_u8(idx0);
                chunk.push_op(Op::Ge);
                chunk.push_op(Op::JumpIfFalse);
                let j_to_neg = chunk.emit_u16_placeholder();

                // positive step compare: var <= end
                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                self.emit_expr_in(&mut chunk, end, None)?;
                chunk.push_op(Op::Le);
                chunk.push_op(Op::JumpIfFalse);
                let j_exit1 = chunk.emit_u16_placeholder();
                chunk.push_op(Op::Jump);
                let j_after_pos = chunk.emit_u16_placeholder();

                // negative step path label
                let after_pos = chunk.here();
                let off_to_neg = (after_pos - (j_to_neg + 2)) as u16;
                chunk.patch_u16_at(j_to_neg, off_to_neg);

                // negative step compare: var >= end
                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                self.emit_expr_in(&mut chunk, end, None)?;
                chunk.push_op(Op::Ge);
                chunk.push_op(Op::JumpIfFalse);
                let j_exit2 = chunk.emit_u16_placeholder();

                // after compare join
                let after_cmp = chunk.here();
                let off_after_pos = (after_cmp - (j_after_pos + 2)) as u16;
                chunk.patch_u16_at(j_after_pos, off_after_pos);

                // body
                self.emit_stmt_tl_in_chunk(&mut chunk, body)?;

                // increment: var = var + step
                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                match step {
                    Some(e) => { self.emit_expr_in(&mut chunk, e, None)?; }
                    None => {
                        let idx1 = chunk.add_const(Value::Num(1.0));
                        chunk.push_op(Op::ConstU8); chunk.push_u8(idx1);
                    }
                }
                chunk.push_op(Op::Add);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);

                // jump back (use JumpBack with u16 distance backwards)
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - loop_start) as u16; // ip after reading u16 minus loop_start
                chunk.patch_u16_at(j_back, off_back);

                // exit label patches
                let exit_here = chunk.here();
                let off_exit1 = (exit_here - (j_exit1 + 2)) as u16; chunk.patch_u16_at(j_exit1, off_exit1);
                let off_exit2 = (exit_here - (j_exit2 + 2)) as u16; chunk.patch_u16_at(j_exit2, off_exit2);

                self.chunk = chunk;
            }
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
            Stmt::For { var, start, end, step, body } => {
                // init var
                self.emit_expr_in(chunk, start, Some(env))?;
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                }

                // loop start
                let loop_start = chunk.here();

                // step >= 0 ?
                match step {
                    Some(e) => { self.emit_expr_in(chunk, e, Some(env))?; }
                    None => {
                        let idx = chunk.add_const(Value::Num(1.0));
                        chunk.push_op(Op::ConstU8); chunk.push_u8(idx);
                    }
                }
                let idx0 = chunk.add_const(Value::Num(0.0));
                chunk.push_op(Op::ConstU8); chunk.push_u8(idx0);
                chunk.push_op(Op::Ge);
                chunk.push_op(Op::JumpIfFalse);
                let j_to_neg = chunk.emit_u16_placeholder();

                // positive compare: var <= end
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                }
                self.emit_expr_in(chunk, end, Some(env))?;
                chunk.push_op(Op::Le);
                chunk.push_op(Op::JumpIfFalse);
                let j_exit1 = chunk.emit_u16_placeholder();
                chunk.push_op(Op::Jump);
                let j_after_pos = chunk.emit_u16_placeholder();

                // negative path label
                let after_pos = chunk.here();
                let off_to_neg = (after_pos - (j_to_neg + 2)) as u16;
                chunk.patch_u16_at(j_to_neg, off_to_neg);

                // negative compare: var >= end
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                }
                self.emit_expr_in(chunk, end, Some(env))?;
                chunk.push_op(Op::Ge);
                chunk.push_op(Op::JumpIfFalse);
                let j_exit2 = chunk.emit_u16_placeholder();

                // after compare join
                let after_cmp = chunk.here();
                let off_after_pos = (after_cmp - (j_after_pos + 2)) as u16;
                chunk.patch_u16_at(j_after_pos, off_after_pos);

                // body
                self.emit_stmt_func(chunk, body, env)?;

                // increment: var = var + step
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                }
                match step {
                    Some(e) => { self.emit_expr_in(chunk, e, Some(env))?; }
                    None => {
                        let idx1 = chunk.add_const(Value::Num(1.0));
                        chunk.push_op(Op::ConstU8); chunk.push_u8(idx1);
                    }
                }
                chunk.push_op(Op::Add);
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                }

                // jump back
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - loop_start) as u16;
                chunk.patch_u16_at(j_back, off_back);

                // exit label patch
                let exit_here = chunk.here();
                let off_exit1 = (exit_here - (j_exit1 + 2)) as u16; chunk.patch_u16_at(j_exit1, off_exit1);
                let off_exit2 = (exit_here - (j_exit2 + 2)) as u16; chunk.patch_u16_at(j_exit2, off_exit2);
            }
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


impl C {
    fn emit_if_tl_into(&mut self, chunk: &mut Chunk, cond: &Expr, then_s: &Stmt, else_s: &Option<Box<Stmt>>) -> Result<()> {
        self.emit_expr_in(chunk, cond, None)?;
        chunk.push_op(Op::JumpIfFalse);
        let jf = chunk.emit_u16_placeholder();

        self.emit_stmt_tl_in_chunk(chunk, then_s)?;
        chunk.push_op(Op::Jump);
        let je = chunk.emit_u16_placeholder();

        let after_then = chunk.here();
        let off_then = (after_then - (jf + 2)) as u16;
        chunk.patch_u16_at(jf, off_then);

        if let Some(e) = else_s {
            self.emit_stmt_tl_in_chunk(chunk, e)?;
        }

        let after_else = chunk.here();
        let off_else = (after_else - (je + 2)) as u16;
        chunk.patch_u16_at(je, off_else);

        Ok(())
    }

    fn emit_stmt_tl_in_chunk(&mut self, chunk: &mut Chunk, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let { name, init } => {
                self.emit_expr_in(chunk, init, None)?;
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::Print { expr } => {
                self.emit_expr_in(chunk, expr, None)?;
                chunk.push_op(Op::Print);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr_in(chunk, e, None)?;
                chunk.push_op(Op::Pop);
            }
            Stmt::Return(_) => { /* ignore at top level inside FOR body */ }
            Stmt::If { cond, then_branch, else_branch } => {
                self.emit_if_tl_into(chunk, cond, then_branch, else_branch)?;
            }
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
            }
            Stmt::Func { name, params, body } => {
                let f = self.compile_function(name.clone(), params.clone(), body);
                chunk.push_op(Op::ConstU8);
                let idx = chunk.add_const(f);
                chunk.push_u8(idx);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::For { var, start, end, step, body } => {
                self.emit_for_toplevel_into(chunk, var, start, end, step, body)?;
            }
        }
        Ok(())
    }

    fn emit_for_toplevel_into(&mut self, chunk: &mut Chunk, var: &String, start: &Expr, end: &Expr, step: &Option<Expr>, body: &Stmt) -> Result<()> {
        // init
        self.emit_expr_in(chunk, start, None)?;
        let g = self.gslot(var);
        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);

        // loop start
        let loop_start = chunk.here();

        // step >= 0 ?
        match step {
            Some(e) => { self.emit_expr_in(chunk, e, None)?; }
            None => { let idx = chunk.add_const(Value::Num(1.0)); chunk.push_op(Op::ConstU8); chunk.push_u8(idx); }
        }
        let idx0 = chunk.add_const(Value::Num(0.0));
        chunk.push_op(Op::ConstU8); chunk.push_u8(idx0);
        chunk.push_op(Op::Ge);
        chunk.push_op(Op::JumpIfFalse);
        let j_to_neg = chunk.emit_u16_placeholder();

        // positive compare var <= end
        chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
        self.emit_expr_in(chunk, end, None)?;
        chunk.push_op(Op::Le);
        chunk.push_op(Op::JumpIfFalse);
        let j_exit1 = chunk.emit_u16_placeholder();
        chunk.push_op(Op::Jump);
        let j_after_pos = chunk.emit_u16_placeholder();

        // negative label
        let after_pos = chunk.here();
        let off_to_neg = (after_pos - (j_to_neg + 2)) as u16; chunk.patch_u16_at(j_to_neg, off_to_neg);

        // negative compare var >= end
        chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
        self.emit_expr_in(chunk, end, None)?;
        chunk.push_op(Op::Ge);
        chunk.push_op(Op::JumpIfFalse);
        let j_exit2 = chunk.emit_u16_placeholder();

        // after cmp join
        let after_cmp = chunk.here();
        let off_after_pos = (after_cmp - (j_after_pos + 2)) as u16; chunk.patch_u16_at(j_after_pos, off_after_pos);

        // body
        self.emit_stmt_tl_in_chunk(chunk, body)?;

        // increment
        chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
        match step { Some(e) => { self.emit_expr_in(chunk, e, None)?; }, None => { let idx1 = chunk.add_const(Value::Num(1.0)); chunk.push_op(Op::ConstU8); chunk.push_u8(idx1); } }
        chunk.push_op(Op::Add);
        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);

        // back jump
        chunk.push_op(Op::JumpBack);
        let j_back = chunk.emit_u16_placeholder();
        let off_back = (j_back + 2 - loop_start) as u16; chunk.patch_u16_at(j_back, off_back);

        // exit label
        let exit_here = chunk.here();
        let off_exit1 = (exit_here - (j_exit1 + 2)) as u16; chunk.patch_u16_at(j_exit1, off_exit1);
        let off_exit2 = (exit_here - (j_exit2 + 2)) as u16; chunk.patch_u16_at(j_exit2, off_exit2);

        Ok(())
    }
}
