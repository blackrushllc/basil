//! Implementation of Sequoia crypto API using the OpenSSL cryptographic library.

use crate::types::*;

pub mod aead;
pub mod asymmetric;
pub mod ecdh;
pub mod hash;
pub mod symmetric;

pub struct Backend(());

impl super::interface::Backend for Backend {
    fn backend() -> String {
        "OpenSSL".to_string()
    }

    fn random(buf: &mut [u8]) -> crate::Result<()> {
        openssl::rand::rand_bytes(buf)?;
        Ok(())
    }
}

impl AEADAlgorithm {
    /// Returns the best AEAD mode supported by the backend.
    ///
    /// This SHOULD return OCB, which is the mandatory-to-implement
    /// algorithm and the most performing one, but fall back to any
    /// supported algorithm.
    pub(crate) const fn const_default() -> AEADAlgorithm {
        if cfg!(not(osslconf = "OPENSSL_NO_OCB")) {
            AEADAlgorithm::OCB
        } else {
            AEADAlgorithm::GCM
        }
    }

    pub(crate) fn is_supported_by_backend(&self) -> bool {
        match self {
            AEADAlgorithm::EAX => false,
            AEADAlgorithm::OCB => cfg!(not(osslconf = "OPENSSL_NO_OCB")),
            AEADAlgorithm::GCM => true,
            AEADAlgorithm::Private(_) |
            AEADAlgorithm::Unknown(_) => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn supports_symmetric_algo(&self, algo: &SymmetricAlgorithm) -> bool {
        match &self {
            AEADAlgorithm::EAX => false,
            AEADAlgorithm::OCB =>
                match algo {
                    // OpenSSL supports OCB only with AES
                    // see: https://wiki.openssl.org/index.php/OCB
                    SymmetricAlgorithm::AES128 |
                    SymmetricAlgorithm::AES192 |
                    SymmetricAlgorithm::AES256 => true,
                    _ => false,
                },
            AEADAlgorithm::GCM =>
                match algo {
                    // OpenSSL supports GCM only with AES
                    // see: https://wiki.openssl.org/index.php/GCM
                    SymmetricAlgorithm::AES128 |
                    SymmetricAlgorithm::AES192 |
                    SymmetricAlgorithm::AES256 => true,
                    _ => false,
                },
            AEADAlgorithm::Private(_) |
            AEADAlgorithm::Unknown(_) => false,
        }
    }
}
