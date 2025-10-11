//! Minimal runtime API used by AOT-emitted Rust. This is a local development
//! crate; published builds will use the crates.io version.

use std::fmt;

#[derive(Clone)]
pub struct Str(String);

impl Str {
    pub fn from_static(s: &'static str) -> Str { Str(s.to_string()) }
    pub fn from_string(s: String) -> Str { Str(s) }
}

impl fmt::Display for Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Clone)]
pub enum Val { Int(i64), Bool(bool), Str(Str) /* , Obj(ObjHandle) */ }

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Int(i) => write!(f, "{}", i),
            Val::Bool(b) => write!(f, "{}", b),
            Val::Str(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug)]
pub struct RtError(String);

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

impl std::error::Error for RtError {}

pub type RtResult<T> = Result<T, RtError>;

pub fn print(v: &Val) -> RtResult<()> { print!("{}", v); Ok(()) }
pub fn println(v: &Val) -> RtResult<()> { println!("{}", v); Ok(()) }

pub fn input_line(prompt: &Str) -> Str {
    use std::io::{self, Write};
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    if s.ends_with('\n') { s.pop(); if s.ends_with('\r') { s.pop(); } }
    Str::from_string(s)
}

pub mod features {
    // Modules are gated by Cargo features; keep empty shims for now.
    // In a real published crate, these would expose thin, monomorphic APIs.
    #[cfg(feature = "audio")] pub mod audio {}
    #[cfg(feature = "midi")]  pub mod midi {}
    #[cfg(feature = "daw")]   pub mod daw {}
    #[cfg(feature = "term")]  pub mod term {}
}
