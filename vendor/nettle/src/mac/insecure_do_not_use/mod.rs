//! Broken message authentication codes.
//!
//! The algorithms in this module are known to be broken or too short to be secure. Only use them
//! for legacy applications.

mod umac;
pub use self::umac::{Umac128, Umac32, Umac64, Umac96};
