//! Implementation of crypto primitives using the Windows CNG (Cryptographic API: Next Generation).

use crate::types::*;

use win_crypto_ng::random::RandomNumberGenerator;

pub mod aead;
pub mod asymmetric;
pub mod ecdh;
pub mod hash;
pub mod symmetric;

pub struct Backend(());

impl super::interface::Backend for Backend {
    fn backend() -> String {
        // XXX: can we include features and the version?
        "Windows CNG".to_string()
    }

    fn random(buf: &mut [u8]) -> crate::Result<()> {
        RandomNumberGenerator::system_preferred()
            .gen_random(buf)?;
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
        AEADAlgorithm::EAX
    }

    pub(crate) fn is_supported_by_backend(&self) -> bool {
        use self::AEADAlgorithm::*;
        match &self {
            EAX => true,
            OCB => false,
            GCM => true,
            Private(_) | Unknown(_)
                => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn supports_symmetric_algo(&self, algo: &SymmetricAlgorithm) -> bool {
        match &self {
            AEADAlgorithm::EAX =>
                match algo {
                    SymmetricAlgorithm::AES128 |
                    SymmetricAlgorithm::AES192 |
                    SymmetricAlgorithm::AES256 |
                    SymmetricAlgorithm::Twofish |
                    SymmetricAlgorithm::Camellia128 |
                    SymmetricAlgorithm::Camellia192 |
                    SymmetricAlgorithm::Camellia256 => true,
                    _ => false,
                },
            _ => false
        }
    }
}
