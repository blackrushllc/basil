//! Cryptographic hash functions.

/// Emits a Write implementation for hashes.
macro_rules! impl_write_for_hash {
    ($($name:tt)*) => {
        impl ::std::io::Write for $($name)* {
            fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize> {
                self.update(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> ::std::io::Result<()> {
                Ok(())
            }
        }
    };
}

pub mod insecure_do_not_use;

mod hash;
pub use self::hash::{Hash, NettleHash};

mod sha224;
pub use self::sha224::Sha224;
mod sha256;
pub use self::sha256::Sha256;

mod sha512_224;
pub use self::sha512_224::Sha512_224;
mod sha512_256;
pub use self::sha512_256::Sha512_256;
mod sha384;
pub use self::sha384::Sha384;
mod sha512;
pub use self::sha512::Sha512;

mod sha3_224;
pub use self::sha3_224::Sha3_224;
mod sha3_256;
pub use self::sha3_256::Sha3_256;
mod sha3_384;
pub use self::sha3_384::Sha3_384;
mod sha3_512;
pub use self::sha3_512::Sha3_512;
