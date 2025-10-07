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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use basil_common::{Result, BasilError};
use basil_ast::{Program, Stmt, Expr, BinOp};
use basil_bytecode::{Chunk, Program as BCProgram, Value, Op, Function};

pub fn compile(ast: &Program) -> Result<BCProgram> {
    let mut c = C::new();
    // Pre-scan to collect all function names so calls can be resolved before definitions
    for s in ast {
        if let Stmt::Func { name, .. } = s {
            c.fn_names.insert(name.to_ascii_uppercase());
        }
    }
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
    fn_names: HashSet<String>,
    loop_stack: Vec<LoopCtx>,
}

impl C {
    fn new() -> Self {
        Self { chunk: Chunk::default(), globals: Vec::new(), gmap: HashMap::new(), fn_names: HashSet::new(), loop_stack: Vec::new() }
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
                // remember function name for call vs array indexing disambiguation
                self.fn_names.insert(name.to_ascii_uppercase());
                let f = self.compile_function(name.clone(), params.clone(), body);
                self.chunk.push_op(Op::Const);
                let idx = self.chunk.add_const(f);
                self.chunk.push_u16(idx);

                let g = self.gslot(name);
                self.chunk.push_op(Op::StoreGlobal);
                self.chunk.push_u8(g);
            }

            // Top-level LET/PRINT/EXPR: move chunk out to avoid &mut self + &mut self.chunk alias.
            Stmt::Let { name, indices, init } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                match indices {
                    None => {
                        self.emit_expr_in(&mut chunk, init, None)?;
                        if name.ends_with('%') { chunk.push_op(Op::ToInt); }
                        let g = self.gslot(name);
                        chunk.push_op(Op::StoreGlobal);
                        chunk.push_u8(g);
                    }
                    Some(idxs) => {
                        // array element assignment
                        let g = self.gslot(name);
                        chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                        for ix in idxs { self.emit_expr_in(&mut chunk, ix, None)?; }
                        self.emit_expr_in(&mut chunk, init, None)?;
                        chunk.push_op(Op::ArrSet); chunk.push_u8(idxs.len() as u8);
                    }
                }
                self.chunk = chunk;
            }
            Stmt::Dim { name, dims } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                for d in dims { self.emit_expr_in(&mut chunk, d, None)?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                let et = if name.ends_with('%') { 1u8 } else if name.ends_with('$') { 2u8 } else { 0u8 };
                chunk.push_u8(et);
                // primitive arrays: emit placeholder type-name const index as u16 (0xFFFF)
                chunk.push_u16(65535u16);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.chunk = chunk;
            }
            Stmt::DimObjectArray { name, dims, type_name } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                for d in dims { self.emit_expr_in(&mut chunk, d, None)?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                chunk.push_u8(3u8); // object array
                let tci = if let Some(tn) = type_name { chunk.add_const(Value::Str(tn.clone())) } else { 65535u16 };
                chunk.push_u16(tci);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.chunk = chunk;
            }
            Stmt::Print { expr } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, expr, None)?;
                chunk.push_op(Op::Print);
                self.chunk = chunk;
            }
            Stmt::Describe { target } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, target, None)?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
                self.chunk = chunk;
            }
            // Unstructured flow placeholders (no-op for now)
            Stmt::Label(_name) => { /* no-op at toplevel */ }
            Stmt::Goto(_name) => { /* GOTO unsupported at MVP; ignoring to proceed */ }
            Stmt::Gosub(_name) => { /* GOSUB unsupported at MVP; ignoring to proceed */ }
            Stmt::DimObject { name, type_name, args } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // push args
                for a in args { self.emit_expr_in(&mut chunk, a, None)?; }
                // get type name constant index
                let tci = chunk.add_const(Value::Str(type_name.clone()));
                // emit NEW_OBJ (note: VM expects type const index operand then argc)
                chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.chunk = chunk;
            }
            Stmt::SetProp { target, prop, value } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, target, None)?;
                self.emit_expr_in(&mut chunk, value, None)?;
                // property name const index
                let pci = chunk.add_const(Value::Str(prop.clone()));
                chunk.push_op(Op::SetProp); chunk.push_u16(pci);
                self.chunk = chunk;
            }
            Stmt::ExprStmt(e) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, e, None)?;
                chunk.push_op(Op::Pop);
                self.chunk = chunk;
            }
            Stmt::Line(line) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                chunk.push_op(Op::SetLine);
                chunk.push_u16((*line as u16).min(u16::MAX));
                self.chunk = chunk;
            }

            // Not needed for `fib`, but harmless if someone writes a block at top level.
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_toplevel(s2)?; }
            }

            // Ignore RETURN at toplevel (harmless), but support IF blocks.
            Stmt::Return(_) => {}
            Stmt::If { cond, then_branch, else_branch } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_if_tl_into(&mut chunk, cond, then_branch, else_branch)?;
                self.chunk = chunk;
            }

            // WHILE at toplevel
            Stmt::While { cond, body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                let test_here = chunk.here();
                self.emit_expr_in(&mut chunk, cond, None)?;
                chunk.push_op(Op::JumpIfFalse);
                let j_exit = chunk.emit_u16_placeholder();
                // push loop ctx
                self.loop_stack.push(LoopCtx { test_here, break_sites: Vec::new() });
                // body
                self.emit_stmt_tl_in_chunk(&mut chunk, body)?;
                // back to test
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                // exit label
                let exit_here = chunk.here();
                let off_exit = (exit_here - (j_exit + 2)) as u16; chunk.patch_u16_at(j_exit, off_exit);
                // patch BREAKs
                let ctx = self.loop_stack.pop().unwrap();
                for site in ctx.break_sites { let off = (exit_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                self.chunk = chunk;
            }

            Stmt::Break => { return Err(BasilError("BREAK used outside of loop".into())); }
            Stmt::Continue => { return Err(BasilError("CONTINUE used outside of loop".into())); }

            // FOR EACH at toplevel
            Stmt::ForEach { var, enumerable, body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // Evaluate enumerable and create enumerator
                self.emit_expr_in(&mut chunk, enumerable, None)?;
                chunk.push_op(Op::EnumNew);
                // test label
                let test_here = chunk.here();
                chunk.push_op(Op::EnumMoveNext);
                chunk.push_op(Op::JumpIfFalse);
                let j_end = chunk.emit_u16_placeholder();
                // current element -> assign to loop var
                chunk.push_op(Op::EnumCurrent);
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
                let g = self.gslot(var);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                // body
                self.emit_stmt_tl_in_chunk(&mut chunk, body)?;
                // jump back to test
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                // end label
                let end_here = chunk.here();
                let off_end = (end_here - (j_end + 2)) as u16; chunk.patch_u16_at(j_end, off_end);
                // dispose enumerator (pops handle)
                chunk.push_op(Op::EnumDispose);
                self.chunk = chunk;
            }

            // FOR at toplevel
            Stmt::For { var, start, end, step, body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // init: var = start
                self.emit_expr_in(&mut chunk, start, None)?;
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
                let g = self.gslot(var);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);

                // loop start label
                let loop_start = chunk.here();

                // step >= 0 ?
                match step {
                    Some(e) => { self.emit_expr_in(&mut chunk, e, None)?; }
                    None => {
                        let idx = chunk.add_const(Value::Num(1.0));
                        chunk.push_op(Op::Const); chunk.push_u16(idx);
                    }
                }
                let idx0 = chunk.add_const(Value::Num(0.0));
                chunk.push_op(Op::Const); chunk.push_u16(idx0);
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
                        chunk.push_op(Op::Const); chunk.push_u16(idx1);
                    }
                }
                chunk.push_op(Op::Add);
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
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
        fchunk.push_op(Op::Const);
        let cid = fchunk.add_const(Value::Null);
        fchunk.push_u16(cid);
        fchunk.push_op(Op::Ret);

        Value::Func(Rc::new(Function {
            arity: params.len() as u8,
            name: Some(name),
            chunk: Rc::new(fchunk),
        }))
    }

    fn emit_stmt_func(&mut self, chunk: &mut Chunk, s: &Stmt, env: &mut LocalEnv) -> Result<()> {
        match s {
            Stmt::Let { name, indices, init } => {
                match indices {
                    None => {
                        self.emit_expr_in(chunk, init, Some(env))?;
                        if name.ends_with('%') { chunk.push_op(Op::ToInt); }
                        let slot = env.bind_next_if_absent(name.clone());
                        chunk.push_op(Op::StoreLocal);
                        chunk.push_u8(slot);
                    }
                    Some(idxs) => {
                        // array element assignment: load array ref (local or global), push indices, value, ArrSet
                        if let Some(slot) = env.lookup(name) {
                            chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                        } else {
                            let g = self.gslot(name);
                            chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                        }
                        for ix in idxs { self.emit_expr_in(chunk, ix, Some(env))?; }
                        self.emit_expr_in(chunk, init, Some(env))?;
                        chunk.push_op(Op::ArrSet); chunk.push_u8(idxs.len() as u8);
                    }
                }
            }
            Stmt::Dim { name, dims } => {
                for d in dims { self.emit_expr_in(chunk, d, Some(env))?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                let et = if name.ends_with('%') { 1u8 } else if name.ends_with('$') { 2u8 } else { 0u8 };
                chunk.push_u8(et);
                // primitive arrays: emit placeholder type-name const index as u16 (0xFFFF)
                chunk.push_u16(65535u16);
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::DimObjectArray { name, dims, type_name } => {
                for d in dims { self.emit_expr_in(chunk, d, Some(env))?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                chunk.push_u8(3u8);
                let tci: u16 = if let Some(tn) = type_name { chunk.add_const(Value::Str(tn.clone())) } else { 65535u16 };
                chunk.push_u16(tci);
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::Print { expr } => {
                self.emit_expr_in(chunk, expr, Some(env))?;
                chunk.push_op(Op::Print);
            }
            Stmt::Describe { target } => {
                self.emit_expr_in(chunk, target, Some(env))?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
            }
            // Unstructured flow placeholders (no-op inside function for now)
            Stmt::Label(_name) => { /* no-op */ }
            Stmt::Goto(_name) => { /* ignore */ }
            Stmt::Gosub(_name) => { /* ignore */ }
            Stmt::DimObject { name, type_name, args } => {
                for a in args { self.emit_expr_in(chunk, a, Some(env))?; }
                let tci = chunk.add_const(Value::Str(type_name.clone()));
                chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::SetProp { target, prop, value } => {
                self.emit_expr_in(chunk, target, Some(env))?;
                self.emit_expr_in(chunk, value, Some(env))?;
                let pci = chunk.add_const(Value::Str(prop.clone()));
                chunk.push_op(Op::SetProp); chunk.push_u16(pci);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr_in(chunk, e, Some(env))?;
                chunk.push_op(Op::Pop);
            }
            Stmt::Line(line) => {
                chunk.push_op(Op::SetLine);
                chunk.push_u16((*line as u16).min(u16::MAX));
            }
            Stmt::Return(eopt) => {
                if let Some(e) = eopt {
                    self.emit_expr_in(chunk, e, Some(env))?;
                } else {
                    chunk.push_op(Op::Const);
                    let cid = chunk.add_const(Value::Null);
                    chunk.push_u16(cid);
                }
                chunk.push_op(Op::Ret);
            }
            Stmt::If { cond, then_branch, else_branch } => {
                self.emit_if_func(chunk, cond, then_branch, else_branch, env)?;
            }
            // WHILE in function
            Stmt::While { cond, body } => {
                let test_here = chunk.here();
                self.emit_expr_in(chunk, cond, Some(env))?;
                chunk.push_op(Op::JumpIfFalse);
                let j_exit = chunk.emit_u16_placeholder();
                self.loop_stack.push(LoopCtx { test_here, break_sites: Vec::new() });
                self.emit_stmt_func(chunk, body, env)?;
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                let exit_here = chunk.here();
                let off_exit = (exit_here - (j_exit + 2)) as u16; chunk.patch_u16_at(j_exit, off_exit);
                let ctx = self.loop_stack.pop().unwrap();
                for site in ctx.break_sites { let off = (exit_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
            }
            Stmt::Break => {
                if self.loop_stack.is_empty() { return Err(BasilError("BREAK used outside of loop".into())); }
                chunk.push_op(Op::Jump);
                let site = chunk.emit_u16_placeholder();
                if let Some(ctx) = self.loop_stack.last_mut() { ctx.break_sites.push(site); }
            }
            Stmt::Continue => {
                if self.loop_stack.is_empty() { return Err(BasilError("CONTINUE used outside of loop".into())); }
                let test_here = self.loop_stack.last().unwrap().test_here;
                chunk.push_op(Op::JumpBack);
                let jb = chunk.emit_u16_placeholder();
                let off = (jb + 2 - test_here) as u16; chunk.patch_u16_at(jb, off);
            }
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_func(chunk, s2, env)?; }
            }
            Stmt::Func { .. } => { /* no nested funcs in MVP */ }
            Stmt::ForEach { var, enumerable, body } => {
                // Evaluate enumerable and create enumerator
                self.emit_expr_in(chunk, enumerable, Some(env))?;
                chunk.push_op(Op::EnumNew);
                // test
                let test_here = chunk.here();
                chunk.push_op(Op::EnumMoveNext);
                chunk.push_op(Op::JumpIfFalse);
                let j_end = chunk.emit_u16_placeholder();
                // current -> assign to loop var (local if exists else global)
                chunk.push_op(Op::EnumCurrent);
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
                if let Some(slot) = env.lookup(var) {
                    chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                } else {
                    let g = self.gslot(var);
                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                }
                // body
                self.emit_stmt_func(chunk, body, env)?;
                // back to test
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                // end
                let end_here = chunk.here();
                let off_end = (end_here - (j_end + 2)) as u16; chunk.patch_u16_at(j_end, off_end);
                chunk.push_op(Op::EnumDispose);
            }
            Stmt::For { var, start, end, step, body } => {
                // init var
                self.emit_expr_in(chunk, start, Some(env))?;
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
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
                        chunk.push_op(Op::Const); chunk.push_u16(idx);
                    }
                }
                let idx0 = chunk.add_const(Value::Num(0.0));
                chunk.push_op(Op::Const); chunk.push_u16(idx0);
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
                        chunk.push_op(Op::Const); chunk.push_u16(idx1);
                    }
                }
                chunk.push_op(Op::Add);
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
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
                chunk.push_op(Op::Const); chunk.push_u16(idx);
            }
            Expr::Str(s) => {
                let idx = chunk.add_const(Value::Str(s.clone()));
                chunk.push_op(Op::Const); chunk.push_u16(idx);
            }
            Expr::Bool(b) => {
                let idx = chunk.add_const(Value::Bool(*b));
                chunk.push_op(Op::Const); chunk.push_u16(idx);
            }
            Expr::Var(name) => {
                // Minimal constants for object features
                let uname = name.to_ascii_uppercase();
                if uname == "PRO" {
                    let ci = chunk.add_const(Value::Int(1));
                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                    return Ok(());
                } else if uname == "NOT_PRO" {
                    let ci = chunk.add_const(Value::Int(0));
                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                    return Ok(());
                }
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
            Expr::UnaryNot(inner) => {
                // NOT with truthiness
                self.emit_expr_in(chunk, inner, env)?;
                chunk.push_op(Op::JumpIfFalse);
                let jf = chunk.emit_u16_placeholder();
                // truthy path: push false
                let cf = chunk.add_const(Value::Bool(false));
                chunk.push_op(Op::Const); chunk.push_u16(cf);
                chunk.push_op(Op::Jump);
                let jend = chunk.emit_u16_placeholder();
                // falsey path label
                let after_jf = chunk.here();
                let off_jf = (after_jf - (jf + 2)) as u16; chunk.patch_u16_at(jf, off_jf);
                let ct = chunk.add_const(Value::Bool(true));
                chunk.push_op(Op::Const); chunk.push_u16(ct);
                // end label
                let end_here = chunk.here();
                let off_end = (end_here - (jend + 2)) as u16; chunk.patch_u16_at(jend, off_end);
            }
            Expr::Binary { op, lhs, rhs } => {
                match op {
                    BinOp::And => {
                        // Short-circuit AND producing Bool
                        self.emit_expr_in(chunk, lhs, env)?;
                        chunk.push_op(Op::JumpIfFalse);
                        let jf_lhs = chunk.emit_u16_placeholder();
                        self.emit_expr_in(chunk, rhs, env)?;
                        chunk.push_op(Op::JumpIfFalse);
                        let jf_rhs = chunk.emit_u16_placeholder();
                        // both truthy
                        let ct = chunk.add_const(Value::Bool(true));
                        chunk.push_op(Op::Const); chunk.push_u16(ct);
                        chunk.push_op(Op::Jump);
                        let jend = chunk.emit_u16_placeholder();
                        // false label
                        let l_false = chunk.here();
                        let off_lhs = (l_false - (jf_lhs + 2)) as u16; chunk.patch_u16_at(jf_lhs, off_lhs);
                        let off_rhs = (l_false - (jf_rhs + 2)) as u16; chunk.patch_u16_at(jf_rhs, off_rhs);
                        let cf = chunk.add_const(Value::Bool(false));
                        chunk.push_op(Op::Const); chunk.push_u16(cf);
                        // end label
                        let l_end = chunk.here();
                        let off_end = (l_end - (jend + 2)) as u16; chunk.patch_u16_at(jend, off_end);
                    }
                    BinOp::Or => {
                        // Short-circuit OR producing Bool
                        self.emit_expr_in(chunk, lhs, env)?;
                        chunk.push_op(Op::JumpIfFalse);
                        let j_eval_rhs = chunk.emit_u16_placeholder();
                        // lhs truthy => true
                        let ct = chunk.add_const(Value::Bool(true));
                        chunk.push_op(Op::Const); chunk.push_u16(ct);
                        chunk.push_op(Op::Jump);
                        let jend = chunk.emit_u16_placeholder();
                        // evaluate rhs label
                        let l_rhs = chunk.here();
                        let off_rhs = (l_rhs - (j_eval_rhs + 2)) as u16; chunk.patch_u16_at(j_eval_rhs, off_rhs);
                        self.emit_expr_in(chunk, rhs, env)?;
                        chunk.push_op(Op::JumpIfFalse);
                        let jf_false = chunk.emit_u16_placeholder();
                        // rhs truthy => true
                        let ct2 = chunk.add_const(Value::Bool(true));
                        chunk.push_op(Op::Const); chunk.push_u16(ct2);
                        chunk.push_op(Op::Jump);
                        let jend2 = chunk.emit_u16_placeholder();
                        // false label
                        let l_false = chunk.here();
                        let off_false = (l_false - (jf_false + 2)) as u16; chunk.patch_u16_at(jf_false, off_false);
                        let cf = chunk.add_const(Value::Bool(false));
                        chunk.push_op(Op::Const); chunk.push_u16(cf);
                        // end label
                        let l_end = chunk.here();
                        let off_end1 = (l_end - (jend + 2)) as u16; chunk.patch_u16_at(jend, off_end1);
                        let off_end2 = (l_end - (jend2 + 2)) as u16; chunk.patch_u16_at(jend2, off_end2);
                    }
                    _ => {
                        self.emit_expr_in(chunk, lhs, env)?;
                        self.emit_expr_in(chunk, rhs, env)?;
                        chunk.push_op(match op {
                            BinOp::Add => Op::Add, BinOp::Sub => Op::Sub, BinOp::Mul => Op::Mul, BinOp::Div => Op::Div,
                            BinOp::Eq  => Op::Eq,  BinOp::Ne  => Op::Ne,
                            BinOp::Lt  => Op::Lt,  BinOp::Le  => Op::Le, BinOp::Gt => Op::Gt, BinOp::Ge => Op::Ge,
                            BinOp::And | BinOp::Or => unreachable!(),
                        });
                    }
                }
            }
            Expr::Call { callee, args } => {
                // Detect DESCRIBE$(obj) pseudo-builtin
                if let Expr::Var(name) = &**callee {
                    let uname = name.to_ascii_uppercase();
                    if uname == "DESCRIBE$" {
                        if args.len() != 1 { /* fall through to regular call for error later */ } else {
                            for a in args { self.emit_expr_in(chunk, a, env)?; }
                            chunk.push_op(Op::DescribeObj);
                            return Ok(());
                        }
                    }
                    // Detect other built-in string functions by name and emit a Builtin opcode instead of a call
                    let bid = match &*uname {
                        "LEN" => Some(1u8),
                        "MID$" => Some(2u8),
                        "LEFT$" => Some(3u8),
                        "RIGHT$" => Some(4u8),
                        "INSTR" => Some(5u8),
                        "INPUT$" => Some(6u8),
                        "INPUT" => Some(6u8), // alias for convenience
                        "INKEY$" => Some(7u8),
                        "INKEY%" => Some(8u8),
                        "TYPE$" => Some(9u8),
                        "HTML$" => Some(10u8),
                        "HTML" => Some(10u8),
                        "GET$" => Some(11u8),
                        "POST$" => Some(12u8),
                        "REQUEST$" => Some(13u8),
                        "UCASE$" => Some(14u8),
                        "LCASE$" => Some(15u8),
                        "TRIM$"  => Some(16u8),
                        "CHR$"   => Some(17u8),
                        "ASC%"   => Some(18u8),
                        "INPUTC$"=> Some(19u8),
                        _ => None,
                    };
                    if let Some(id) = bid {
                        for a in args { self.emit_expr_in(chunk, a, env)?; }
                        chunk.push_op(Op::Builtin); chunk.push_u8(id); chunk.push_u8(args.len() as u8);
                        return Ok(());
                    }
                    // If not builtin, treat as array access when not a known function
                    if args.len() >= 1 && args.len() <= 4 {
                        let is_func = self.fn_names.contains(&uname);
                        // prefer local var if present
                        if let Some(env) = env {
                            if env.lookup(name).is_some() && !is_func {
                                let slot = env.lookup(name).unwrap();
                                chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                                for a in args { self.emit_expr_in(chunk, a, Some(env))?; }
                                chunk.push_op(Op::ArrGet); chunk.push_u8(args.len() as u8);
                                return Ok(());
                            }
                        }
                        if !is_func {
                            let g = self.gslot(name);
                            chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                            for a in args { self.emit_expr_in(chunk, a, env)?; }
                            chunk.push_op(Op::ArrGet); chunk.push_u8(args.len() as u8);
                            return Ok(());
                        }
                    }
                }
                // Regular call
                self.emit_expr_in(chunk, callee, env)?;
                for a in args { self.emit_expr_in(chunk, a, env)?; }
                chunk.push_op(Op::Call); chunk.push_u8(args.len() as u8);
            }
            Expr::MemberGet { target, name } => {
                self.emit_expr_in(chunk, target, env)?;
                let ci = chunk.add_const(Value::Str(name.clone()));
                chunk.push_op(Op::GetProp); chunk.push_u16(ci);
            }
            Expr::MemberCall { target, method, args } => {
                self.emit_expr_in(chunk, target, env)?;
                for a in args { self.emit_expr_in(chunk, a, env)?; }
                let ci = chunk.add_const(Value::Str(method.clone()));
                chunk.push_op(Op::CallMethod); chunk.push_u16(ci); chunk.push_u8(args.len() as u8);
            }
            Expr::NewObject { type_name, args } => {
                for a in args { self.emit_expr_in(chunk, a, env)?; }
                let tci = chunk.add_const(Value::Str(type_name.clone()));
                chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
            }
            Expr::NewClass { filename } => {
                // Evaluate filename and instantiate class at runtime
                self.emit_expr_in(chunk, filename, env)?;
                chunk.push_op(Op::NewClass);
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
            Stmt::Let { name, indices, init } => {
                match indices {
                    None => {
                        self.emit_expr_in(chunk, init, None)?;
                        let g = self.gslot(name);
                        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                    }
                    Some(idxs) => {
                        let g = self.gslot(name);
                        chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                        for ix in idxs { self.emit_expr_in(chunk, ix, None)?; }
                        self.emit_expr_in(chunk, init, None)?;
                        chunk.push_op(Op::ArrSet); chunk.push_u8(idxs.len() as u8);
                    }
                }
            }
            Stmt::Dim { name, dims } => {
                for d in dims { self.emit_expr_in(chunk, d, None)?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                let et = if name.ends_with('%') { 1u8 } else if name.ends_with('$') { 2u8 } else { 0u8 };
                chunk.push_u8(et);
                // primitive arrays: emit placeholder type-name const index as u16 (0xFFFF)
                chunk.push_u16(65535u16);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::DimObjectArray { name, dims, type_name } => {
                for d in dims { self.emit_expr_in(chunk, d, None)?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                chunk.push_u8(3u8);
                let tci: u16 = if let Some(tn) = type_name { chunk.add_const(Value::Str(tn.clone())) } else { 65535u16 };
                chunk.push_u16(tci);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::Print { expr } => {
                self.emit_expr_in(chunk, expr, None)?;
                chunk.push_op(Op::Print);
            }
            Stmt::Describe { target } => {
                self.emit_expr_in(chunk, target, None)?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
            }
            // Unstructured flow placeholders (no-op inside toplevel chunk)
            Stmt::Label(_name) => { /* no-op */ }
            Stmt::Goto(_name) => { /* ignore */ }
            Stmt::Gosub(_name) => { /* ignore */ }
            Stmt::DimObject { name, type_name, args } => {
                for a in args { self.emit_expr_in(chunk, a, None)?; }
                let tci = chunk.add_const(Value::Str(type_name.clone()));
                chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::SetProp { target, prop, value } => {
                self.emit_expr_in(chunk, target, None)?;
                self.emit_expr_in(chunk, value, None)?;
                let pci = chunk.add_const(Value::Str(prop.clone()));
                chunk.push_op(Op::SetProp); chunk.push_u16(pci);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr_in(chunk, e, None)?;
                chunk.push_op(Op::Pop);
            }
            Stmt::Line(line) => {
                chunk.push_op(Op::SetLine);
                chunk.push_u16((*line as u16).min(u16::MAX));
            }
            Stmt::Return(_) => { /* ignore at top level inside FOR body */ }
            Stmt::If { cond, then_branch, else_branch } => {
                self.emit_if_tl_into(chunk, cond, then_branch, else_branch)?;
            }
            // WHILE inside toplevel chunk
            Stmt::While { cond, body } => {
                let test_here = chunk.here();
                self.emit_expr_in(chunk, cond, None)?;
                chunk.push_op(Op::JumpIfFalse);
                let j_exit = chunk.emit_u16_placeholder();
                self.loop_stack.push(LoopCtx { test_here, break_sites: Vec::new() });
                self.emit_stmt_tl_in_chunk(chunk, body)?;
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                let exit_here = chunk.here();
                let off_exit = (exit_here - (j_exit + 2)) as u16; chunk.patch_u16_at(j_exit, off_exit);
                let ctx = self.loop_stack.pop().unwrap();
                for site in ctx.break_sites { let off = (exit_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
            }
            Stmt::Break => {
                if self.loop_stack.is_empty() { return Err(BasilError("BREAK used outside of loop".into())); }
                chunk.push_op(Op::Jump);
                let site = chunk.emit_u16_placeholder();
                if let Some(ctx) = self.loop_stack.last_mut() { ctx.break_sites.push(site); }
            }
            Stmt::Continue => {
                if self.loop_stack.is_empty() { return Err(BasilError("CONTINUE used outside of loop".into())); }
                let test_here = self.loop_stack.last().unwrap().test_here;
                chunk.push_op(Op::JumpBack);
                let jb = chunk.emit_u16_placeholder();
                let off = (jb + 2 - test_here) as u16; chunk.patch_u16_at(jb, off);
            }
            Stmt::Block(stmts) => {
                for s2 in stmts { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
            }
            Stmt::Func { name, params, body } => {
                let f = self.compile_function(name.clone(), params.clone(), body);
                chunk.push_op(Op::Const);
                let idx = chunk.add_const(f);
                chunk.push_u16(idx);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::ForEach { var, enumerable, body } => {
                // Evaluate enumerable and create enumerator
                self.emit_expr_in(chunk, enumerable, None)?;
                chunk.push_op(Op::EnumNew);
                let test_here = chunk.here();
                chunk.push_op(Op::EnumMoveNext);
                chunk.push_op(Op::JumpIfFalse);
                let j_end = chunk.emit_u16_placeholder();
                chunk.push_op(Op::EnumCurrent);
                if var.ends_with('%') { chunk.push_op(Op::ToInt); }
                let g = self.gslot(var);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.emit_stmt_tl_in_chunk(chunk, body)?;
                chunk.push_op(Op::JumpBack);
                let j_back = chunk.emit_u16_placeholder();
                let off_back = (j_back + 2 - test_here) as u16; chunk.patch_u16_at(j_back, off_back);
                let end_here = chunk.here();
                let off_end = (end_here - (j_end + 2)) as u16; chunk.patch_u16_at(j_end, off_end);
                chunk.push_op(Op::EnumDispose);
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
            None => { let idx = chunk.add_const(Value::Num(1.0)); chunk.push_op(Op::Const); chunk.push_u16(idx); }
        }
        let idx0 = chunk.add_const(Value::Num(0.0));
        chunk.push_op(Op::Const); chunk.push_u16(idx0);
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
        match step { Some(e) => { self.emit_expr_in(chunk, e, None)?; }, None => { let idx1 = chunk.add_const(Value::Num(1.0)); chunk.push_op(Op::Const); chunk.push_u16(idx1); } }
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


// loop context for BREAK/CONTINUE within WHILE loops
struct LoopCtx { test_here: usize, break_sites: Vec<usize> }
