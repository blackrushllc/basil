//! Implementation of AEAD using Windows CNG API.
use std::cmp::{self, Ordering};

use crate::{Error, Result};
use crate::crypto::aead::{Aead, CipherOp};
use crate::crypto::mem::secure_cmp;
use crate::seal;
use crate::types::{AEADAlgorithm, SymmetricAlgorithm};

use cipher::generic_array::{GenericArray, ArrayLength};
use cipher::generic_array::typenum::{U12, U16, U128, U192, U256};
use cipher::{BlockCipher, BlockEncrypt, BlockSizeUser, KeyInit, Unsigned};
use eax::online::{Eax, Encrypt, Decrypt};
use win_crypto_ng::symmetric::{BlockCipherKey, Aes};

/// Disables authentication checks.
///
/// This is DANGEROUS, and is only useful for debugging problems with
/// malformed AEAD-encrypted messages.
const DANGER_DISABLE_AUTHENTICATION: bool = false;

trait GenericArrayExt<T, N: ArrayLength<T>> {
    const LEN: usize;

    /// Like [`GenericArray::from_slice`], but fallible.
    fn try_from_slice(slice: &[T]) -> Result<&GenericArray<T, N>> {
        if slice.len() == Self::LEN {
            Ok(GenericArray::from_slice(slice))
        } else {
            Err(Error::InvalidArgument(
                format!("Invalid slice length, want {}, got {}",
                        Self::LEN, slice.len())).into())
        }
    }
}

impl<T, N: ArrayLength<T>> GenericArrayExt<T, N> for GenericArray<T, N> {
    const LEN: usize = N::USIZE;
}

impl AEADAlgorithm {
    pub(crate) fn context(
        &self,
        sym_algo: SymmetricAlgorithm,
        key: &[u8],
        aad: &[u8],
        nonce: &[u8],
        op: CipherOp,
    ) -> Result<Box<dyn Aead>> {

        match self {
            AEADAlgorithm::EAX => match sym_algo {
                SymmetricAlgorithm::AES128 => match op {
                    CipherOp::Encrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U128>, Encrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                    CipherOp::Decrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U128>, Decrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                },
                SymmetricAlgorithm::AES192 => match op {
                    CipherOp::Encrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U192>, Encrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                    CipherOp::Decrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U192>, Decrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                },
                SymmetricAlgorithm::AES256 => match op {
                    CipherOp::Encrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U256>, Encrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                    CipherOp::Decrypt => {
                        let mut ctx =
                            Eax::<BlockCipherKey<Aes, U256>, Decrypt>::with_key_and_nonce(
                                GenericArray::try_from_slice(key)?,
                                GenericArray::try_from_slice(nonce)?);
                        ctx.update_assoc(aad);
                        Ok(Box::new(ctx))
                    },
                },
                _ => Err(Error::UnsupportedSymmetricAlgorithm(sym_algo).into()),
            },

            AEADAlgorithm::OCB =>
                Err(Error::UnsupportedAEADAlgorithm(*self).into()),

            AEADAlgorithm::GCM => {
                use aes_gcm::{AesGcm, Nonce};
                match sym_algo {
                    SymmetricAlgorithm::AES128 => {
                        let nonce = Nonce::try_from_slice(nonce)?;
                        let cipher =
                            AesGcm::<BlockCipherKey<Aes, U128>, U12>::new_from_slice(key)?;
                        Ok(Box::new(Gcm { cipher, nonce: *nonce, aad: aad.to_vec() }))
                    },
                    SymmetricAlgorithm::AES192 => {
                        let nonce = Nonce::try_from_slice(nonce)?;
                        let cipher =
                            AesGcm::<BlockCipherKey<Aes, U192>, U12>::new_from_slice(key)?;
                        Ok(Box::new(Gcm { cipher, nonce: *nonce, aad: aad.to_vec() }))
                    },
                    SymmetricAlgorithm::AES256 => {
                        let nonce = Nonce::try_from_slice(nonce)?;
                        let cipher =
                            AesGcm::<BlockCipherKey<Aes, U256>, U12>::new_from_slice(key)?;
                        Ok(Box::new(Gcm { cipher, nonce: *nonce, aad: aad.to_vec() }))
                    },
                    | SymmetricAlgorithm::IDEA
                    | SymmetricAlgorithm::TripleDES
                    | SymmetricAlgorithm::CAST5
                    | SymmetricAlgorithm::Blowfish
                    | SymmetricAlgorithm::Twofish
                    | SymmetricAlgorithm::Camellia128
                    | SymmetricAlgorithm::Camellia192
                    | SymmetricAlgorithm::Camellia256
                    | SymmetricAlgorithm::Private(_)
                    | SymmetricAlgorithm::Unknown(_)
                    | SymmetricAlgorithm::Unencrypted =>
                        Err(Error::UnsupportedSymmetricAlgorithm(sym_algo).into()),
                }
            },

            AEADAlgorithm::Private(_) | AEADAlgorithm::Unknown(_) =>
                Err(Error::UnsupportedAEADAlgorithm(*self).into()),
        }
    }
}

type TagLen = U16;

macro_rules! impl_aead {
    ($($type: ty),*) => {
        $(
        impl Aead for Eax<$type, Encrypt, TagLen> {
            fn digest_size(&self) -> usize {
                TagLen::USIZE
            }
            fn encrypt_seal(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
                debug_assert_eq!(dst.len(), src.len() + self.digest_size());
                let len = core::cmp::min(dst.len(), src.len());
                dst[..len].copy_from_slice(&src[..len]);
                Eax::<$type, Encrypt>::encrypt(self, &mut dst[..len]);

                let tag = self.tag_clone();
                dst[src.len()..].copy_from_slice(&tag[..]);
                Ok(())
            }
            fn decrypt_verify(&mut self, _dst: &mut [u8], _src: &[u8]) -> Result<()> {
                panic!("AEAD decryption called in the encryption context")
            }
        }
        impl seal::Sealed for Eax<$type, Encrypt> {}
        )*
        $(
        impl Aead for Eax<$type, Decrypt> {
            fn digest_size(&self) -> usize {
                TagLen::USIZE
            }
            fn encrypt_seal(&mut self, _dst: &mut [u8], _src: &[u8]) -> Result<()> {
                panic!("AEAD encryption called in the decryption context")
            }
            fn decrypt_verify(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
                debug_assert_eq!(dst.len() + self.digest_size(), src.len());

                // Split src into ciphertext and digest.
                let l = self.digest_size();
                let digest = &src[src.len().saturating_sub(l)..];
                let src = &src[..src.len().saturating_sub(l)];

                let len = core::cmp::min(dst.len(), src.len());
                dst[..len].copy_from_slice(&src[..len]);
                self.decrypt_unauthenticated_hazmat(&mut dst[..len]);

                let chunk_digest = self.tag_clone();
                if secure_cmp(&chunk_digest[..], digest)
                    != Ordering::Equal && ! DANGER_DISABLE_AUTHENTICATION
                {
                    return Err(Error::ManipulatedMessage.into());
                }
                Ok(())
            }
        }
        impl seal::Sealed for Eax<$type, Decrypt> {}
        )*
    };
}

impl_aead!(BlockCipherKey<Aes, U128>, BlockCipherKey<Aes, U192>, BlockCipherKey<Aes, U256>);

struct Gcm<Cipher: BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt> {
    cipher: aes_gcm::AesGcm<Cipher, U12>,
    nonce: GenericArray<u8, U12>,
    aad: Vec<u8>,
}

impl<Cipher> Aead for Gcm<Cipher>
where
    Cipher: BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt,
{
    fn digest_size(&self) -> usize {
        TagLen::USIZE
    }

    fn encrypt_seal(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), src.len() + self.digest_size());
        use aes_gcm::AeadInPlace;

        let len = cmp::min(dst.len(), src.len());
        dst[..len].copy_from_slice(&src[..len]);
        let tag =
            self.cipher.encrypt_in_place_detached(&self.nonce, &self.aad,
                                                  &mut dst[..len])?;
        debug_assert_eq!(dst[len..].len(), tag.len());
        let tag_len = cmp::min(dst[len..].len(), tag.len());
        dst[len..len + tag_len].copy_from_slice(&tag[..tag_len]);
        Ok(())
    }

    fn decrypt_verify(&mut self, dst: &mut [u8], src: &[u8]) -> Result<()> {
        debug_assert_eq!(dst.len() + self.digest_size(), src.len());
        use aes_gcm::AeadInPlace;

        // Split src into ciphertext and digest.
        let len = src.len().saturating_sub(self.digest_size());
        let digest = &src[len..];
        let src = &src[..len];

        debug_assert_eq!(dst.len(), src.len());
        let len = core::cmp::min(dst.len(), src.len());
        dst[..len].copy_from_slice(&src[..len]);
        self.cipher.decrypt_in_place_detached(&self.nonce, &self.aad, dst,
                                              digest.try_into()?)?;
        Ok(())
    }
}

impl<'a, Cipher> seal::Sealed for Gcm<Cipher>
where
    Cipher: BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt,
{}
