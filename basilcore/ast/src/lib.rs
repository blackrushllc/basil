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
    // Implicit receiver inside WITH block
    ImplicitThis,
    // NEW TYPE(args) expression
    NewObject { type_name: String, args: Vec<Expr> },
    // CLASS("filename") expression
    NewClass { filename: Box<Expr> },
    // EVAL("expr") expression: parse+compile at runtime and push result
    Eval(Box<Expr>),
    // New: list and dictionary literals, and square-bracket indexing
    List(Vec<Expr>),
    Dict(Vec<(String, Expr)>),
    IndexSquare { target: Box<Expr>, index: Box<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
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
    // Fixed-length string declaration: DIM name$ AS STRING * N  or  DIM name$[N]
    DimFixedStr { name: String, len: usize },
    // TYPE ... END TYPE (struct definition)
    TypeDef { name: String, fields: Vec<StructField> },
    // Property set: obj.Prop = expr (without LET)
    SetProp { target: Expr, prop: String, value: Expr },
    // Square-bracket index set: list[i] = expr or dict["k"] = expr
    SetIndexSquare { target: Expr, index: Expr, value: Expr },
    // DESCRIBE obj or array
    Describe { target: Expr },
    Print { expr: Expr },
    // EXEC statement: EXEC("...basil code...")
    Exec { code: Expr },
    // SETENV/EXPORTENV statements
    SetEnv { name: String, value: Expr, export: bool },
    // SHELL statement
    Shell { cmd: Expr },
    // EXIT statement (optional numeric code)
    Exit(Option<Expr>),
    // STOP statement: suspend execution
    Stop,
    ExprStmt(Expr),
    // Function return (inside FUNC)
    Return(Option<Expr>),
    // GOSUB/RETURN control (RETURN; or RETURN TO <label>)
    ReturnFromGosub(Option<String>),
    // Labels and unstructured flow
    Label(String),
    Goto(String),
    Gosub(String),
    If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
    While { cond: Expr, body: Box<Stmt> },
    Break,
    Continue,
    Block(Vec<Stmt>),
    Func { kind: FuncKind, name: String, params: Vec<String>, body: Vec<Stmt> },
    For { var: String, start: Expr, end: Expr, step: Option<Expr>, body: Box<Stmt> },
    // FOR EACH var IN expr ... NEXT
    ForEach { var: String, enumerable: Expr, body: Box<Stmt> },
    // SELECT CASE statement
    SelectCase { selector: Expr, arms: Vec<CaseArm>, else_body: Option<Vec<Stmt>> },
    // WITH block
    With { target: Expr, body: Vec<Stmt> },
    // TRY/CATCH/FINALLY
    Try { try_body: Vec<Stmt>, catch_var: Option<String>, catch_body: Option<Vec<Stmt>>, finally_body: Option<Vec<Stmt>> },
    // RAISE statement
    Raise(Option<Expr>),
    // Line marker for runtime error reporting
    Line(u32),
}

#[derive(Debug, Clone)]
pub struct CaseArm {
    pub patterns: Vec<CasePattern>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum CasePattern {
    Value(Expr),
    Range { lo: Expr, hi: Expr },
    Compare { op: BinOp, rhs: Expr },
}

pub type Program = Vec<Stmt>;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncKind {
    Func,
    Sub,
}


#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub kind: StructFieldKind,
}

#[derive(Debug, Clone)]
pub enum StructFieldKind {
    Int32,
    Float64,
    VarString,              // variable-length string (handle/pointer at runtime)
    FixedString(usize),     // fixed-length string with declared byte size
    Struct(String),         // nested struct by type name
}
