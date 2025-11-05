use crate::crypto::symmetric::Mode;

use crate::Result;
use crate::types::SymmetricAlgorithm;

struct NullCipher(usize);

impl Mode for NullCipher {
    fn block_size(&self) -> usize {
        self.0
    }

    fn encrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        dst.copy_from_slice(src);
        Ok(())
    }

    fn decrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        dst.copy_from_slice(src);
        Ok(())
    }
}

impl SymmetricAlgorithm {
    /// Returns whether this algorithm is supported by the crypto backend.
    ///
    /// All backends support all the AES variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sequoia_openpgp as openpgp;
    /// use openpgp::types::SymmetricAlgorithm;
    ///
    /// assert!(SymmetricAlgorithm::AES256.is_supported());
    /// assert!(SymmetricAlgorithm::TripleDES.is_supported());
    ///
    /// assert!(!SymmetricAlgorithm::IDEA.is_supported());
    /// assert!(!SymmetricAlgorithm::Unencrypted.is_supported());
    /// assert!(!SymmetricAlgorithm::Private(101).is_supported());
    /// ```
    pub(crate) fn is_supported_by_backend(&self) -> bool {
        true
    }

    /// Creates a Nettle context for encrypting in CFB mode.
    pub(crate) fn make_encrypt_cfb(self, key: &[u8], iv: Vec<u8>)
                                   -> Result<Box<dyn Mode>> {
        Ok(Box::new(NullCipher(self.block_size().unwrap_or(16))))
    }

    /// Creates a Nettle context for decrypting in CFB mode.
    pub(crate) fn make_decrypt_cfb(self, key: &[u8], iv: Vec<u8>)
                                   -> Result<Box<dyn Mode>> {
        Ok(Box::new(NullCipher(self.block_size().unwrap_or(16))))
    }

    /// Creates a Nettle context for encrypting in ECB mode.
    pub(crate) fn make_encrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        Ok(Box::new(NullCipher(self.block_size().unwrap_or(16))))
    }

    /// Creates a Nettle context for decrypting in ECB mode.
    pub(crate) fn make_decrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        Ok(Box::new(NullCipher(self.block_size().unwrap_or(16))))
    }
}
