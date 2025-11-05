//! Implementation of Sequoia crypto API using a fuzzing-friendly null
//! backend.

use crate::types::*;

#[allow(unused_variables)]
pub mod aead;
#[allow(unused_variables)]
pub mod asymmetric;
#[allow(unused_variables)]
pub mod ecdh;
#[allow(unused_variables)]
pub mod hash;
#[allow(unused_variables)]
pub mod symmetric;

pub struct Backend(());

impl super::interface::Backend for Backend {
    fn backend() -> String {
        "Fuzzing".to_string()
    }

    fn random(buf: &mut [u8]) -> crate::Result<()> {
        buf.iter_mut().for_each(|b| *b = 4);
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
        AEADAlgorithm::OCB
    }

    pub(crate) fn is_supported_by_backend(&self) -> bool {
        true
    }

    #[cfg(test)]
    pub(crate) fn supports_symmetric_algo(&self, _: &SymmetricAlgorithm)
                                          -> bool {
        true
    }
}
