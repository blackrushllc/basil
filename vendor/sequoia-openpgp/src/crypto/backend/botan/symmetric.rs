use crate::crypto::symmetric::Mode;

use crate::{Error, Result};
use crate::types::SymmetricAlgorithm;

struct Ecb(botan::BlockCipher, usize);

impl Mode for Ecb {
    fn block_size(&self) -> usize {
        self.1
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len());
        let l = dst.len().min(src.len());
        dst[..l].copy_from_slice(&src[..l]);
        self.0.encrypt_in_place(dst)?;
        Ok(())
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len());
        let l = dst.len().min(src.len());
        dst[..l].copy_from_slice(&src[..l]);
        self.0.decrypt_in_place(dst)?;
        Ok(())
    }
}

struct Cfb(botan::Cipher, usize);

impl Mode for Cfb {
    fn block_size(&self) -> usize {
        self.1
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len());
        self.0.finish_into(src, dst)?;
        Ok(())
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len());
        self.0.finish_into(src, dst)?;
        Ok(())
    }
}

impl SymmetricAlgorithm {
    /// Returns whether this algorithm is supported by the crypto backend.
    pub(crate) fn is_supported_by_backend(&self) -> bool {
        use self::SymmetricAlgorithm::*;
        match &self {
            TripleDES | IDEA | CAST5 | Blowfish |
            AES128 | AES192 | AES256 | Twofish |
            Camellia128 | Camellia192 | Camellia256
                => true,
            Unencrypted | Private(_) | Unknown(_)
                => false,
        }
    }

    /// Returns the name of the algorithm for use with Botan's
    /// constructor.
    pub(crate) fn botan_name(self) -> Result<&'static str> {
        match self {
            SymmetricAlgorithm::IDEA => Ok("IDEA"),
            SymmetricAlgorithm::TripleDES => Ok("3DES"),
            SymmetricAlgorithm::CAST5 => Ok("CAST-128"),
            SymmetricAlgorithm::Blowfish => Ok("Blowfish"),
            SymmetricAlgorithm::AES128 => Ok("AES-128"),
            SymmetricAlgorithm::AES192 => Ok("AES-192"),
            SymmetricAlgorithm::AES256 => Ok("AES-256"),
            SymmetricAlgorithm::Twofish => Ok("Twofish"),
            SymmetricAlgorithm::Camellia128 => Ok("Camellia-128"),
            SymmetricAlgorithm::Camellia192 => Ok("Camellia-192"),
            SymmetricAlgorithm::Camellia256 => Ok("Camellia-256"),
            SymmetricAlgorithm::Unencrypted |
            SymmetricAlgorithm::Unknown(_) |
            SymmetricAlgorithm::Private(_) =>
                Err(Error::UnsupportedSymmetricAlgorithm(self).into()),
        }
    }

    /// Creates a context for encrypting in CFB mode.
    pub(crate) fn make_encrypt_cfb(self, key: &[u8], iv: Vec<u8>) -> Result<Box<dyn Mode>> {
        let mut cipher = botan::Cipher::new(
            &format!("{}/CFB", self.botan_name()?),
            botan::CipherDirection::Encrypt)?;

        cipher.set_key(key)?;
        cipher.start(&iv)?;

        Ok(Box::new(Cfb(cipher, self.block_size()?)))
    }

    /// Creates a context for decrypting in CFB mode.
    pub(crate) fn make_decrypt_cfb(self, key: &[u8], iv: Vec<u8>) -> Result<Box<dyn Mode>> {
        let mut cipher = botan::Cipher::new(
            &format!("{}/CFB", self.botan_name()?),
            botan::CipherDirection::Decrypt)?;

        cipher.set_key(key)?;
        cipher.start(&iv)?;

        Ok(Box::new(Cfb(cipher, self.block_size()?)))
    }

    /// Creates a context for encrypting in ECB mode.
    pub(crate) fn make_encrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        let mut cipher = botan::BlockCipher::new(self.botan_name()?)?;

        cipher.set_key(key)?;

        Ok(Box::new(Ecb(cipher, self.block_size()?)))
    }

    /// Creates a context for decrypting in ECB mode.
    pub(crate) fn make_decrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        self.make_encrypt_ecb(key)
    }
}
