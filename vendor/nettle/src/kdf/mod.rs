//! Key derivation functions

mod hkdf;
pub use self::hkdf::hkdf;

mod pbkdf2;
pub use self::pbkdf2::pbkdf2;
