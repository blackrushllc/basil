use serde::{Serialize, Deserialize};

use basil_parser::parse;
use basil_ast::{Program, Stmt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity { Error, Warning, Information }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind { Function, Variable, Label }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilerDiagnostics {
    pub errors: Vec<Diagnostic>,
    pub symbols: Vec<SymbolInfo>,
}

pub fn analyze_source(source: &str, _filename: &str) -> CompilerDiagnostics {
    let mut out = CompilerDiagnostics::default();
    match parse(source) {
        Ok(ast) => {
            // Collect simple symbol information from AST
            collect_symbols(&ast, &mut out.symbols);
        }
        Err(e) => {
            // Parser returns a BasilError string; no location info available here → use line 0
            out.errors.push(Diagnostic {
                message: format!("{}", e),
                line: 0,
                column: 0,
                severity: DiagnosticSeverity::Error,
            });
        }
    }
    out
}

fn collect_symbols(ast: &Program, syms: &mut Vec<SymbolInfo>) {
    for s in ast {
        match s {
            Stmt::Func { name, .. } => {
                syms.push(SymbolInfo { name: name.clone(), kind: SymbolKind::Function, line: 0, col: 0 });
            }
            Stmt::Let { name, .. } => {
                syms.push(SymbolInfo { name: name.clone(), kind: SymbolKind::Variable, line: 0, col: 0 });
            }
            Stmt::Label(lbl) => {
                syms.push(SymbolInfo { name: lbl.clone(), kind: SymbolKind::Label, line: 0, col: 0 });
            }
            _ => {}
        }
    }
}
