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

pub mod service;

pub fn compile(ast: &Program) -> Result<BCProgram> {
    let mut c = C::new();
    // Pre-scan to collect all routine names (FUNC/SUB) with arity and kind so calls can be resolved before definitions
    for s in ast {
        if let Stmt::Func { kind, name, params, .. } = s {
            let uname = name.to_ascii_uppercase();
            c.fn_names.insert(uname.clone());
            c.routines.insert(uname, RoutineInfo { arity: params.len(), is_sub: matches!(kind, basil_ast::FuncKind::Sub) });
        }
    }
    for s in ast {
        c.emit_stmt_toplevel(s)?;
    }
    // Resolve top-level GOTO fixups now that all labels are known
    for (op_pos, u16_pos, label) in std::mem::take(&mut c.tl_goto_fixups) {
        if let Some(&target) = c.tl_labels.get(&label) {
            // Decide direction and patch
            if target >= u16_pos + 2 {
                // forward jump
                let off = (target - (u16_pos + 2)) as u16;
                c.chunk.patch_u16_at(u16_pos, off);
            } else {
                // backward jump → flip opcode to JumpBack and patch distance backwards
                c.chunk.code[op_pos] = Op::JumpBack as u8;
                let off = ((u16_pos + 2) - target) as u16;
                c.chunk.patch_u16_at(u16_pos, off);
            }
        } else {
            return Err(BasilError(format!("Undefined label: {}", label)));
        }
    }
    // Resolve top-level GOSUB fixups
    for (op_pos, u16_pos, label) in std::mem::take(&mut c.tl_gosub_fixups) {
        if let Some(&target) = c.tl_labels.get(&label) {
            if target >= u16_pos + 2 {
                let off = (target - (u16_pos + 2)) as u16;
                c.chunk.patch_u16_at(u16_pos, off);
            } else {
                c.chunk.code[op_pos] = Op::GosubBack as u8;
                let off = ((u16_pos + 2) - target) as u16;
                c.chunk.patch_u16_at(u16_pos, off);
            }
        } else {
            return Err(BasilError(format!("Undefined label: {}", label)));
        }
    }
    c.chunk.push_op(Op::Halt);
    Ok(BCProgram { chunk: c.chunk, globals: c.globals })
}

struct RoutineInfo { arity: usize, is_sub: bool }

fn expr_contains_sub_call(routines: &HashMap<String, RoutineInfo>, e: &Expr) -> bool {
    match e {
        Expr::Call { callee, args } => {
            if let Expr::Var(name) = &**callee {
                let uname = name.to_ascii_uppercase();
                if let Some(info) = routines.get(&uname) {
                    if info.is_sub { return true; }
                }
            }
            for a in args {
                if expr_contains_sub_call(routines, a) { return true; }
            }
            false
        }
        Expr::UnaryNeg(e1) | Expr::UnaryNot(e1) => expr_contains_sub_call(routines, e1),
        Expr::Binary { lhs, rhs, .. } => expr_contains_sub_call(routines, lhs) || expr_contains_sub_call(routines, rhs),
        Expr::MemberGet { target, .. } => expr_contains_sub_call(routines, target),
        Expr::MemberCall { target, args, .. } => {
            if expr_contains_sub_call(routines, target) { return true; }
            for a in args { if expr_contains_sub_call(routines, a) { return true; } }
            false
        }
        Expr::NewObject { args, .. } => args.iter().any(|a| expr_contains_sub_call(routines, a)),
        Expr::NewClass { filename } => expr_contains_sub_call(routines, filename),
        _ => false,
    }
}

struct C {
    chunk: Chunk,
    globals: Vec<String>,
    gmap: HashMap<String, u8>,
    fn_names: HashSet<String>,
    routines: HashMap<String, RoutineInfo>,
    loop_stack: Vec<LoopCtx>,
    // Label/GOTO support (top-level)
    tl_labels: HashMap<String, usize>,
    tl_goto_fixups: Vec<(usize, usize, String)>, // (op_pos, u16_pos, label)
    tl_gosub_fixups: Vec<(usize, usize, String)>,
    // Label/GOTO support (current function)
    fn_labels: HashMap<String, usize>,
    fn_goto_fixups: Vec<(usize, usize, String)>,
    fn_gosub_fixups: Vec<(usize, usize, String)>,
}

impl C {
    fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            globals: Vec::new(),
            gmap: HashMap::new(),
            fn_names: HashSet::new(),
            routines: HashMap::new(),
            loop_stack: Vec::new(),
            tl_labels: HashMap::new(),
            tl_goto_fixups: Vec::new(),
            tl_gosub_fixups: Vec::new(),
            fn_labels: HashMap::new(),
            fn_goto_fixups: Vec::new(),
            fn_gosub_fixups: Vec::new(),
        }
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
            Stmt::Func { name, params, body, .. } => {
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
                        // array element assignment or whole-array assignment if idxs is empty: name$() = expr
                        if idxs.is_empty() {
                            // Evaluate RHS and convert from StrArray2D to real 2D string array via builtin 138
                            self.emit_expr_in(&mut chunk, init, None)?;
                            chunk.push_op(Op::Builtin); chunk.push_u8(138u8); chunk.push_u8(1u8);
                            let g = self.gslot(name);
                            chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                        } else {
                            let g = self.gslot(name);
                            chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                            for ix in idxs { self.emit_expr_in(&mut chunk, ix, None)?; }
                            self.emit_expr_in(&mut chunk, init, None)?;
                            chunk.push_op(Op::ArrSet); chunk.push_u8(idxs.len() as u8);
                        }
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
            // SETENV/EXPORTENV
            Stmt::SetEnv { name, value, export } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // push name, value, export flag
                let nci = chunk.add_const(Value::Str(name.clone()));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                self.emit_expr_in(&mut chunk, value, None)?;
                let eci = chunk.add_const(Value::Bool(*export));
                chunk.push_op(Op::Const); chunk.push_u16(eci);
                // call builtin 59 with 3 args
                chunk.push_op(Op::Builtin); chunk.push_u8(59u8); chunk.push_u8(3u8);
                // discard result
                chunk.push_op(Op::Pop);
                self.chunk = chunk;
            }
            // SHELL
            Stmt::Shell { cmd } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, cmd, None)?;
                chunk.push_op(Op::Builtin); chunk.push_u8(60u8); chunk.push_u8(1u8);
                // discard exit code (statement form)
                chunk.push_op(Op::Pop);
                self.chunk = chunk;
            }
            // EXIT [code]
            Stmt::Exit(code_opt) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                if let Some(e) = code_opt { self.emit_expr_in(&mut chunk, e, None)?; }
                else { let ci = chunk.add_const(Value::Int(0)); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                chunk.push_op(Op::Builtin); chunk.push_u8(61u8); chunk.push_u8(1u8);
                self.chunk = chunk;
            }
            // Unstructured flow: LABEL/GOTO at top level
            Stmt::Label(name) => {
                let pos = self.chunk.here();
                if self.tl_labels.insert(name.clone(), pos).is_some() {
                    return Err(BasilError(format!("Duplicate label: {}", name)));
                }
            }
            Stmt::Goto(name) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                chunk.push_op(Op::Jump);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.tl_goto_fixups.push((op_pos, u16_pos, name.clone()));
                self.chunk = chunk;
            }
            Stmt::Gosub(name) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                chunk.push_op(Op::Gosub);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.tl_gosub_fixups.push((op_pos, u16_pos, name.clone()));
                self.chunk = chunk;
            }
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
                // Special-case: direct SUB call as a statement: NAME(args...);
                if let Expr::Call { callee, args } = e {
                    if let Expr::Var(name) = &**callee {
                        let uname = name.to_ascii_uppercase();
                        if let Some(info) = self.routines.get(&uname) {
                            if info.is_sub {
                                // Arity check
                                if info.arity != args.len() {
                                    return Err(BasilError(format!("procedure '{}' expects {} arguments but {} given", name, info.arity, args.len())));
                                }
                                // Ensure no nested SUB calls inside arguments
                                for a in args {
                                    if expr_contains_sub_call(&self.routines, a) {
                                        return Err(BasilError("SUB call has no value; cannot be used inside arguments".into()));
                                    }
                                }
                                // Emit callee and arguments and call
                                let g = self.gslot(name);
                                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                                for a in args { self.emit_expr_in(&mut chunk, a, None)?; }
                                chunk.push_op(Op::Call); chunk.push_u8(args.len() as u8);
                                // discard result (SUB has no value)
                                chunk.push_op(Op::Pop);
                                self.chunk = chunk;
                                return Ok(());
                            }
                        }
                    }
                }
                // Generic expression statement (includes FUNC calls): validate no SUB in value context
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

            // Ignore function RETURN at toplevel (harmless)
            Stmt::Return(_) => {}
            // RETURN from GOSUB
            Stmt::ReturnFromGosub(lbl_opt) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                match lbl_opt {
                    None => { chunk.push_op(Op::GosubRet); }
                    Some(label) => {
                        chunk.push_op(Op::GosubPop);
                        chunk.push_op(Op::Jump);
                        let op_pos = chunk.here() - 1;
                        let u16_pos = chunk.emit_u16_placeholder();
                        self.tl_goto_fixups.push((op_pos, u16_pos, label.clone()));
                    }
                }
                self.chunk = chunk;
            }
            Stmt::If { cond, then_branch, else_branch } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_if_tl_into(&mut chunk, cond, then_branch, else_branch)?;
                self.chunk = chunk;
            }
            Stmt::SelectCase { selector, arms, else_body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_select_case_tl_into(&mut chunk, selector, arms, else_body)?;
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

        // reset function-scope labels/fixups
        self.fn_labels.clear();
        self.fn_goto_fixups.clear();

        // body
        for s in body {
            self.emit_stmt_func(&mut fchunk, s, &mut env).unwrap();
        }

        // resolve function-level GOTOs now that all labels are known
        for (op_pos, u16_pos, label) in std::mem::take(&mut self.fn_goto_fixups) {
            if let Some(&target) = self.fn_labels.get(&label) {
                if target >= u16_pos + 2 {
                    let off = (target - (u16_pos + 2)) as u16;
                    fchunk.patch_u16_at(u16_pos, off);
                } else {
                    fchunk.code[op_pos] = Op::JumpBack as u8;
                    let off = ((u16_pos + 2) - target) as u16;
                    fchunk.patch_u16_at(u16_pos, off);
                }
            } else {
                panic!("Undefined label in function {}: {}", name, label);
            }
        }

        // resolve function-level GOSUBs now that all labels are known
        for (op_pos, u16_pos, label) in std::mem::take(&mut self.fn_gosub_fixups) {
            if let Some(&target) = self.fn_labels.get(&label) {
                if target >= u16_pos + 2 {
                    let off = (target - (u16_pos + 2)) as u16;
                    fchunk.patch_u16_at(u16_pos, off);
                } else {
                    fchunk.code[op_pos] = Op::GosubBack as u8;
                    let off = ((u16_pos + 2) - target) as u16;
                    fchunk.patch_u16_at(u16_pos, off);
                }
            } else {
                panic!("Undefined label in function {}: {}", name, label);
            }
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
                        // If a local with this name already exists, store into it.
                        if let Some(slot) = env.lookup(name) {
                            chunk.push_op(Op::StoreLocal);
                            chunk.push_u8(slot);
                        } else if self.gmap.contains_key(name) && !self.routines.contains_key(&name.to_ascii_uppercase()) {
                            // Otherwise, if a global of this name exists (e.g., class field), assign to the global.
                            let g = self.gslot(name);
                            chunk.push_op(Op::StoreGlobal);
                            chunk.push_u8(g);
                        } else {
                            // Fallback: create/bind a new local.
                            let slot = env.bind_next_if_absent(name.clone());
                            chunk.push_op(Op::StoreLocal);
                            chunk.push_u8(slot);
                        }
                    }
                    Some(idxs) => {
                        if idxs.is_empty() {
                            // Whole-array assignment: name$() = expr
                            self.emit_expr_in(chunk, init, Some(env))?;
                            chunk.push_op(Op::Builtin); chunk.push_u8(138u8); chunk.push_u8(1u8);
                            if let Some(slot) = env.lookup(name) {
                                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                            } else if self.gmap.contains_key(name) && !self.routines.contains_key(&name.to_ascii_uppercase()) {
                                let g = self.gslot(name);
                                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                            } else {
                                let slot = env.bind_next_if_absent(name.clone());
                                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                            }
                        } else {
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
            // SETENV/EXPORTENV inside function
            Stmt::SetEnv { name, value, export } => {
                // push name, value, export flag
                let nci = chunk.add_const(Value::Str(name.clone()));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                self.emit_expr_in(chunk, value, Some(env))?;
                let eci = chunk.add_const(Value::Bool(*export));
                chunk.push_op(Op::Const); chunk.push_u16(eci);
                chunk.push_op(Op::Builtin); chunk.push_u8(59u8); chunk.push_u8(3u8);
                chunk.push_op(Op::Pop);
            }
            // SHELL inside function
            Stmt::Shell { cmd } => {
                self.emit_expr_in(chunk, cmd, Some(env))?;
                chunk.push_op(Op::Builtin); chunk.push_u8(60u8); chunk.push_u8(1u8);
                chunk.push_op(Op::Pop);
            }
            // EXIT inside function
            Stmt::Exit(code_opt) => {
                if let Some(e) = code_opt { self.emit_expr_in(chunk, e, Some(env))?; }
                else { let ci = chunk.add_const(Value::Int(0)); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                chunk.push_op(Op::Builtin); chunk.push_u8(61u8); chunk.push_u8(1u8);
            }
            // Unstructured flow inside function: support LABEL/GOTO
            Stmt::Label(name) => {
                let pos = chunk.here();
                if self.fn_labels.insert(name.clone(), pos).is_some() {
                    return Err(BasilError(format!("Duplicate label: {}", name)));
                }
            }
            Stmt::Goto(name) => {
                chunk.push_op(Op::Jump);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.fn_goto_fixups.push((op_pos, u16_pos, name.clone()));
            }
            Stmt::Gosub(name) => {
                chunk.push_op(Op::Gosub);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.fn_gosub_fixups.push((op_pos, u16_pos, name.clone()));
            }
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
                // Special-case: direct SUB call as a statement
                if let Expr::Call { callee, args } = e {
                    if let Expr::Var(name) = &**callee {
                        let uname = name.to_ascii_uppercase();
                        if let Some(info) = self.routines.get(&uname) {
                            if info.is_sub {
                                if info.arity != args.len() {
                                    return Err(BasilError(format!("procedure '{}' expects {} arguments but {} given", name, info.arity, args.len())));
                                }
                                for a in args {
                                    if expr_contains_sub_call(&self.routines, a) {
                                        return Err(BasilError("SUB call has no value; cannot be used inside arguments".into()));
                                    }
                                }
                                // Emit callee and args
                                let g = self.gslot(name);
                                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                                for a in args { self.emit_expr_in(chunk, a, Some(env))?; }
                                chunk.push_op(Op::Call); chunk.push_u8(args.len() as u8);
                                chunk.push_op(Op::Pop);
                                return Ok(());
                            }
                        }
                    }
                }
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
            Stmt::ReturnFromGosub(lbl_opt) => {
                match lbl_opt {
                    None => { chunk.push_op(Op::GosubRet); }
                    Some(label) => {
                        chunk.push_op(Op::GosubPop);
                        chunk.push_op(Op::Jump);
                        let op_pos = chunk.here() - 1;
                        let u16_pos = chunk.emit_u16_placeholder();
                        self.fn_goto_fixups.push((op_pos, u16_pos, label.clone()));
                    }
                }
            }
            Stmt::If { cond, then_branch, else_branch } => {
                self.emit_if_func(chunk, cond, then_branch, else_branch, env)?;
            }
            Stmt::SelectCase { selector, arms, else_body } => {
                self.emit_select_case_func(chunk, selector, arms, else_body, env)?;
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
                // Save enumerator handle in a temp local so the loop body can freely use the stack
                let tmp_name = format!("$__enumH%{}", env.next);
                let tmp_slot = env.bind_next_if_absent(tmp_name);
                chunk.push_op(Op::StoreLocal); chunk.push_u8(tmp_slot);
                // test
                let test_here = chunk.here();
                chunk.push_op(Op::LoadLocal); chunk.push_u8(tmp_slot);
                chunk.push_op(Op::EnumMoveNext);
                chunk.push_op(Op::JumpIfFalse);
                let j_end = chunk.emit_u16_placeholder();
                // current -> assign to loop var (local if exists else global)
                chunk.push_op(Op::LoadLocal); chunk.push_u8(tmp_slot);
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
                // dispose enumerator
                chunk.push_op(Op::LoadLocal); chunk.push_u8(tmp_slot);
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
            // Forbid SUB calls in value contexts (allowed only as direct statements)
            if expr_contains_sub_call(&self.routines, e) {
                return Err(BasilError("SUB call has no value; cannot be used in an expression. Call it as a statement: NAME(...);".into()));
            }
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
                            BinOp::Add => Op::Add, BinOp::Sub => Op::Sub, BinOp::Mul => Op::Mul, BinOp::Div => Op::Div, BinOp::Mod => Op::Mod,
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
                        "ESCAPE$" => Some(20u8),
                        "UNESCAPE$" => Some(21u8),
                        "URLENCODE$" => Some(22u8),
                        "URLDECODE$" => Some(23u8),
                        "STRING$" => Some(26u8),
                        "SLEEP" => Some(24u8),
                        "FOPEN" => Some(40u8),
                        "FCLOSE" => Some(41u8),
                        "FFLUSH" => Some(42u8),
                        "FEOF" => Some(43u8),
                        "FTELL&" => Some(44u8),
                        "FSEEK" => Some(45u8),
                        "FREAD$" => Some(46u8),
                        "FREADLINE$" => Some(47u8),
                        "FWRITE" => Some(48u8),
                        "FWRITELN" => Some(49u8),
                        "READFILE$" => Some(50u8),
                        "WRITEFILE" => Some(51u8),
                        "APPENDFILE" => Some(52u8),
                        "COPY" => Some(53u8),
                        "MOVE" => Some(54u8),
                        "RENAME" => Some(55u8),
                        "DELETE" => Some(56u8),
                        "DIR$" => Some(57u8),
                        "ENV$" => Some(58u8),
                        #[cfg(feature = "obj-base64")] "BASE64_ENCODE$" => Some(90u8),
                        #[cfg(feature = "obj-base64")] "BASE64_DECODE$" => Some(91u8),
                        #[cfg(feature = "obj-zip")] "ZIP_EXTRACT_ALL" => Some(120u8),
                        #[cfg(feature = "obj-zip")] "ZIP_COMPRESS_FILE" => Some(121u8),
                        #[cfg(feature = "obj-zip")] "ZIP_COMPRESS_DIR" => Some(122u8),
                        #[cfg(feature = "obj-zip")] "ZIP_LIST$" => Some(123u8),
                        #[cfg(feature = "obj-curl")] "HTTP_GET$" => Some(124u8),
                        #[cfg(feature = "obj-curl")] "HTTP_POST$" => Some(125u8),
                        #[cfg(feature = "obj-json")] "JSON_PARSE$" => Some(126u8),
                        #[cfg(feature = "obj-json")] "JSON_STRINGIFY$" => Some(127u8),
                        #[cfg(feature = "obj-csv")] "CSV_PARSE$" => Some(128u8),
                        #[cfg(feature = "obj-csv")] "CSV_WRITE$" => Some(129u8),
                        #[cfg(feature = "obj-sqlite")] "SQLITE_OPEN%" => Some(130u8),
                        #[cfg(feature = "obj-sqlite")] "SQLITE_CLOSE" => Some(131u8),
                        #[cfg(feature = "obj-sqlite")] "SQLITE_EXEC%" => Some(132u8),
                        #[cfg(feature = "obj-sqlite")] "SQLITE_QUERY2D$" => Some(133u8),
                        #[cfg(feature = "obj-sqlite")] "SQLITE_LAST_INSERT_ID%" => Some(134u8),
                        // --- Terminal builtins ---
                        #[cfg(feature = "obj-term")] "CLS" => Some(230u8),
                        #[cfg(feature = "obj-term")] "CLEAR" => Some(230u8),
                        #[cfg(feature = "obj-term")] "HOME" => Some(230u8),
                        #[cfg(feature = "obj-term")] "LOCATE" => Some(231u8),
                        #[cfg(feature = "obj-term")] "COLOR" => Some(232u8),
                        #[cfg(feature = "obj-term")] "COLOR_RESET" => Some(233u8),
                        #[cfg(feature = "obj-term")] "ATTR" => Some(234u8),
                        #[cfg(feature = "obj-term")] "ATTR_RESET" => Some(235u8),
                        #[cfg(feature = "obj-term")] "CURSOR_SAVE" => Some(236u8),
                        #[cfg(feature = "obj-term")] "CURSOR_RESTORE" => Some(237u8),
                        #[cfg(feature = "obj-term")] "TERM_COLS%" => Some(238u8),
                        #[cfg(feature = "obj-term")] "TERM_ROWS%" => Some(239u8),
                        #[cfg(feature = "obj-term")] "CURSOR_HIDE" => Some(241u8),
                        #[cfg(feature = "obj-term")] "CURSOR_SHOW" => Some(242u8),
                        #[cfg(feature = "obj-term")] "TERM_ERR$" => Some(243u8),
                        // Phase 2 additions
                        #[cfg(feature = "obj-term")] "TERM.INIT" => Some(244u8),
                        #[cfg(feature = "obj-term")] "TERM.END" => Some(245u8),
                        #[cfg(feature = "obj-term")] "TERM.RAW" => Some(246u8),
                        #[cfg(feature = "obj-term")] "ALTSCREEN_ON" => Some(247u8),
                        #[cfg(feature = "obj-term")] "ALTSCREEN_OFF" => Some(248u8),
                        #[cfg(feature = "obj-term")] "TERM.FLUSH" => Some(249u8),
                        #[cfg(feature = "obj-term")] "TERM.POLLKEY$" => Some(250u8),
                        // --- Audio/MIDI/DAW builtins ---
                        #[cfg(feature = "obj-daw")] "DAW_STOP" => Some(180u8),
                        #[cfg(feature = "obj-daw")] "DAW_ERR$" => Some(181u8),
                        #[cfg(feature = "obj-daw")] "AUDIO_RECORD%" => Some(182u8),
                        #[cfg(feature = "obj-daw")] "AUDIO_PLAY%" => Some(183u8),
                        #[cfg(feature = "obj-daw")] "AUDIO_MONITOR%" => Some(184u8),
                        #[cfg(feature = "obj-daw")] "MIDI_CAPTURE%" => Some(185u8),
                        #[cfg(feature = "obj-daw")] "SYNTH_LIVE%" => Some(186u8),
                        #[cfg(feature = "obj-daw")] "DAW_RESET" => Some(187u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_OUTPUTS$" => Some(190u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_INPUTS$" => Some(191u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_DEFAULT_RATE%" => Some(192u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_DEFAULT_CHANS%" => Some(193u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_OPEN_IN@" => Some(194u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_OPEN_OUT@" => Some(195u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_START%" => Some(196u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_STOP%" => Some(197u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_CLOSE%" => Some(198u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_RING_CREATE@" => Some(199u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_RING_PUSH%" => Some(200u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_RING_POP%" => Some(201u8),
                        #[cfg(feature = "obj-audio")] "WAV_WRITER_OPEN@" => Some(202u8),
                        #[cfg(feature = "obj-audio")] "WAV_WRITER_WRITE%" => Some(203u8),
                        #[cfg(feature = "obj-audio")] "WAV_WRITER_CLOSE%" => Some(204u8),
                        #[cfg(feature = "obj-audio")] "WAV_READ_ALL![]" => Some(205u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_CONNECT_IN_TO_RING%" => Some(206u8),
                        #[cfg(feature = "obj-audio")] "AUDIO_CONNECT_RING_TO_OUT%" => Some(207u8),
                        #[cfg(feature = "obj-audio")] "SYNTH_NEW@" => Some(220u8),
                        #[cfg(feature = "obj-audio")] "SYNTH_NOTE_ON%" => Some(221u8),
                        #[cfg(feature = "obj-audio")] "SYNTH_NOTE_OFF%" => Some(222u8),
                        #[cfg(feature = "obj-audio")] "SYNTH_RENDER%" => Some(223u8),
                        #[cfg(feature = "obj-audio")] "SYNTH_DELETE%" => Some(224u8),
                        #[cfg(feature = "obj-midi")]  "MIDI_PORTS$" => Some(210u8),
                        #[cfg(feature = "obj-midi")]  "MIDI_OPEN_IN@" => Some(211u8),
                        #[cfg(feature = "obj-midi")]  "MIDI_POLL%" => Some(212u8),
                        #[cfg(feature = "obj-midi")]  "MIDI_GET_EVENT$[]" => Some(213u8),
                        #[cfg(feature = "obj-midi")]  "MIDI_CLOSE%" => Some(214u8),
                        "ARRAY_ROWS%" => Some(139u8),
                        "ARRAY_COLS%" => Some(140u8),
                        _ => None,
                    };
                    if let Some(id) = bid {
                        for a in args { self.emit_expr_in(chunk, a, env)?; }
                        chunk.push_op(Op::Builtin); chunk.push_u8(id); chunk.push_u8(args.len() as u8);
                        return Ok(());
                    }
                    // If not builtin, treat as array access when not a known function
                    if args.len() >= 1 && args.len() <= 4 {
                        let is_func = self.routines.contains_key(&uname);
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
                // Special case: TERM.* as builtins when parsed as member-get callee
                if let Expr::MemberGet { target, name } = &**callee {
                    if let Expr::Var(tn) = &**target {
                        if tn.to_ascii_uppercase() == "TERM" {
                            let m = name.to_ascii_uppercase();
                            let bid_opt = match &*m {
                                #[cfg(feature = "obj-term")] "INIT" => Some(244u8),
                                #[cfg(feature = "obj-term")] "END" => Some(245u8),
                                #[cfg(feature = "obj-term")] "RAW" => Some(246u8),
                                #[cfg(feature = "obj-term")] "FLUSH" => Some(249u8),
                                #[cfg(feature = "obj-term")] "POLLKEY$" => Some(250u8),
                                _ => None,
                            };
                            if let Some(bid) = bid_opt {
                                for a in args { self.emit_expr_in(chunk, a, env)?; }
                                chunk.push_op(Op::Builtin); chunk.push_u8(bid); chunk.push_u8(args.len() as u8);
                                return Ok(());
                            }
                        }
                    }
                }
                // Regular call
                self.emit_expr_in(chunk, callee, env)?;
                for a in args { self.emit_expr_in(chunk, a, env)?; }
                chunk.push_op(Op::Call); chunk.push_u8(args.len() as u8);
            }
            Expr::MemberGet { target, name } => {
                // Allow zero-arg TERM.* calls written without parentheses (e.g., TERM.INIT;)
                if let Expr::Var(tn) = &**target {
                    if tn.to_ascii_uppercase() == "TERM" {
                        let m = name.to_ascii_uppercase();
                        let bid_opt = match &*m {
                            #[cfg(feature = "obj-term")] "INIT" => Some(244u8),
                            #[cfg(feature = "obj-term")] "END" => Some(245u8),
                            #[cfg(feature = "obj-term")] "FLUSH" => Some(249u8),
                            #[cfg(feature = "obj-term")] "POLLKEY$" => Some(250u8),
                            _ => None,
                        };
                        if let Some(bid) = bid_opt {
                            chunk.push_op(Op::Builtin); chunk.push_u8(bid); chunk.push_u8(0u8);
                            return Ok(());
                        }
                    }
                }
                self.emit_expr_in(chunk, target, env)?;
                let ci = chunk.add_const(Value::Str(name.clone()));
                chunk.push_op(Op::GetProp); chunk.push_u16(ci);
            }
            Expr::MemberCall { target, method, args } => {
                // Map TERM.* member-call forms to builtins
                if let Expr::Var(tn) = &**target {
                    if tn.to_ascii_uppercase() == "TERM" {
                        let m = method.to_ascii_uppercase();
                        let bid_opt = match &*m {
                            #[cfg(feature = "obj-term")] "INIT" => Some(244u8),
                            #[cfg(feature = "obj-term")] "END" => Some(245u8),
                            #[cfg(feature = "obj-term")] "RAW" => Some(246u8),
                            #[cfg(feature = "obj-term")] "FLUSH" => Some(249u8),
                            #[cfg(feature = "obj-term")] "POLLKEY$" => Some(250u8),
                            _ => None,
                        };
                        if let Some(bid) = bid_opt {
                            for a in args { self.emit_expr_in(chunk, a, env)?; }
                            chunk.push_op(Op::Builtin); chunk.push_u8(bid); chunk.push_u8(args.len() as u8);
                            return Ok(());
                        }
                    }
                }
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

    // Build a boolean Expr that matches any of the case patterns against a selector variable name
    fn build_case_cond_expr(&self, tmp_name: &str, patterns: &Vec<basil_ast::CasePattern>) -> Expr {
        // Start with false; OR in each pattern
        let mut cond_opt: Option<Expr> = None;
        for p in patterns {
            let one = match p {
                basil_ast::CasePattern::Value(v) => Expr::Binary { op: BinOp::Eq, lhs: Box::new(Expr::Var(tmp_name.to_string())), rhs: Box::new(v.clone()) },
                basil_ast::CasePattern::Range { lo, hi } => {
                    let ge = Expr::Binary { op: BinOp::Ge, lhs: Box::new(Expr::Var(tmp_name.to_string())), rhs: Box::new(lo.clone()) };
                    let le = Expr::Binary { op: BinOp::Le, lhs: Box::new(Expr::Var(tmp_name.to_string())), rhs: Box::new(hi.clone()) };
                    Expr::Binary { op: BinOp::And, lhs: Box::new(ge), rhs: Box::new(le) }
                }
                basil_ast::CasePattern::Compare { op, rhs } => Expr::Binary { op: *op, lhs: Box::new(Expr::Var(tmp_name.to_string())), rhs: Box::new(rhs.clone()) },
            };
            cond_opt = Some(match cond_opt {
                None => one,
                Some(prev) => Expr::Binary { op: BinOp::Or, lhs: Box::new(prev), rhs: Box::new(one) },
            });
        }
        cond_opt.unwrap_or(Expr::Bool(false))
    }

    fn emit_select_case_tl_into(&mut self, chunk: &mut Chunk, selector: &Expr, arms: &Vec<basil_ast::CaseArm>, else_body: &Option<Vec<Stmt>>) -> Result<()> {
        // Evaluate selector once into a hidden global
        let tmp_name = "\u{0001}SEL#TMP".to_string();
        self.emit_expr_in(chunk, selector, None)?;
        let g = self.gslot(&tmp_name);
        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
        let mut end_jumps: Vec<usize> = Vec::new();
        let mut next_labels: Vec<usize> = Vec::new();
        for arm in arms {
            // next label for this arm (from previous arm's jf)
            let here = chunk.here();
            for site in std::mem::take(&mut next_labels) { let off = (here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
            // condition
            let cond = self.build_case_cond_expr(&tmp_name, &arm.patterns);
            self.emit_expr_in(chunk, &cond, None)?;
            chunk.push_op(Op::JumpIfFalse);
            let jf = chunk.emit_u16_placeholder();
            // body
            for s in &arm.body { self.emit_stmt_tl_in_chunk(chunk, s)?; }
            // jump to end
            chunk.push_op(Op::Jump);
            let jend = chunk.emit_u16_placeholder();
            end_jumps.push(jend);
            // record where to patch for next arm
            next_labels.push(jf);
        }
        // After last arm, patch next_labels to current position
        let after_arms = chunk.here();
        for site in next_labels { let off = (after_arms - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
        // else body
        if let Some(body) = else_body {
            for s in body { self.emit_stmt_tl_in_chunk(chunk, s)?; }
        }
        // end label
        let end_here = chunk.here();
        for j in end_jumps { let off = (end_here - (j + 2)) as u16; chunk.patch_u16_at(j, off); }
        Ok(())
    }

    fn emit_select_case_func(&mut self, chunk: &mut Chunk, selector: &Expr, arms: &Vec<basil_ast::CaseArm>, else_body: &Option<Vec<Stmt>>, env: &mut LocalEnv) -> Result<()> {
        // Evaluate selector once into a hidden local
        let tmp_name = "\u{0001}SEL#TMP".to_string();
        let slot = env.bind_next_if_absent(tmp_name.clone());
        self.emit_expr_in(chunk, selector, Some(env))?;
        chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
        let mut end_jumps: Vec<usize> = Vec::new();
        let mut next_labels: Vec<usize> = Vec::new();
        for arm in arms {
            // next label for this arm (from previous arm's jf)
            let here = chunk.here();
            for site in std::mem::take(&mut next_labels) { let off = (here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
            // condition
            let cond = self.build_case_cond_expr(&tmp_name, &arm.patterns);
            self.emit_expr_in(chunk, &cond, Some(env))?;
            chunk.push_op(Op::JumpIfFalse);
            let jf = chunk.emit_u16_placeholder();
            // body
            for s in &arm.body { self.emit_stmt_func(chunk, s, env)?; }
            // jump to end
            chunk.push_op(Op::Jump);
            let jend = chunk.emit_u16_placeholder();
            end_jumps.push(jend);
            // record where to patch for next arm
            next_labels.push(jf);
        }
        // After last arm, patch next_labels to current position
        let after_arms = chunk.here();
        for site in next_labels { let off = (after_arms - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
        // else body
        if let Some(body) = else_body {
            for s in body { self.emit_stmt_func(chunk, s, env)?; }
        }
        // end label
        let end_here = chunk.here();
        for j in end_jumps { let off = (end_here - (j + 2)) as u16; chunk.patch_u16_at(j, off); }
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
            // SETENV/EXPORTENV
            Stmt::SetEnv { name, value, export } => {
                let nci = chunk.add_const(Value::Str(name.clone()));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                self.emit_expr_in(chunk, value, None)?;
                let eci = chunk.add_const(Value::Bool(*export));
                chunk.push_op(Op::Const); chunk.push_u16(eci);
                chunk.push_op(Op::Builtin); chunk.push_u8(59u8); chunk.push_u8(3u8);
                chunk.push_op(Op::Pop);
            }
            // SHELL
            Stmt::Shell { cmd } => {
                self.emit_expr_in(chunk, cmd, None)?;
                chunk.push_op(Op::Builtin); chunk.push_u8(60u8); chunk.push_u8(1u8);
                chunk.push_op(Op::Pop);
            }
            // EXIT
            Stmt::Exit(code_opt) => {
                if let Some(e) = code_opt { self.emit_expr_in(chunk, e, None)?; }
                else { let ci = chunk.add_const(Value::Int(0)); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                chunk.push_op(Op::Builtin); chunk.push_u8(61u8); chunk.push_u8(1u8);
            }
            // Unstructured flow inside toplevel chunk: support LABEL/GOTO
            Stmt::Label(name) => {
                let pos = chunk.here();
                if self.tl_labels.insert(name.clone(), pos).is_some() {
                    return Err(BasilError(format!("Duplicate label: {}", name)));
                }
            }
            Stmt::Goto(name) => {
                chunk.push_op(Op::Jump);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.tl_goto_fixups.push((op_pos, u16_pos, name.clone()));
            }
            Stmt::Gosub(name) => {
                chunk.push_op(Op::Gosub);
                let op_pos = chunk.here() - 1;
                let u16_pos = chunk.emit_u16_placeholder();
                self.tl_gosub_fixups.push((op_pos, u16_pos, name.clone()));
            }
            Stmt::ReturnFromGosub(lbl_opt) => {
                match lbl_opt {
                    None => { chunk.push_op(Op::GosubRet); }
                    Some(label) => {
                        chunk.push_op(Op::GosubPop);
                        chunk.push_op(Op::Jump);
                        let op_pos = chunk.here() - 1;
                        let u16_pos = chunk.emit_u16_placeholder();
                        self.tl_goto_fixups.push((op_pos, u16_pos, label.clone()));
                    }
                }
            }
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
            Stmt::SelectCase { selector, arms, else_body } => {
                self.emit_select_case_tl_into(chunk, selector, arms, else_body)?;
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
            Stmt::Func { name, params, body, .. } => {
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
