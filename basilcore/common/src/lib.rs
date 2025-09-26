#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span { pub start: u32, pub end: u32 }
impl Span { pub fn new(start: usize, end: usize) -> Self { Self { start: start as u32, end: end as u32 } } }


#[derive(Debug)]
pub struct BasilError(pub String);
impl std::fmt::Display for BasilError { fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl std::error::Error for BasilError {}


pub type Result<T> = std::result::Result<T, BasilError>;