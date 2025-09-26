//! AST for Basil v0

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Var(String),
    UnaryNeg(Box<Expr>),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp { Add, Sub, Mul, Div }

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, init: Expr },
    Print { expr: Expr },
    ExprStmt(Expr),
}

pub type Program = Vec<Stmt>;
