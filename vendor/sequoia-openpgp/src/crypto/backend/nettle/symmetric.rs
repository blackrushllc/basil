use nettle::cipher::{self, Cipher};
use nettle::mode::{self};

use crate::crypto::mem::Protected;
use crate::crypto::symmetric::Mode;

use crate::{Error, Result};
use crate::types::SymmetricAlgorithm;

struct ModeWrapper<M>
{
    mode: M,
    iv: Protected,
}

impl<M> ModeWrapper<M>
where
    M: nettle::mode::Mode + Send + Sync + 'static,
{
    fn new(mode: M, iv: Vec<u8>) -> Box<dyn Mode> {
        Box::new(ModeWrapper {
            mode,
            iv: iv.into(),
        })
    }
}

impl<M> Mode for ModeWrapper<M>
where
    M: nettle::mode::Mode + Send + Sync,
{
    fn block_size(&self) -> usize {
        self.mode.block_size()
    }

    fn encrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        self.mode.encrypt(&mut self.iv, dst, src)?;
        Ok(())
    }

    fn decrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        self.mode.decrypt(&mut self.iv, dst, src)?;
        Ok(())
    }
}

impl<C> Mode for C
where
    C: Cipher + Send + Sync,
{
    fn block_size(&self) -> usize {
        C::BLOCK_SIZE
    }

    fn encrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        self.encrypt(dst, src);
        Ok(())
    }

    fn decrypt(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()> {
        self.decrypt(dst, src);
        Ok(())
    }
}

impl SymmetricAlgorithm {
    /// Returns whether this algorithm is supported by the crypto backend.
    pub(crate) fn is_supported_by_backend(&self) -> bool {
        use self::SymmetricAlgorithm::*;
        match &self {
            TripleDES | CAST5 | Blowfish | AES128 | AES192 | AES256 | Twofish
                | Camellia128 | Camellia192 | Camellia256
                => true,
            Unencrypted | IDEA | Private(_) | Unknown(_)
                => false,
        }
    }

    /// Creates a Nettle context for encrypting in CFB mode.
    pub(crate) fn make_encrypt_cfb(self, key: &[u8], iv: Vec<u8>) -> Result<Box<dyn Mode>> {
        match self {
            SymmetricAlgorithm::TripleDES =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Des3>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::CAST5 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Cast128>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::Blowfish =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Blowfish>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES128 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes128>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES192 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes192>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES256 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes256>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::Twofish =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Twofish>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia128 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia128>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia192 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia192>::with_encrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia256 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia256>::with_encrypt_key(key)?, iv)),
            _ => Err(Error::UnsupportedSymmetricAlgorithm(self).into()),
        }
    }

    /// Creates a Nettle context for decrypting in CFB mode.
    pub(crate) fn make_decrypt_cfb(self, key: &[u8], iv: Vec<u8>) -> Result<Box<dyn Mode>> {
        match self {
            SymmetricAlgorithm::TripleDES =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Des3>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::CAST5 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Cast128>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::Blowfish =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Blowfish>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES128 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes128>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES192 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes192>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::AES256 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Aes256>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::Twofish =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Twofish>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia128 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia128>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia192 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia192>::with_decrypt_key(key)?, iv)),
            SymmetricAlgorithm::Camellia256 =>
                Ok(ModeWrapper::new(
                    mode::Cfb::<cipher::Camellia256>::with_decrypt_key(key)?, iv)),
            _ => Err(Error::UnsupportedSymmetricAlgorithm(self).into())
        }
    }

    /// Creates a Nettle context for encrypting in ECB mode.
    pub(crate) fn make_encrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        match self {
            SymmetricAlgorithm::TripleDES => Ok(Box::new(cipher::Des3::with_encrypt_key(key)?)),
            SymmetricAlgorithm::CAST5 => Ok(Box::new(cipher::Cast128::with_encrypt_key(key)?)),
            SymmetricAlgorithm::Blowfish => Ok(Box::new(cipher::Blowfish::with_encrypt_key(key)?)),
            SymmetricAlgorithm::AES128 => Ok(Box::new(cipher::Aes128::with_encrypt_key(key)?)),
            SymmetricAlgorithm::AES192 => Ok(Box::new(cipher::Aes192::with_encrypt_key(key)?)),
            SymmetricAlgorithm::AES256 => Ok(Box::new(cipher::Aes256::with_encrypt_key(key)?)),
            SymmetricAlgorithm::Twofish => Ok(Box::new(cipher::Twofish::with_encrypt_key(key)?)),
            SymmetricAlgorithm::Camellia128 => Ok(Box::new(cipher::Camellia128::with_encrypt_key(key)?)),
            SymmetricAlgorithm::Camellia192 => Ok(Box::new(cipher::Camellia192::with_encrypt_key(key)?)),
            SymmetricAlgorithm::Camellia256 => Ok(Box::new(cipher::Camellia256::with_encrypt_key(key)?)),
            _ => Err(Error::UnsupportedSymmetricAlgorithm(self).into())
        }
    }

    /// Creates a Nettle context for decrypting in ECB mode.
    pub(crate) fn make_decrypt_ecb(self, key: &[u8]) -> Result<Box<dyn Mode>> {
        match self {
            SymmetricAlgorithm::TripleDES => Ok(Box::new(cipher::Des3::with_decrypt_key(key)?)),
            SymmetricAlgorithm::CAST5 => Ok(Box::new(cipher::Cast128::with_decrypt_key(key)?)),
            SymmetricAlgorithm::Blowfish => Ok(Box::new(cipher::Blowfish::with_decrypt_key(key)?)),
            SymmetricAlgorithm::AES128 => Ok(Box::new(cipher::Aes128::with_decrypt_key(key)?)),
            SymmetricAlgorithm::AES192 => Ok(Box::new(cipher::Aes192::with_decrypt_key(key)?)),
            SymmetricAlgorithm::AES256 => Ok(Box::new(cipher::Aes256::with_decrypt_key(key)?)),
            SymmetricAlgorithm::Twofish => Ok(Box::new(cipher::Twofish::with_decrypt_key(key)?)),
            SymmetricAlgorithm::Camellia128 => Ok(Box::new(cipher::Camellia128::with_decrypt_key(key)?)),
            SymmetricAlgorithm::Camellia192 => Ok(Box::new(cipher::Camellia192::with_decrypt_key(key)?)),
            SymmetricAlgorithm::Camellia256 => Ok(Box::new(cipher::Camellia256::with_decrypt_key(key)?)),
            _ => Err(Error::UnsupportedSymmetricAlgorithm(self).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anchors the constants used in Sequoia with the ones from
    /// Nettle.
    #[test]
    fn key_size() -> Result<()> {
        assert_eq!(SymmetricAlgorithm::TripleDES.key_size()?,
                   cipher::Des3::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::CAST5.key_size()?,
                   cipher::Cast128::KEY_SIZE);
        // RFC4880, Section 9.2: Blowfish (128 bit key, 16 rounds)
        assert_eq!(SymmetricAlgorithm::Blowfish.key_size()?, 16);
        assert_eq!(SymmetricAlgorithm::AES128.key_size()?,
                   cipher::Aes128::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::AES192.key_size()?,
                   cipher::Aes192::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::AES256.key_size()?,
                   cipher::Aes256::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::Twofish.key_size()?,
                   cipher::Twofish::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia128.key_size()?,
                   cipher::Camellia128::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia192.key_size()?,
                   cipher::Camellia192::KEY_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia256.key_size()?,
                   cipher::Camellia256::KEY_SIZE);
        Ok(())
    }

    /// Anchors the constants used in Sequoia with the ones from
    /// Nettle.
    #[test]
    fn block_size() -> Result<()> {
        assert_eq!(SymmetricAlgorithm::TripleDES.block_size()?,
                   cipher::Des3::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::CAST5.block_size()?,
                   cipher::Cast128::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::Blowfish.block_size()?,
                   cipher::Blowfish::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::AES128.block_size()?,
                   cipher::Aes128::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::AES192.block_size()?,
                   cipher::Aes192::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::AES256.block_size()?,
                   cipher::Aes256::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::Twofish.block_size()?,
                   cipher::Twofish::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia128.block_size()?,
                   cipher::Camellia128::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia192.block_size()?,
                   cipher::Camellia192::BLOCK_SIZE);
        assert_eq!(SymmetricAlgorithm::Camellia256.block_size()?,
                   cipher::Camellia256::BLOCK_SIZE);
        Ok(())
    }
}
