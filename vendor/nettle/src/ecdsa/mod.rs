//! Elliptic curve variant of the Digital Signature Standard.

mod keys;
pub use self::keys::generate_keypair;

mod sign;
pub use self::sign::{sign, verify};
