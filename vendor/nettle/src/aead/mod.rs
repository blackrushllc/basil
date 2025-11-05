//! Authenticated encryption mode with associated data.

mod aead;
pub use self::aead::Aead;
use self::aead::{AdStream, AeadInternal};

mod eax;
pub use self::eax::Eax;
mod ocb;
pub use self::ocb::{Ocb, OCB_IS_SUPPORTED, typenum};
mod gcm;
pub use self::gcm::Gcm;
mod ccm;
pub use self::ccm::Ccm;
mod chacha_poly1305;
pub use self::chacha_poly1305::ChaChaPoly1305;
