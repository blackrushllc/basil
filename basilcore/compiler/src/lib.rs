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
        Expr::Eval(inner) => expr_contains_sub_call(routines, inner),
        _ => false,
    }
}

struct C {
    cur_line: u32,
    chunk: Chunk,
    globals: Vec<String>,
    gmap: HashMap<String, u8>,
    fn_names: HashSet<String>,
    routines: HashMap<String, RoutineInfo>,
    loop_stack: Vec<LoopCtx>,
    // WITH support
    with_counter: u32,
    with_stack_tl: Vec<String>,
    with_stack_fn: Vec<String>,
    // Explicit tracking of active WITH target names (stack)
    with_current_stack: Vec<String>,
    // Label/GOTO support (top-level)
    tl_labels: HashMap<String, usize>,
    tl_goto_fixups: Vec<(usize, usize, String)>, // (op_pos, u16_pos, label)
    tl_gosub_fixups: Vec<(usize, usize, String)>,
    // Label/GOTO support (current function)
    fn_labels: HashMap<String, usize>,
    fn_goto_fixups: Vec<(usize, usize, String)>,
    fn_gosub_fixups: Vec<(usize, usize, String)>,
    // Recorded TYPE definitions: map uppercase type name -> fields
    struct_types: HashMap<String, Vec<basil_ast::StructField>>,
    // Fixed-length string metadata: globals
    fixed_globs: HashMap<String, usize>,
    // Struct variable type bindings for globals
    var_struct_globs: HashMap<String, String>,            // var -> TypeName (upper)
    // Arrays of struct element type bindings (globals)
    var_struct_array_globs: HashMap<String, String>,
}

impl C {
    fn new() -> Self {
        Self {
            cur_line: 0,
            chunk: Chunk::default(),
            with_current_stack: Vec::new(),
            globals: Vec::new(),
            gmap: HashMap::new(),
            fn_names: HashSet::new(),
            routines: HashMap::new(),
            loop_stack: Vec::new(),
            with_counter: 0,
            with_stack_tl: Vec::new(),
            with_stack_fn: Vec::new(),
            tl_labels: HashMap::new(),
            tl_goto_fixups: Vec::new(),
            tl_gosub_fixups: Vec::new(),
            fn_labels: HashMap::new(),
            fn_goto_fixups: Vec::new(),
            fn_gosub_fixups: Vec::new(),
            struct_types: HashMap::new(),
            fixed_globs: HashMap::new(),
            var_struct_globs: HashMap::new(),
            var_struct_array_globs: HashMap::new(),
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

            Stmt::TypeDef { name, fields } => {
                // Record TYPE definition for later struct variable initializations
                let key = name.to_ascii_uppercase();
                self.struct_types.insert(key, fields.clone());
                // Emit STRUCT_REG(name$, spec$)
                let mut chunk = std::mem::take(&mut self.chunk);
                let spec = {
                    let mut s = String::new();
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 { s.push(';'); }
                        s.push_str(&f.name);
                        s.push('|');
                        match &f.kind {
                            basil_ast::StructFieldKind::Int32 => s.push('I'),
                            basil_ast::StructFieldKind::Float64 => s.push('F'),
                            basil_ast::StructFieldKind::VarString => s.push('S'),
                            basil_ast::StructFieldKind::FixedString(n) => { s.push('X'); s.push('|'); s.push_str(&n.to_string()); },
                            basil_ast::StructFieldKind::Struct(tn) => { s.push('T'); s.push('|'); s.push_str(tn); },
                        }
                    }
                    s
                };
                let nci = chunk.add_const(Value::Str(name.clone()));
                let sci = chunk.add_const(Value::Str(spec));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                chunk.push_op(Op::Const); chunk.push_u16(sci);
                // call STRUCT_REG (161)
                chunk.push_op(Op::Builtin); chunk.push_u8(161u8); chunk.push_u8(2u8);
                chunk.push_op(Op::Pop);
                self.chunk = chunk;
            }

            Stmt::DimFixedStr { name, len } => {
                // Record metadata and initialize to empty string
                self.fixed_globs.insert(name.clone(), *len);
                let mut chunk = std::mem::take(&mut self.chunk);
                let ci = chunk.add_const(Value::Str(String::new()));
                chunk.push_op(Op::Const); chunk.push_u16(ci);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.chunk = chunk;
            }

            // Top-level LET/PRINT/EXPR: move chunk out to avoid &mut self + &mut self.chunk alias.
            Stmt::Let { name, indices, init } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                match indices {
                    None => {
                        // Detect struct <-> string pack/unpack first
                        if let Some(ty_s) = self.var_struct_globs.get(name).cloned() {
                            // LET <struct> = <string>
                            if let Expr::Var(rn) = init {
                                if rn.ends_with('$') {
                                    // push RHS string, then type name, call STRUCT_UNPACK (164)
                                    self.emit_expr_in(&mut chunk, init, None)?;
                                    let tci = chunk.add_const(Value::Str(ty_s.clone()));
                                    chunk.push_op(Op::Const); chunk.push_u16(tci);
                                    chunk.push_op(Op::Builtin); chunk.push_u8(164u8); chunk.push_u8(2u8);
                                    let g = self.gslot(name);
                                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                                    self.chunk = chunk; return Ok(());
                                }
                            }
                        }
                        if name.ends_with('$') {
                            // LET <string> = <struct> ? pack
                            if let Expr::Var(rn) = init {
                                if let Some(ty_s) = self.var_struct_globs.get(rn).cloned() {
                                    self.emit_expr_in(&mut chunk, init, None)?; // push dict
                                    let tci = chunk.add_const(Value::Str(ty_s.clone()));
                                    chunk.push_op(Op::Const); chunk.push_u16(tci);
                                    chunk.push_op(Op::Builtin); chunk.push_u8(163u8); chunk.push_u8(2u8);
                                    let g = self.gslot(name);
                                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                                    self.chunk = chunk; return Ok(());
                                }
                            }
                        }
                        // Regular assignment with possible fixed-length enforcement and % coercion
                        self.emit_expr_in(&mut chunk, init, None)?;
                        if let Some(n) = self.fixed_globs.get(name) {
                            let ci = chunk.add_const(Value::Int(*n as i64));
                            chunk.push_op(Op::Const); chunk.push_u16(ci);
                            chunk.push_op(Op::Builtin); chunk.push_u8(160u8); chunk.push_u8(2u8);
                        } else if name.ends_with('%') {
                            chunk.push_op(Op::ToInt);
                        }
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
                let tci = if let Some(tn) = type_name {
                    // Record global array element type binding
                    self.var_struct_array_globs.insert(name.clone(), tn.to_ascii_uppercase());
                    chunk.add_const(Value::Str(tn.clone()))
                } else { 65535u16 };
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
            Stmt::Exec { code } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, code, None)?;
                chunk.push_op(Op::ExecString);
                self.chunk = chunk;
            }
            Stmt::Describe { target } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, target, None)?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
                self.chunk = chunk;
            }
            Stmt::With { target, body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, target, None)?;
                let name = format!("\u{0001}WITH#TMP{}", self.with_counter);
                self.with_counter += 1;
                let g = self.gslot(&name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.with_stack_tl.push(name.clone());
                self.with_current_stack.push(name.clone());
                for s2 in body { self.emit_stmt_tl_in_chunk(&mut chunk, s2)?; }
                self.with_current_stack.pop();
                self.with_stack_tl.pop();
                self.chunk = chunk;
            }
            Stmt::Raise(expr_opt) => {
                let mut chunk = std::mem::take(&mut self.chunk);
                match expr_opt {
                    Some(e) => { self.emit_expr_in(&mut chunk, e, None)?; chunk.push_op(Op::Raise); }
                    None => { chunk.push_op(Op::Reraise); }
                }
                self.chunk = chunk;
            }
            Stmt::Try { try_body, catch_var, catch_body, finally_body } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                let has_catch = catch_body.is_some();
                let has_finally = finally_body.is_some();
                // Enter TRY region
                chunk.push_op(Op::TryPush);
                let hp = chunk.emit_u16_placeholder();
                let fp = chunk.emit_u16_placeholder();
                // TRY body
                for s2 in try_body { self.emit_stmt_tl_in_chunk(&mut chunk, s2)?; }
                // Normal completion path
                let mut j_after_sites: Vec<usize> = Vec::new();
                let mut j_to_finally_norm: Option<usize> = None;
                if has_finally {
                    chunk.push_op(Op::Jump);
                    j_to_finally_norm = Some(chunk.emit_u16_placeholder());
                } else {
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    j_after_sites.push(chunk.emit_u16_placeholder());
                }
                // Handler label
                let handler_here = chunk.here();
                let off_h = (handler_here - (fp + 2)) as u16; chunk.patch_u16_at(hp, off_h);
                // We don't use VM-run finally; compile-time handles finally
                chunk.patch_u16_at(fp, 0);
                // Handler code
                let mut j_to_finally_exc: Option<usize> = None;
                if has_catch {
                    if let Some(name) = catch_var {
                        let g = self.gslot(name);
                        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                    } else {
                        chunk.push_op(Op::Pop);
                    }
                    if let Some(body) = catch_body {
                        for s2 in body { self.emit_stmt_tl_in_chunk(&mut chunk, s2)?; }
                    }
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::TryPop);
                        chunk.push_op(Op::Jump);
                        j_after_sites.push(chunk.emit_u16_placeholder());
                    }
                } else {
                    // no catch: run finally (if any) then rethrow
                    chunk.push_op(Op::Pop); // discard message on stack
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::Reraise);
                    }
                }
                // FINALLY blocks (duplicated for normal/exceptional)
                if let Some(fbody) = finally_body {
                    // finally (normal)
                    if let Some(site) = j_to_finally_norm { let here = chunk.here(); let off = (here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                    for s2 in fbody { self.emit_stmt_tl_in_chunk(&mut chunk, s2)?; }
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    let j_after_from_finally_norm = chunk.emit_u16_placeholder();
                    // finally (exception)
                    if let Some(site) = j_to_finally_exc { let here2 = chunk.here(); let off2 = (here2 - (site + 2)) as u16; chunk.patch_u16_at(site, off2); }
                    for s2 in fbody { self.emit_stmt_tl_in_chunk(&mut chunk, s2)?; }
                    chunk.push_op(Op::TryPop);
                    if has_catch {
                        chunk.push_op(Op::Jump);
                        let j_after_from_finally_exc = chunk.emit_u16_placeholder();
                        // After label
                        let after_here = chunk.here();
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                        let offe = (after_here - (j_after_from_finally_exc + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_exc, offe);
                    } else {
                        // rethrow after finally
                        chunk.push_op(Op::Reraise);
                        // After label for normal path only: allow normal path to skip exceptional FINALLY+RERAISE
                        let after_here = chunk.here();
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                    }
                } else {
                    // No FINALLY: create after label to land normal/catch paths
                    let after_here = chunk.here();
                    for site in j_after_sites { let off = (after_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                }
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
            // STOP
            Stmt::Stop => {
                let mut chunk = std::mem::take(&mut self.chunk);
                chunk.push_op(Op::Stop);
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
                let key = type_name.to_ascii_uppercase();
                // Record global struct variable type binding
                self.var_struct_globs.insert(name.clone(), key.clone());
                if let Some(fields) = self.struct_types.get(&key) {
                    // Build a dictionary of default field values
                    for f in fields.iter() {
                        let kci = chunk.add_const(Value::Str(f.name.clone()));
                        chunk.push_op(Op::Const); chunk.push_u16(kci);
                        match &f.kind {
                            basil_ast::StructFieldKind::Int32 => {
                                let ci = chunk.add_const(Value::Int(0));
                                chunk.push_op(Op::Const); chunk.push_u16(ci);
                            }
                            basil_ast::StructFieldKind::Float64 => {
                                let ci = chunk.add_const(Value::Num(0.0));
                                chunk.push_op(Op::Const); chunk.push_u16(ci);
                            }
                            basil_ast::StructFieldKind::VarString | basil_ast::StructFieldKind::FixedString(_) => {
                                let ci = chunk.add_const(Value::Str(String::new()));
                                chunk.push_op(Op::Const); chunk.push_u16(ci);
                            }
                            basil_ast::StructFieldKind::Struct(_) => {
                                // empty dict for nested struct
                                chunk.push_op(Op::Builtin); chunk.push_u8(252u8); chunk.push_u8(0u8);
                            }
                        }
                    }
                    let argc = (fields.len() * 2) as u8;
                    chunk.push_op(Op::Builtin); chunk.push_u8(252u8); chunk.push_u8(argc);
                    let g = self.gslot(name);
                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                } else {
                    // Fallback: object instance via registry
                    for a in args { self.emit_expr_in(&mut chunk, a, None)?; }
                    let tci = chunk.add_const(Value::Str(type_name.clone()));
                    chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
                    let g = self.gslot(name);
                    chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                }
                self.chunk = chunk;
            }
            Stmt::SetProp { target, prop, value } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                // push target object/dict first
                self.emit_expr_in(&mut chunk, target, None)?;
                // Determine field coercion if target is a known struct type
                let mut coerce_to_int = false;
                let mut fixed_n_opt: Option<usize> = None;
                if let Some(ty) = self.resolve_struct_type_of_expr(target) {
                    if let Some(kind) = self.field_kind_of(&ty, prop) {
                        match kind {
                            basil_ast::StructFieldKind::Int32 => coerce_to_int = true,
                            basil_ast::StructFieldKind::FixedString(n) => fixed_n_opt = Some(n),
                            _ => {}
                        }
                    }
                }
                // push value and apply coercions if needed
                self.emit_expr_in(&mut chunk, value, None)?;
                if coerce_to_int { chunk.push_op(Op::ToInt); }
                if let Some(n) = fixed_n_opt {
                    let ci = chunk.add_const(Value::Int(n as i64));
                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                    chunk.push_op(Op::Builtin); chunk.push_u8(160u8); chunk.push_u8(2u8);
                }
                // property name const index
                let pci = chunk.add_const(Value::Str(prop.clone()));
                chunk.push_op(Op::SetProp); chunk.push_u16(pci);
                self.chunk = chunk;
            }
            Stmt::SetIndexSquare { target, index, value } => {
                let mut chunk = std::mem::take(&mut self.chunk);
                self.emit_expr_in(&mut chunk, target, None)?;
                self.emit_expr_in(&mut chunk, index, None)?;
                self.emit_expr_in(&mut chunk, value, None)?;
                chunk.push_op(Op::Builtin); chunk.push_u8(254u8); chunk.push_u8(3u8);
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
                self.cur_line = *line;
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
                        // Detect struct <-> string conversions first
                        if let Some(ty_s) = env.var_struct.get(name).cloned().or_else(|| self.var_struct_globs.get(name).cloned()) {
                            if let Expr::Var(rn) = init {
                                if rn.ends_with('$') {
                                    // UNPACK: push rhs, type name, call 164
                                    self.emit_expr_in(chunk, init, Some(env))?;
                                    let tci = chunk.add_const(Value::Str(ty_s.clone()));
                                    chunk.push_op(Op::Const); chunk.push_u16(tci);
                                    chunk.push_op(Op::Builtin); chunk.push_u8(164u8); chunk.push_u8(2u8);
                                    // store to local/global using same logic
                                    if let Some(slot) = env.lookup(name) {
                                        chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                                    } else if self.gmap.contains_key(name) && !self.routines.contains_key(&name.to_ascii_uppercase()) {
                                        let g = self.gslot(name); chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                                    } else {
                                        let slot = env.bind_next_if_absent(name.clone()); chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                                    }
                                    return Ok(());
                                }
                            }
                        }
                        if name.ends_with('$') {
                            if let Expr::Var(rn) = init {
                                if let Some(ty_s) = env.var_struct.get(rn).cloned().or_else(|| self.var_struct_globs.get(rn).cloned()) {
                                    // PACK: push dict, type name, call 163
                                    self.emit_expr_in(chunk, init, Some(env))?;
                                    let tci = chunk.add_const(Value::Str(ty_s.clone()));
                                    chunk.push_op(Op::Const); chunk.push_u16(tci);
                                    chunk.push_op(Op::Builtin); chunk.push_u8(163u8); chunk.push_u8(2u8);
                                    if let Some(slot) = env.lookup(name) {
                                        chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                                    } else if self.gmap.contains_key(name) && !self.routines.contains_key(&name.to_ascii_uppercase()) {
                                        let g = self.gslot(name); chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                                    } else {
                                        let slot = env.bind_next_if_absent(name.clone()); chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                                    }
                                    return Ok(());
                                }
                            }
                        }
                        // Regular assignment with fixed-string/int coercion
                        self.emit_expr_in(chunk, init, Some(env))?;
                        if let Some(n) = env.fixed.get(name).copied() {
                            let ci = chunk.add_const(Value::Int(n as i64));
                            chunk.push_op(Op::Const); chunk.push_u16(ci);
                            chunk.push_op(Op::Builtin); chunk.push_u8(160u8); chunk.push_u8(2u8);
                        } else if name.ends_with('%') { chunk.push_op(Op::ToInt); }
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
            Stmt::DimFixedStr { name, len } => {
                // record local fixed-length string and init to empty
                env.fixed.insert(name.clone(), *len);
                let ci = chunk.add_const(Value::Str(String::new()));
                chunk.push_op(Op::Const); chunk.push_u16(ci);
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::DimObjectArray { name, dims, type_name } => {
                for d in dims { self.emit_expr_in(chunk, d, Some(env))?; }
                chunk.push_op(Op::ArrMake); chunk.push_u8(dims.len() as u8);
                chunk.push_u8(3u8);
                let tci: u16 = if let Some(tn) = type_name {
                    env.var_struct_array.insert(name.clone(), tn.to_ascii_uppercase());
                    chunk.add_const(Value::Str(tn.clone()))
                } else { 65535u16 };
                chunk.push_u16(tci);
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::Print { expr } => {
                self.emit_expr_in(chunk, expr, Some(env))?;
                chunk.push_op(Op::Print);
            }
            Stmt::Exec { code } => {
                self.emit_expr_in(chunk, code, Some(env))?;
                chunk.push_op(Op::ExecString);
            }
            Stmt::Describe { target } => {
                self.emit_expr_in(chunk, target, Some(env))?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
            }
            Stmt::TypeDef { name, fields } => {
                // Record TYPE definitions inside functions as well and register at runtime
                let key = name.to_ascii_uppercase();
                self.struct_types.insert(key, fields.clone());
                let spec = {
                    let mut s = String::new();
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 { s.push(';'); }
                        s.push_str(&f.name);
                        s.push('|');
                        match &f.kind {
                            basil_ast::StructFieldKind::Int32 => s.push('I'),
                            basil_ast::StructFieldKind::Float64 => s.push('F'),
                            basil_ast::StructFieldKind::VarString => s.push('S'),
                            basil_ast::StructFieldKind::FixedString(n) => { s.push('X'); s.push('|'); s.push_str(&n.to_string()); },
                            basil_ast::StructFieldKind::Struct(tn) => { s.push('T'); s.push('|'); s.push_str(tn); },
                        }
                    }
                    s
                };
                let nci = chunk.add_const(Value::Str(name.clone()));
                let sci = chunk.add_const(Value::Str(spec));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                chunk.push_op(Op::Const); chunk.push_u16(sci);
                chunk.push_op(Op::Builtin); chunk.push_u8(161u8); chunk.push_u8(2u8);
                chunk.push_op(Op::Pop);
            }
            Stmt::With { target, body } => {
                // Evaluate target once into a hidden local and push with-scope
                self.emit_expr_in(chunk, target, Some(env))?;
                let name = format!("\u{0001}WITH#TMP{}", self.with_counter);
                self.with_counter += 1;
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                self.with_stack_fn.push(name.clone());
                self.with_current_stack.push(name.clone());
                for s2 in body { self.emit_stmt_func(chunk, s2, env)?; }
                self.with_current_stack.pop();
                self.with_stack_fn.pop();
            }
            Stmt::Raise(expr_opt) => {
                match expr_opt {
                    Some(e) => { self.emit_expr_in(chunk, e, Some(env))?; chunk.push_op(Op::Raise); }
                    None => { chunk.push_op(Op::Reraise); }
                }
            }
            Stmt::Try { try_body, catch_var, catch_body, finally_body } => {
                let has_catch = catch_body.is_some();
                let has_finally = finally_body.is_some();
                // Enter TRY region
                chunk.push_op(Op::TryPush);
                let hp = chunk.emit_u16_placeholder();
                let fp = chunk.emit_u16_placeholder();
                // TRY body
                for s2 in try_body { self.emit_stmt_func(chunk, s2, env)?; }
                // Normal completion path
                let mut j_after_sites: Vec<usize> = Vec::new();
                let mut j_to_finally_norm: Option<usize> = None;
                let j_after_from_normal: Option<usize> = None;
                if has_finally {
                    chunk.push_op(Op::Jump);
                    j_to_finally_norm = Some(chunk.emit_u16_placeholder());
                } else {
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    j_after_sites.push(chunk.emit_u16_placeholder());
                }
                // Handler label
                let handler_here = chunk.here();
                let off_h = (handler_here - (fp + 2)) as u16; chunk.patch_u16_at(hp, off_h);
                // Don't use VM-run finally
                chunk.patch_u16_at(fp, 0);
                // Handler code
                let mut j_to_finally_exc: Option<usize> = None;
                if has_catch {
                    if let Some(name) = catch_var {
                        let slot = env.bind_next_if_absent(name.clone());
                        chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
                    } else {
                        chunk.push_op(Op::Pop);
                    }
                    if let Some(body) = catch_body {
                        for s2 in body { self.emit_stmt_func(chunk, s2, env)?; }
                    }
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::TryPop);
                        chunk.push_op(Op::Jump);
                        j_after_sites.push(chunk.emit_u16_placeholder());
                    }
                } else {
                    // no catch: run finally (if any) then rethrow
                    chunk.push_op(Op::Pop);
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::Reraise);
                    }
                }
                // FINALLY blocks (duplicated)
                if let Some(fbody) = finally_body {
                    // finally (normal)
                    if let Some(site) = j_to_finally_norm { let here = chunk.here(); let off = (here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                    for s2 in fbody { self.emit_stmt_func(chunk, s2, env)?; }
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    let j_after_from_finally_norm = chunk.emit_u16_placeholder();
                    // finally (exception)
                    if let Some(site) = j_to_finally_exc { let here2 = chunk.here(); let off2 = (here2 - (site + 2)) as u16; chunk.patch_u16_at(site, off2); }
                    for s2 in fbody { self.emit_stmt_func(chunk, s2, env)?; }
                    chunk.push_op(Op::TryPop);
                    if has_catch {
                        chunk.push_op(Op::Jump);
                        let j_after_from_finally_exc = chunk.emit_u16_placeholder();
                        // After label
                        let after_here = chunk.here();
                        if let Some(site) = j_after_from_normal { let off = (after_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                        let offe = (after_here - (j_after_from_finally_exc + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_exc, offe);
                    } else {
                        // rethrow after finally
                        chunk.push_op(Op::Reraise);
                        // After (normal only): allow normal path to skip exceptional FINALLY+RERAISE
                        let after_here = chunk.here();
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                    }
                } else {
                    let after_here = chunk.here();
                    if let Some(site) = j_after_from_normal { let off = (after_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                }
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
            // STOP inside function
            Stmt::Stop => {
                chunk.push_op(Op::Stop);
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
                let key = type_name.to_ascii_uppercase();
                // record local struct var binding
                env.var_struct.insert(name.clone(), key.clone());
                if let Some(fields) = self.struct_types.get(&key) {
                    // Build a dictionary of default field values
                    for f in fields.iter() {
                        let kci = chunk.add_const(Value::Str(f.name.clone()));
                        chunk.push_op(Op::Const); chunk.push_u16(kci);
                        match &f.kind {
                            basil_ast::StructFieldKind::Int32 => { let ci = chunk.add_const(Value::Int(0)); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                            basil_ast::StructFieldKind::Float64 => { let ci = chunk.add_const(Value::Num(0.0)); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                            basil_ast::StructFieldKind::VarString | basil_ast::StructFieldKind::FixedString(_) => { let ci = chunk.add_const(Value::Str(String::new())); chunk.push_op(Op::Const); chunk.push_u16(ci); }
                            basil_ast::StructFieldKind::Struct(_) => { chunk.push_op(Op::Builtin); chunk.push_u8(252u8); chunk.push_u8(0u8); }
                        }
                    }
                    let argc = (fields.len() * 2) as u8;
                    chunk.push_op(Op::Builtin); chunk.push_u8(252u8); chunk.push_u8(argc);
                } else {
                    for a in args { self.emit_expr_in(chunk, a, Some(env))?; }
                    let tci = chunk.add_const(Value::Str(type_name.clone()));
                    chunk.push_op(Op::NewObj); chunk.push_u16(tci); chunk.push_u8(args.len() as u8);
                }
                let slot = env.bind_next_if_absent(name.clone());
                chunk.push_op(Op::StoreLocal); chunk.push_u8(slot);
            }
            Stmt::SetProp { target, prop, value } => {
                // push target first
                self.emit_expr_in(chunk, target, Some(env))?;
                // Determine field coercion
                let mut coerce_to_int = false;
                let mut fixed_n_opt: Option<usize> = None;
                if let Some(ty) = self.resolve_struct_type_of_expr_in_fn(target, env) {
                    if let Some(kind) = self.field_kind_of(&ty, prop) {
                        match kind {
                            basil_ast::StructFieldKind::Int32 => coerce_to_int = true,
                            basil_ast::StructFieldKind::FixedString(n) => fixed_n_opt = Some(n),
                            _ => {}
                        }
                    }
                }
                // push value and apply
                self.emit_expr_in(chunk, value, Some(env))?;
                if coerce_to_int { chunk.push_op(Op::ToInt); }
                if let Some(n) = fixed_n_opt {
                    let ci = chunk.add_const(Value::Int(n as i64));
                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                    chunk.push_op(Op::Builtin); chunk.push_u8(160u8); chunk.push_u8(2u8);
                }
                let pci = chunk.add_const(Value::Str(prop.clone()));
                chunk.push_op(Op::SetProp); chunk.push_u16(pci);
            }
            Stmt::SetIndexSquare { target, index, value } => {
                self.emit_expr_in(chunk, target, Some(env))?;
                self.emit_expr_in(chunk, index, Some(env))?;
                self.emit_expr_in(chunk, value, Some(env))?;
                chunk.push_op(Op::Builtin); chunk.push_u8(254u8); chunk.push_u8(3u8);
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
                self.cur_line = *line;
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
            Expr::List(items) => {
                // Evaluate items left-to-right, then call MAKE_LIST builtin with argc
                for it in items { self.emit_expr_in(chunk, it, env)?; }
                chunk.push_op(Op::Builtin); chunk.push_u8(251u8); chunk.push_u8(items.len() as u8);
            }
            Expr::Dict(entries) => {
                // Push key (string const) then value expr for each entry; call MAKE_DICT with 2*len args
                for (k, v) in entries {
                    let ki = chunk.add_const(Value::Str(k.clone()));
                    chunk.push_op(Op::Const); chunk.push_u16(ki);
                    self.emit_expr_in(chunk, v, env)?;
                }
                let argc = (entries.len() * 2) as u8;
                chunk.push_op(Op::Builtin); chunk.push_u8(252u8); chunk.push_u8(argc);
            }
            Expr::IndexSquare { target, index } => {
                self.emit_expr_in(chunk, target, env)?;
                self.emit_expr_in(chunk, index, env)?;
                chunk.push_op(Op::Builtin); chunk.push_u8(253u8); chunk.push_u8(2u8);
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
                    // Special handling for LEN for compile-time folding of fixed strings and struct sizes
                    if uname == "LEN" && args.len() == 1 {
                        // Try folding cases in order
                        let a0 = &args[0];
                        // 1) LEN(fixedVar$)
                        if let Expr::Var(vn) = a0 {
                            if let Some(env) = env {
                                if let Some(&n) = env.fixed.get(vn) {
                                    let ci = chunk.add_const(Value::Int(n as i64));
                                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                                    return Ok(());
                                }
                            }
                            if let Some(&n) = self.fixed_globs.get(vn) {
                                let ci = chunk.add_const(Value::Int(n as i64));
                                chunk.push_op(Op::Const); chunk.push_u16(ci);
                                return Ok(());
                            }
                            // 2) LEN(varStruct) where var is known struct type with fixed size
                            let ty_opt = if let Some(env) = env { env.var_struct.get(vn).cloned() } else { None }
                                .or_else(|| self.var_struct_globs.get(vn).cloned());
                            if let Some(tyu) = ty_opt {
                                if let Some(sz) = self.compute_struct_fixed_size(&tyu) {
                                    let ci = chunk.add_const(Value::Int(sz as i64));
                                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                                    return Ok(());
                                }
                            }
                            // 3) LEN(TypeName) — if looks like a type (no var bound and exists in struct_types)
                            let is_var_bound = if let Some(env) = env { env.lookup(vn).is_some() } else { false } || self.gmap.contains_key(vn);
                            let tkey = vn.to_ascii_uppercase();
                            if !is_var_bound {
                                if self.struct_types.contains_key(&tkey) {
                                    let sz = self.compute_struct_fixed_size(&tkey).unwrap_or(0);
                                    let ci = chunk.add_const(Value::Int(sz as i64));
                                    chunk.push_op(Op::Const); chunk.push_u16(ci);
                                    return Ok(());
                                }
                            }
                        }
                        // 4) LEN(rec.Field$) when Field is fixed-length
                        if let Expr::MemberGet { target, name } = a0 {
                            // resolve type of target
                            let ty_opt = if let Some(env) = env { self.resolve_struct_type_of_expr_in_fn(target, env) } else { self.resolve_struct_type_of_expr(target) };
                            if let Some(tyu) = ty_opt {
                                if let Some(k) = self.field_kind_of(&tyu, name) {
                                    if let basil_ast::StructFieldKind::FixedString(n) = k {
                                        let ci = chunk.add_const(Value::Int(n as i64));
                                        chunk.push_op(Op::Const); chunk.push_u16(ci);
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        // Fallback: emit normal LEN builtin
                    }
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
                        "LOADENV%" => Some(63u8),
                        "MKDIRS%" => Some(62u8),
                        #[cfg(feature = "obj-base64")] "BASE64_ENCODE$" => Some(90u8),
                        #[cfg(feature = "obj-base64")] "BASE64_DECODE$" => Some(91u8),
                        #[cfg(feature = "obj-zip")] "ZIP_EXTRACT_ALL" => Some(120u8),
                        #[cfg(feature = "obj-zip")] "ZIP_COMPRESS_FILE" => Some(121u8),
                        #[cfg(feature = "obj-zip")] "ZIP_COMPRESS_DIR" => Some(122u8),
                        #[cfg(feature = "obj-zip")] "ZIP_LIST$" => Some(123u8),
                        #[cfg(feature = "obj-zip")] "ZIP_ARRAY$" => Some(135u8),
                        #[cfg(feature = "obj-zip")] "ZIP_ARRAY$[]" => Some(135u8),
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
            Expr::Eval(inner) => {
                self.emit_expr_in(chunk, inner, env)?;
                chunk.push_op(Op::EvalString);
            }
            Expr::ImplicitThis => {
                // Prefer explicit current WITH target if present
                if let Some(nm) = self.with_current_stack.last().cloned() {
                    match env {
                        Some(env) => {
                            if let Some(slot) = env.lookup(&nm) {
                                chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                            } else {
                                let g = self.gslot(&nm);
                                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                            }
                        }
                        None => {
                            let g = self.gslot(&nm);
                            chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                        }
                    }
                } else {
                    // Load the current WITH target from legacy stacks as fallback; if none, error at compile time
                    match env {
                        Some(env) => {
                            if let Some(nm) = self.with_stack_fn.last().cloned() {
                                if let Some(slot) = env.lookup(&nm) {
                                    chunk.push_op(Op::LoadLocal); chunk.push_u8(slot);
                                } else {
                                    let g = self.gslot(&nm);
                                    chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                                }
                            } else {
                                return Err(BasilError(format!("Leading '.' member requires a WITH block (at line {})", self.cur_line)));
                            }
                        }
                        None => {
                            if let Some(nm) = self.with_stack_tl.last().cloned() {
                                let g = self.gslot(&nm);
                                chunk.push_op(Op::LoadGlobal); chunk.push_u8(g);
                            } else {
                                return Err(BasilError(format!("Leading '.' member requires a WITH block (at line {})", self.cur_line)));
                            }
                        }
                    }
                }
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
    // function-scope metadata
    fixed: HashMap<String, usize>,                 // local fixed-length strings
    var_struct: HashMap<String, String>,           // local struct vars: var -> TypeName (upper)
    var_struct_array: HashMap<String, String>,     // local arrays of struct: var -> ElemTypeName (upper)
}
impl LocalEnv {
    fn new() -> Self { Self { map: HashMap::new(), next: 0, fixed: HashMap::new(), var_struct: HashMap::new(), var_struct_array: HashMap::new() } }
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
            Stmt::Exec { code } => {
                self.emit_expr_in(chunk, code, None)?;
                chunk.push_op(Op::ExecString);
            }
            Stmt::Describe { target } => {
                self.emit_expr_in(chunk, target, None)?;
                chunk.push_op(Op::DescribeObj);
                chunk.push_op(Op::Print);
            }
            Stmt::TypeDef { name, fields } => {
                let key = name.to_ascii_uppercase();
                self.struct_types.insert(key, fields.clone());
                // Emit STRUCT_REG(name$, spec$) at top-level in-chunk
                let spec = {
                    let mut s = String::new();
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 { s.push(';'); }
                        s.push_str(&f.name);
                        s.push('|');
                        match &f.kind {
                            basil_ast::StructFieldKind::Int32 => s.push('I'),
                            basil_ast::StructFieldKind::Float64 => s.push('F'),
                            basil_ast::StructFieldKind::VarString => s.push('S'),
                            basil_ast::StructFieldKind::FixedString(n) => { s.push('X'); s.push('|'); s.push_str(&n.to_string()); },
                            basil_ast::StructFieldKind::Struct(tn) => { s.push('T'); s.push('|'); s.push_str(tn); },
                        }
                    }
                    s
                };
                let nci = chunk.add_const(Value::Str(name.clone()));
                let sci = chunk.add_const(Value::Str(spec));
                chunk.push_op(Op::Const); chunk.push_u16(nci);
                chunk.push_op(Op::Const); chunk.push_u16(sci);
                chunk.push_op(Op::Builtin); chunk.push_u8(161u8); chunk.push_u8(2u8);
                chunk.push_op(Op::Pop);
            }
            Stmt::DimFixedStr { name, len } => {
                // Record metadata and initialize to empty string (top-level in-chunk)
                self.fixed_globs.insert(name.clone(), *len);
                let ci = chunk.add_const(Value::Str(String::new()));
                chunk.push_op(Op::Const); chunk.push_u16(ci);
                let g = self.gslot(name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
            }
            Stmt::Raise(expr_opt) => {
                match expr_opt {
                    Some(e) => { self.emit_expr_in(chunk, e, None)?; chunk.push_op(Op::Raise); }
                    None => { chunk.push_op(Op::Reraise); }
                }
            }
            Stmt::Try { try_body, catch_var, catch_body, finally_body } => {
                let has_catch = catch_body.is_some();
                let has_finally = finally_body.is_some();
                // Enter TRY region
                chunk.push_op(Op::TryPush);
                let hp = chunk.emit_u16_placeholder();
                let fp = chunk.emit_u16_placeholder();
                // TRY body
                for s2 in try_body { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
                // Normal completion path
                let mut j_after_sites: Vec<usize> = Vec::new();
                let mut j_to_finally_norm: Option<usize> = None;
                if has_finally {
                    chunk.push_op(Op::Jump);
                    j_to_finally_norm = Some(chunk.emit_u16_placeholder());
                } else {
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    j_after_sites.push(chunk.emit_u16_placeholder());
                }
                // Handler label
                let handler_here = chunk.here();
                let off_h = (handler_here - (fp + 2)) as u16; chunk.patch_u16_at(hp, off_h);
                // Don't use VM-run finally
                chunk.patch_u16_at(fp, 0);
                // Handler code
                let mut j_to_finally_exc: Option<usize> = None;
                if has_catch {
                    if let Some(name) = catch_var {
                        let g = self.gslot(&name);
                        chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                    } else {
                        chunk.push_op(Op::Pop);
                    }
                    if let Some(body) = catch_body {
                        for s2 in body { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
                    }
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::TryPop);
                        chunk.push_op(Op::Jump);
                        j_after_sites.push(chunk.emit_u16_placeholder());
                    }
                } else {
                    // no catch: run finally (if any) then rethrow
                    chunk.push_op(Op::Pop);
                    if has_finally {
                        chunk.push_op(Op::Jump);
                        j_to_finally_exc = Some(chunk.emit_u16_placeholder());
                    } else {
                        chunk.push_op(Op::Reraise);
                    }
                }
                // FINALLY blocks (duplicated)
                if let Some(fbody) = finally_body {
                    // finally (normal)
                    if let Some(site) = j_to_finally_norm { let here = chunk.here(); let off = (here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                    for s2 in fbody { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
                    chunk.push_op(Op::TryPop);
                    chunk.push_op(Op::Jump);
                    let j_after_from_finally_norm = chunk.emit_u16_placeholder();
                    // finally (exception)
                    if let Some(site) = j_to_finally_exc { let here2 = chunk.here(); let off2 = (here2 - (site + 2)) as u16; chunk.patch_u16_at(site, off2); }
                    for s2 in fbody { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
                    chunk.push_op(Op::TryPop);
                    if has_catch {
                        chunk.push_op(Op::Jump);
                        let j_after_from_finally_exc = chunk.emit_u16_placeholder();
                        // After label
                        let after_here = chunk.here();
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                        let offe = (after_here - (j_after_from_finally_exc + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_exc, offe);
                    } else {
                        // rethrow after finally
                        chunk.push_op(Op::Reraise);
                        // After (normal only): allow normal path to skip exceptional FINALLY+RERAISE
                        let after_here = chunk.here();
                        let offn = (after_here - (j_after_from_finally_norm + 2)) as u16; chunk.patch_u16_at(j_after_from_finally_norm, offn);
                    }
                } else {
                    // No FINALLY: create after label to land normal/catch paths
                    let after_here = chunk.here();
                    for site in j_after_sites { let off = (after_here - (site + 2)) as u16; chunk.patch_u16_at(site, off); }
                }
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
            // STOP
            Stmt::Stop => {
                chunk.push_op(Op::Stop);
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
            Stmt::SetIndexSquare { target, index, value } => {
                self.emit_expr_in(chunk, target, None)?;
                self.emit_expr_in(chunk, index, None)?;
                self.emit_expr_in(chunk, value, None)?;
                chunk.push_op(Op::Builtin); chunk.push_u8(254u8); chunk.push_u8(3u8);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr_in(chunk, e, None)?;
                chunk.push_op(Op::Pop);
            }
            Stmt::Line(line) => {
                self.cur_line = *line;
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
            Stmt::With { target, body } => {
                // Evaluate target once and bind to a hidden global; make it the current implicit receiver
                self.emit_expr_in(chunk, target, None)?;
                let name = format!("\u{0001}WITH#TMP{}", self.with_counter);
                self.with_counter += 1;
                let g = self.gslot(&name);
                chunk.push_op(Op::StoreGlobal); chunk.push_u8(g);
                self.with_stack_tl.push(name.clone());
                self.with_current_stack.push(name.clone());
                for s2 in body { self.emit_stmt_tl_in_chunk(chunk, s2)?; }
                self.with_current_stack.pop();
                self.with_stack_tl.pop();
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


// --- Helpers for struct/fixed-string lowering ---
impl C {
    fn resolve_struct_type_of_expr(&self, e: &Expr) -> Option<String> {
        match e {
            Expr::Var(name) => self.var_struct_globs.get(name).cloned(),
            Expr::Call { callee, .. } => {
                if let Expr::Var(arr_name) = &**callee {
                    self.var_struct_array_globs.get(arr_name).cloned()
                } else { None }
            }
            Expr::MemberGet { target, name } => {
                if let Some(parent_ty) = self.resolve_struct_type_of_expr(target) {
                    if let Some(kind) = self.field_kind_of(&parent_ty, name) {
                        if let basil_ast::StructFieldKind::Struct(nested) = kind { return Some(nested.to_ascii_uppercase()); }
                    }
                }
                None
            }
            _ => None,
        }
    }
    fn field_kind_of(&self, type_upper: &str, field: &str) -> Option<basil_ast::StructFieldKind> {
        let uname = type_upper.to_ascii_uppercase();
        let fields = self.struct_types.get(&uname)?;
        for f in fields {
            if f.name.eq_ignore_ascii_case(field) { return Some(f.kind.clone()); }
        }
        None
    }
}


impl C {
    fn resolve_struct_type_of_expr_in_fn(&self, e: &Expr, env: &LocalEnv) -> Option<String> {
        match e {
            Expr::Var(name) => env.var_struct.get(name).cloned().or_else(|| self.var_struct_globs.get(name).cloned()),
            Expr::Call { callee, .. } => {
                if let Expr::Var(arr_name) = &**callee {
                    env.var_struct_array.get(arr_name).cloned().or_else(|| self.var_struct_array_globs.get(arr_name).cloned())
                } else { None }
            }
            Expr::MemberGet { target, name } => {
                if let Some(parent_ty) = self.resolve_struct_type_of_expr_in_fn(target, env) {
                    if let Some(kind) = self.field_kind_of(&parent_ty, name) {
                        if let basil_ast::StructFieldKind::Struct(nested) = kind { return Some(nested.to_ascii_uppercase()); }
                    }
                }
                None
            }
            _ => None,
        }
    }

    // Compute fixed size of a struct type if all fields are fixed-size; otherwise return None
    fn compute_struct_fixed_size(&self, type_upper: &str) -> Option<usize> {
        fn inner(this: &C, tyu: &str, seen: &mut HashSet<String>) -> Option<usize> {
            let key = tyu.to_ascii_uppercase();
            if !this.struct_types.contains_key(&key) { return None; }
            if seen.contains(&key) { return None; } // cycle
            seen.insert(key.clone());
            let mut total: usize = 0;
            for f in this.struct_types.get(&key).unwrap() {
                match &f.kind {
                    basil_ast::StructFieldKind::Int32 => total += 4,
                    basil_ast::StructFieldKind::Float64 => total += 8,
                    basil_ast::StructFieldKind::FixedString(n) => total += *n,
                    basil_ast::StructFieldKind::VarString => return None,
                    basil_ast::StructFieldKind::Struct(nm) => {
                        let sz = inner(this, &nm.to_ascii_uppercase(), seen)?;
                        total += sz;
                    }
                }
            }
            Some(total)
        }
        let mut seen: HashSet<String> = HashSet::new();
        inner(self, type_upper, &mut seen)
    }
}
