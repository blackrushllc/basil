//! Minimal IR for bcc AOT path. Intentionally tiny to support hello-world and
//! a few simple statements; can be expanded incrementally.

use basil_frontend::ast;

#[derive(Debug, Clone)]
pub enum Ty { Int, Bool, Str, ObjHandle }

#[derive(Debug, Clone)]
pub enum Instr {
    Print(Box<Expr>),
    For { var: String, start: Expr, end: Expr, step: Option<Expr>, body: Vec<Instr> },
    // In future: Call { target: String, args: Vec<Expr> }, Return(Option<Expr>), etc.
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Str(String),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    // Future: Temp(u32), BinOp, Cmp, Concat, etc.
}

#[derive(Debug, Clone, Default)]
pub struct Function {
    pub body: Vec<Instr>,
}

#[derive(Debug, Clone, Default)]
pub struct Module { pub main: Function }

fn lower_expr(e: &ast::Expr) -> Expr {
    match e {
        ast::Expr::Number(n) => Expr::Int(*n as i64),
        ast::Expr::Str(s) => Expr::Str(s.clone()),
        ast::Expr::Bool(b) => Expr::Bool(*b),
        ast::Expr::Var(name) => Expr::Var(name.clone()),
        ast::Expr::Binary { op, lhs, rhs } => {
            use ast::BinOp::*;
            match op {
                Add => Expr::Add(Box::new(lower_expr(lhs)), Box::new(lower_expr(rhs))),
                _ => Expr::Str(format!("{:?}", e)),
            }
        }
        _ => Expr::Str(format!("{:?}", e)),
    }
}

fn lower_stmt(stmt: &ast::Stmt, out: &mut Vec<Instr>) {
    match stmt {
        ast::Stmt::Print { expr } => {
            out.push(Instr::Print(Box::new(lower_expr(expr))));
        }
        ast::Stmt::For { var, start, end, step, body } => {
            // Lower body
            let mut inner = Vec::new();
            if let ast::Stmt::Block(stmts) = &**body {
                for s in stmts { lower_stmt(s, &mut inner); }
            } else {
                lower_stmt(body, &mut inner);
            }
            out.push(Instr::For {
                var: var.clone(),
                start: lower_expr(start),
                end: lower_expr(end),
                step: step.as_ref().map(|e| lower_expr(e)),
                body: inner,
            });
        }
        ast::Stmt::Block(stmts) => {
            for s in stmts { lower_stmt(s, out); }
        }
        _ => {
            // Ignore unsupported statements for now
        }
    }
}

/// Lower from AST to our IR with basic support for Print and For/Next and string concatenation.
pub fn lower_to_ir(prog: &ast::Program) -> Module {
    let mut m = Module::default();
    for stmt in prog { lower_stmt(stmt, &mut m.main.body); }
    m
}
