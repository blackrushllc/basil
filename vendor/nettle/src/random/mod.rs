//! Cryptographic random number generators (CRNG).

mod types;
pub use self::types::Random;

mod yarrow;
pub use self::yarrow::Yarrow;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub use self::tests::TestRandom;
