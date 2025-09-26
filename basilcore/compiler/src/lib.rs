//! AST → bytecode compiler (tiny subset)
use std::collections::HashMap;
use basil_common::{Result};
use basil_ast::{Program, Stmt, Expr, BinOp};
use basil_bytecode::{Chunk, Program as BCProgram, Value, Op};

pub fn compile(ast: &Program) -> Result<BCProgram> {
    let mut c = C { chunk: Chunk::default(), globals: Vec::new(), gmap: HashMap::new() };
    for s in ast { c.emit_stmt(s)?; }
    c.chunk.push_op(Op::Halt);
    Ok(BCProgram { chunk: c.chunk, globals: c.globals })
}

struct C {
    chunk: Chunk,
    globals: Vec<String>,
    gmap: HashMap<String, u8>,
}

impl C {
    fn gslot(&mut self, name: &str) -> u8 {
        if let Some(&i) = self.gmap.get(name) { return i; }
        let i = self.globals.len() as u8;
        self.globals.push(name.to_string());
        self.gmap.insert(name.to_string(), i);
        i
    }

    fn emit_stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let { name, init } => {
                self.emit_expr(init)?;
                let idx = self.gslot(name);
                self.chunk.push_op(Op::StoreGlobal);
                self.chunk.push_u8(idx);
            }
            Stmt::Print { expr } => {
                self.emit_expr(expr)?;
                self.chunk.push_op(Op::Print);
            }
            Stmt::ExprStmt(e) => {
                self.emit_expr(e)?;
                self.chunk.push_op(Op::Pop);
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, e: &Expr) -> Result<()> {
        match e {
            Expr::Number(n) => {
                let idx = self.chunk.add_const(Value::Num(*n));
                self.chunk.push_op(Op::ConstU8);
                self.chunk.push_u8(idx);
            }
            Expr::Str(s) => {
                let idx = self.chunk.add_const(Value::Str(s.clone()));
                self.chunk.push_op(Op::ConstU8);
                self.chunk.push_u8(idx);
            }
            Expr::Var(name) => {
                let idx = self.gslot(name);
                self.chunk.push_op(Op::LoadGlobal);
                self.chunk.push_u8(idx);
            }
            Expr::UnaryNeg(inner) => {
                self.emit_expr(inner)?;
                self.chunk.push_op(Op::Neg);
            }
            Expr::Binary { op, lhs, rhs } => {
                self.emit_expr(lhs)?;
                self.emit_expr(rhs)?;
                self.chunk.push_op(match op {
                    BinOp::Add => Op::Add,
                    BinOp::Sub => Op::Sub,
                    BinOp::Mul => Op::Mul,
                    BinOp::Div => Op::Div,
                });
            }
        }
        Ok(())
    }
}
