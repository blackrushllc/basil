//! Broken hash functions.
//!
//! The algorithms in this module are known to be broken or too short to be secure. Only use them
//! for legacy applications.

mod sha1;
pub use self::sha1::Sha1;
mod md2;
pub use self::md2::Md2;
mod md4;
pub use self::md4::Md4;
mod md5;
pub use self::md5::Md5;
mod ripemd160;
pub use self::ripemd160::Ripemd160;
mod gosthash94;
pub use self::gosthash94::GostHash94;
