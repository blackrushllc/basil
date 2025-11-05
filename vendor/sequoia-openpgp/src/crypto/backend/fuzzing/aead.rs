//! Implementation of AEAD using Nettle cryptographic library.

use crate::Result;

use crate::crypto::aead::{Aead, CipherOp};
use crate::seal;
use crate::types::{AEADAlgorithm, SymmetricAlgorithm};

struct NullAEADMode {}

const DIGEST_SIZE: usize = 16;

impl seal::Sealed for NullAEADMode {}
impl Aead for NullAEADMode {
    fn encrypt_seal(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        let l = dst.len() - DIGEST_SIZE;
        dst[..l].copy_from_slice(src);
        dst[l..].iter_mut().for_each(|p| *p = 0x04);
        Ok(())
    }
    fn decrypt_verify(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        dst.copy_from_slice(&src[..src.len() - DIGEST_SIZE]);
        Ok(())
    }
    fn digest_size(&self) -> usize {
        DIGEST_SIZE
    }
}

impl AEADAlgorithm {
    pub(crate) fn context(
        &self,
        sym_algo: SymmetricAlgorithm,
        key: &[u8],
        aad: &[u8],
        nonce: &[u8],
        _op: CipherOp,
    ) -> Result<Box<dyn Aead>> {
        Ok(Box::new(NullAEADMode {}))
    }
}
