//! Minimal IR for bcc AOT path. Intentionally tiny to support hello-world and
//! a few simple statements; can be expanded incrementally.

use basil_frontend::ast;

#[derive(Debug, Clone)]
pub enum Ty { Int, Bool, Str, ObjHandle }

#[derive(Debug, Clone)]
pub enum Instr {
    Print(Box<Expr>),
    For { var: String, start: Expr, end: Expr, step: Option<Expr>, body: Vec<Instr> },
    Assign { var: String, expr: Expr },
    If { cond: Expr, then_body: Vec<Instr>, else_body: Vec<Instr> },
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Str(String),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
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
                Ne  => Expr::Ne(Box::new(lower_expr(lhs)), Box::new(lower_expr(rhs))),
                _ => Expr::Str(format!("{:?}", e)),
            }
        }
        ast::Expr::Call { callee, args } => {
            if let ast::Expr::Var(name) = &**callee {
                let args2: Vec<Expr> = args.iter().map(|a| lower_expr(a)).collect();
                Expr::Call(name.clone(), args2)
            } else {
                Expr::Str(format!("{:?}", e))
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
        ast::Stmt::Let { name, indices, init } => {
            if indices.is_none() {
                out.push(Instr::Assign { var: name.clone(), expr: lower_expr(init) });
            }
        }
        ast::Stmt::If { cond, then_branch, else_branch } => {
            // Lower then branch
            let mut then_vec = Vec::new();
            match &**then_branch {
                ast::Stmt::Block(stmts) => { for s in stmts { lower_stmt(s, &mut then_vec); } }
                other => { lower_stmt(other, &mut then_vec); }
            }
            // Lower else branch (optional)
            let mut else_vec = Vec::new();
            if let Some(eb) = else_branch {
                match &**eb {
                    ast::Stmt::Block(stmts) => { for s in stmts { lower_stmt(s, &mut else_vec); } }
                    other => { lower_stmt(other, &mut else_vec); }
                }
            }
            out.push(Instr::If { cond: lower_expr(cond), then_body: then_vec, else_body: else_vec });
        }
        ast::Stmt::ExprStmt(e) => {
            out.push(Instr::ExprStmt(lower_expr(e)));
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
