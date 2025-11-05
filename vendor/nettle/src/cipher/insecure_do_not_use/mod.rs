//! Insecure ciphers.
//!
//! The algorithms in this module are known to be broken. Only use them for legacy applications.

mod arcfour;
pub use self::arcfour::ArcFour;
mod arctwo;
pub use self::arctwo::ArcTwo;

mod des;
pub use self::des::Des;
