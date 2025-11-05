//! Block and stream ciphers.

pub mod insecure_do_not_use;

mod cipher;
pub use self::cipher::{BlockSizeIs16, Cipher, RawCipherFunctionPointer};

mod aes128;
pub use self::aes128::Aes128;
mod aes192;
pub use self::aes192::Aes192;
mod aes256;
pub use self::aes256::Aes256;

mod blowfish;
pub use self::blowfish::Blowfish;

mod camellia128;
pub use self::camellia128::Camellia128;
mod camellia192;
pub use self::camellia192::Camellia192;
mod camellia256;
pub use self::camellia256::Camellia256;

mod cast128;
pub use self::cast128::Cast128;

mod chacha;
pub use self::chacha::ChaCha;

mod des3;
pub use self::des3::Des3;

mod salsa20;
pub use self::salsa20::{Salsa20_128, Salsa20_256};
mod salsa20r12;
pub use self::salsa20r12::{Salsa20R12_128, Salsa20R12_256};

mod serpent;
pub use self::serpent::Serpent;

mod twofish;
pub use self::twofish::Twofish;
