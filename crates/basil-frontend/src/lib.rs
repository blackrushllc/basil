//! Shared frontend facade for Basil: re-exports lexer/parser/AST and basic helpers.
//! This crate is intentionally thin so both basilc (VM) and bcc (AOT) can share
//! the same parsing pipeline without duplicating code.

pub use basil_ast as ast;
pub use basil_lexer as lexer;
pub use basil_parser as parser;

pub use basil_common::{BasilError, Result};

/// Parse a Basil source string into an AST Program using the canonical parser.
pub fn parse_program(src: &str) -> Result<ast::Program> {
    parser::parse(src)
}
