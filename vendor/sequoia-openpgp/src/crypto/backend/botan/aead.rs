//! Implementation of AEAD using the Botan cryptographic library.

use crate::{Error, Result};

use crate::crypto::aead::{Aead, CipherOp};
use crate::seal;
use crate::types::{AEADAlgorithm, SymmetricAlgorithm};

struct Cipher(botan::Cipher, usize);

impl seal::Sealed for Cipher {}
impl Aead for Cipher {
    fn encrypt_seal(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len() + self.digest_size());
        self.0.finish_into(src, dst)?;
        Ok(())
    }

    fn decrypt_verify(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len() + self.digest_size(), src.len());
        self.0.finish_into(src, dst)?;
        Ok(())
    }
    fn digest_size(&self) -> usize {
        self.1
    }
}

impl AEADAlgorithm {
    /// Returns the name of the algorithm for use with Botan's
    /// constructor.
    fn botan_name(self) -> Result<&'static str> {
        match self {
            AEADAlgorithm::EAX => Ok("EAX"),
            AEADAlgorithm::OCB => Ok("OCB"),
            AEADAlgorithm::GCM => Ok("GCM"),
            _ => Err(Error::UnsupportedAEADAlgorithm(self).into()),
        }
    }

    pub(crate) fn context(&self,
                          sym_algo: SymmetricAlgorithm,
                          key: &[u8],
                          aad: &[u8],
                          nonce: &[u8],
                          op: CipherOp)
                          -> Result<Box<dyn Aead>>
    {
        let mut cipher = botan::Cipher::new(
            &format!("{}/{}", sym_algo.botan_name()?, self.botan_name()?),
            match op {
                CipherOp::Encrypt => botan::CipherDirection::Encrypt,
                CipherOp::Decrypt => botan::CipherDirection::Decrypt,
            })
            // XXX it could be the cipher that is not supported.
            .map_err(|_| Error::UnsupportedAEADAlgorithm(*self))?;

        cipher.set_key(key)?;
        cipher.set_associated_data(aad)?;
        cipher.start(nonce)?;

        Ok(Box::new(Cipher(cipher, self.digest_size()?)))
    }
}
