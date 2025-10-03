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

//! AST for Basil v0 — functions, calls, returns, if/blocks, comparisons

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Bool(bool),
    Var(String),
    UnaryNeg(Box<Expr>),
    UnaryNot(Box<Expr>),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    // Postfix parentheses used for either function calls or array indexing (disambiguated in compiler)
    Call { callee: Box<Expr>, args: Vec<Expr> },
    // Object member access and method calls
    MemberGet { target: Box<Expr>, name: String },
    MemberCall { target: Box<Expr>, method: String, args: Vec<Expr> },
    // NEW TYPE(args) expression
    NewObject { type_name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    // LET for variables or array elements (if indices present)
    Let   { name: String, indices: Option<Vec<Expr>>, init: Expr },
    // DIM statement to create arrays (1–4 dimensions)
    Dim   { name: String, dims: Vec<Expr> },
    // DIM x@ AS TYPE(args?) (scalar object)
    DimObject { name: String, type_name: String, args: Vec<Expr> },
    // DIM arr@(dims) [AS Type] (object arrays)
    DimObjectArray { name: String, dims: Vec<Expr>, type_name: Option<String> },
    // Property set: obj.Prop = expr (without LET)
    SetProp { target: Expr, prop: String, value: Expr },
    // DESCRIBE obj or array
    Describe { target: Expr },
    Print { expr: Expr },
    ExprStmt(Expr),
    Return(Option<Expr>),
    If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
    Block(Vec<Stmt>),
    Func { name: String, params: Vec<String>, body: Vec<Stmt> },
    For { var: String, start: Expr, end: Expr, step: Option<Expr>, body: Box<Stmt> },
    // FOR EACH var IN expr ... NEXT
    ForEach { var: String, enumerable: Expr, body: Box<Stmt> },
}

pub type Program = Vec<Stmt>;
