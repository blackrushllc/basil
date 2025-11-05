use nettle_sys::{
    ocb_ctx, ocb_key, nettle_ocb_decrypt, nettle_ocb_digest,
    nettle_ocb_encrypt, nettle_ocb_set_key, nettle_ocb_set_nonce,
    nettle_ocb_update,
};
use std::cmp::min;
use std::mem::zeroed;

/// Public re-export to specialize [`Ocb`].
pub use typenum;
use typenum::{Unsigned, type_operators::IsLessOrEqual, consts::{U16, True}};

use crate::{
    Error,
    Result,
    aead::{Aead, AdStream, AeadInternal},
    cipher::{Cipher, BlockSizeIs16},
};

/// Whether or not this algorithm is supported by the version of
/// Nettle we are linked against at build time.
///
/// If this is false, then [`Ocb::with_key_and_nonce`] will fail
/// on all inputs.
pub const OCB_IS_SUPPORTED: bool = nettle_sys::config::HAVE_OCB;

/// Phillip Rogaway's OCB mode of operation.
///
/// The type parameter `T` denotes the tag length, which must not
/// exceed [`Ocb::MAX_DIGEST_SIZE`].
pub struct Ocb<C, T>
where
    C: Cipher + BlockSizeIs16,
    T: Unsigned + IsLessOrEqual<U16, Output = True>,
{
    inner: OcbInner<C, T>,
    ad_stream: AdStream<C>,
}

struct OcbInner<C, T>
where
    C: Cipher + BlockSizeIs16,
    T: Unsigned + IsLessOrEqual<U16, Output = True>,
{
    encrypt: C,
    decrypt: C,
    key: ocb_key,
    context: ocb_ctx,
    _marker: std::marker::PhantomData<T>,
}

impl<C, T> Ocb<C, T>
where
    C: Cipher + BlockSizeIs16,
    T: Unsigned + IsLessOrEqual<U16, Output = True>,
{
    /// Maximum size of a OCB digest in bytes.
    pub const MAX_DIGEST_SIZE: usize = ::nettle_sys::OCB_DIGEST_SIZE as usize;

    /// Creates a new OCB instance with secret `key` and public `nonce`.
    ///
    /// Returns an error if OCB mode is not supported by the version
    /// of Nettle we are linked against at build time.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8])
                              -> Result<Self> {
        // C is BlockSizeIs16, so this is an invariant.
        assert_eq!(C::BLOCK_SIZE, 16);

        // T is sufficiently constrained.
        assert!(T::to_usize() <= Self::MAX_DIGEST_SIZE);

        if ! OCB_IS_SUPPORTED {
            // XXX: Not quite the right error.
            return Err(Error::EncryptionFailed);
        }

        let mut encrypt = C::with_encrypt_key(key)?;
        let decrypt = C::with_decrypt_key(key)?;
        let mut context = unsafe { zeroed() };
        let mut key = unsafe { zeroed() };
        let enc_func = C::raw_encrypt_function().ptr();

        unsafe {
            nettle_ocb_set_key(
                &mut key as *mut _,
                encrypt.context() as *const _,
                enc_func,
            );
            nettle_ocb_set_nonce(
                &mut context as *mut _,
                encrypt.context() as *const _,
                enc_func,
                T::to_usize(),
                nonce.len(),
                nonce.as_ptr(),
            );
        }

        Ok(Ocb {
            inner: OcbInner {
                encrypt, decrypt, key, context, _marker: Default::default(),
            },
            ad_stream: Default::default(),
        })
    }
}

impl<C, T> Aead for Ocb<C, T>
where
    C: Cipher + BlockSizeIs16,
    T: Unsigned + IsLessOrEqual<U16, Output = True>,
{
    fn digest_size(&self) -> usize {
        self.inner.digest_size_internal()
    }

    fn update(&mut self, ad: &[u8]) {
        self.ad_stream.update(ad, &mut self.inner);
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.ad_stream.finalize(&mut self.inner);
        self.inner.encrypt_internal(dst, src);
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.ad_stream.finalize(&mut self.inner);
        self.inner.decrypt_internal(dst, src);
    }

    fn digest(&mut self, digest: &mut [u8]) {
        self.inner.digest_internal(digest);
    }
}

impl<C, T> AeadInternal for OcbInner<C, T>
where
    C: Cipher + BlockSizeIs16,
    T: Unsigned + IsLessOrEqual<U16, Output = True>,
{
    fn digest_size_internal(&self) -> usize {
        ::nettle_sys::OCB_DIGEST_SIZE as usize
    }

    fn update_internal(&mut self, ad: &[u8]) {
        unsafe {
            nettle_ocb_update(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.encrypt.context() as *const _,
                C::raw_encrypt_function().ptr(),
                ad.len(),
                ad.as_ptr(),
            );
        }
    }

    fn encrypt_internal(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_ocb_encrypt(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.encrypt.context() as *const _,
                C::raw_encrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn decrypt_internal(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_ocb_decrypt(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.encrypt.context() as *const _,
                C::raw_encrypt_function().ptr(),
                self.decrypt.context() as *const _,
                C::raw_decrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn digest_internal(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_ocb_digest(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.encrypt.context() as *const _,
                C::raw_encrypt_function().ptr(),
                digest.len(),
                digest.as_mut_ptr(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_NONCE_SIZE: usize = 15;
    use typenum::Unsigned;
    use typenum::consts::U16 as Taglen;

    #[test]
    fn round_trip_ocb_twofish() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Twofish;
        let mut enc = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_twofish() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Twofish;
        let mut enc = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_twofish() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Twofish;
        let mut enc = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Twofish, Taglen>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_aes128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes128;
        let mut enc = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_aes128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes128;
        let mut enc = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_aes128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes128;
        let mut enc = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes128, Taglen>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_aes192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes192;
        let mut enc = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_aes192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes192;
        let mut enc = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_aes192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes192;
        let mut enc = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes192, Taglen>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_aes256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes256;
        let mut enc = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_aes256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes256;
        let mut enc = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_aes256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes256;
        let mut enc = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Aes256, Taglen>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_camellia128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia128;
        let mut enc = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_camellia128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia128;
        let mut enc = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_camellia128() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia128;
        let mut enc = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia128, Taglen>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_camellia192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia192;
        let mut enc = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_camellia192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia192;
        let mut enc = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_camellia192() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia192;
        let mut enc = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia192, Taglen>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_camellia256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia256;
        let mut enc = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_camellia256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia256;
        let mut enc = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_camellia256() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Camellia256;
        let mut enc = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Camellia256, Taglen>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn round_trip_ocb_serpent() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Serpent;
        let mut enc = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn modify_ad_ocb_serpent() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Serpent;
        let mut enc = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        input_ad[1] = 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn modify_ciphertext_ocb_serpent() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Serpent;
        let mut enc = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let mut dec = Ocb::<Serpent, Taglen>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; MAX_NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Taglen::to_usize()];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Taglen::to_usize()];

        enc.update(&input_ad);
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        ciphertext[1] ^= 42;

        dec.update(&input_ad);
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_ne!(input_plaintext, output_plaintext);
        assert_ne!(digest, output_digest);
    }

    #[test]
    fn ocb_aes128_streaming_ad() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        use crate::cipher::Aes128;
        use crate::random::{TestRandom, Yarrow};
        let mut rng = Yarrow::default();
        let mut random_chunk_size = move || -> usize {
            rng.next_usize() % (2 * Aes128::BLOCK_SIZE) + 1
        };

        let key = vec![1; Aes128::KEY_SIZE];
        let nonce = vec![2; MAX_NONCE_SIZE];
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE + 1];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ocb::<Aes128, Taglen>::MAX_DIGEST_SIZE];

        let mut enc = Ocb::<Aes128, Taglen>::with_key_and_nonce(&key, &nonce)
            .unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            enc.update(d);
        }
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ocb::<Aes128, Taglen>::MAX_DIGEST_SIZE];

        let mut dec = Ocb::<Aes128, Taglen>::with_key_and_nonce(&key, &nonce)
            .unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            dec.update(d);
        }
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    fn test_encrypt_decrypt(key: &[u8], nonce: &[u8], aad: &[u8],
                            plaintext: &[u8], ciphertext: &[u8])
    {
        use crate::cipher::Aes128;

        // Split ciphertext in ciphertext and tag.
        let digest = &ciphertext[ciphertext.len() - Taglen::to_usize()..];
        let ciphertext = &ciphertext[..ciphertext.len() - Taglen::to_usize()];

        // First, encrypt and compare ciphertext.
        let mut enc = Ocb::<Aes128, Taglen>::with_key_and_nonce(key, nonce)
            .unwrap();
        let mut ciphertext_ = vec![0u8; plaintext.len()];
        let mut digest_ = vec![0u8; Taglen::to_usize()];
        enc.update(&aad);
        enc.encrypt(&mut ciphertext_, &plaintext);
        enc.digest(&mut digest_);
        assert_eq!(ciphertext, ciphertext_);
        assert_eq!(digest, digest_);

        // Second, decrypt and compare plaintext.
        let mut dec = Ocb::<Aes128, Taglen>::with_key_and_nonce(key, nonce)
            .unwrap();
        let mut plaintext_ = vec![0u8; plaintext.len()];
        let mut digest_ = vec![0u8; Taglen::to_usize()];
        dec.update(&aad);
        dec.decrypt(&mut plaintext_, ciphertext);
        dec.digest(&mut digest_);
        assert_eq!(plaintext, plaintext_);
        assert_eq!(digest, digest_);
    }

    #[test]
    fn rfc7253_test_vectors() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        let key =
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x00",
            b"",
            b"",
            b"\x78\x54\x07\xBF\xFF\xC8\xAD\x9E\xDC\xC5\x52\x0A\xC9\x11\x1E\xE6",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x01",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x68\x20\xB3\x65\x7B\x6F\x61\x5A\x57\x25\xBD\xA0\xD3\xB4\xEB\x3A\x25\x7C\x9A\xF1\xF8\xF0\x30\x09",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x02",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"",
            b"\x81\x01\x7F\x82\x03\xF0\x81\x27\x71\x52\xFA\xDE\x69\x4A\x0A\x00",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x03",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x45\xDD\x69\xF8\xF5\xAA\xE7\x24\x14\x05\x4C\xD1\xF3\x5D\x82\x76\x0B\x2C\xD0\x0D\x2F\x99\xBF\xA9",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x04",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\x57\x1D\x53\x5B\x60\xB2\x77\x18\x8B\xE5\x14\x71\x70\xA9\xA2\x2C\x3A\xD7\xA4\xFF\x38\x35\xB8\xC5\x70\x1C\x1C\xCE\xC8\xFC\x33\x58",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x05",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"",
            b"\x8C\xF7\x61\xB6\x90\x2E\xF7\x64\x46\x2A\xD8\x64\x98\xCA\x6B\x97",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x06",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\x5C\xE8\x8E\xC2\xE0\x69\x27\x06\xA9\x15\xC0\x0A\xEB\x8B\x23\x96\xF4\x0E\x1C\x74\x3F\x52\x43\x6B\xDF\x06\xD8\xFA\x1E\xCA\x34\x3D",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x07",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x1C\xA2\x20\x73\x08\xC8\x7C\x01\x07\x56\x10\x4D\x88\x40\xCE\x19\x52\xF0\x96\x73\xA4\x48\xA1\x22\xC9\x2C\x62\x24\x10\x51\xF5\x73\x56\xD7\xF3\xC9\x0B\xB0\xE0\x7F",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x08",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"",
            b"\x6D\xC2\x25\xA0\x71\xFC\x1B\x9F\x7C\x69\xF9\x3B\x0F\x1E\x10\xDE",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x09",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x22\x1B\xD0\xDE\x7F\xA6\xFE\x99\x3E\xCC\xD7\x69\x46\x0A\x0A\xF2\xD6\xCD\xED\x0C\x39\x5B\x1C\x3C\xE7\x25\xF3\x24\x94\xB9\xF9\x14\xD8\x5C\x0B\x1E\xB3\x83\x57\xFF",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0A",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\xBD\x6F\x6C\x49\x62\x01\xC6\x92\x96\xC1\x1E\xFD\x13\x8A\x46\x7A\xBD\x3C\x70\x79\x24\xB9\x64\xDE\xAF\xFC\x40\x31\x9A\xF5\xA4\x85\x40\xFB\xBA\x18\x6C\x55\x53\xC6\x8A\xD9\xF5\x92\xA7\x9A\x42\x40",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0B",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"",
            b"\xFE\x80\x69\x0B\xEE\x8A\x48\x5D\x11\xF3\x29\x65\xBC\x9D\x2A\x32",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0C",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\x29\x42\xBF\xC7\x73\xBD\xA2\x3C\xAB\xC6\xAC\xFD\x9B\xFD\x58\x35\xBD\x30\x0F\x09\x73\x79\x2E\xF4\x60\x40\xC5\x3F\x14\x32\xBC\xDF\xB5\xE1\xDD\xE3\xBC\x18\xA5\xF8\x40\xB5\x2E\x65\x34\x44\xD5\xDF",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0D",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\xD5\xCA\x91\x74\x84\x10\xC1\x75\x1F\xF8\xA2\xF6\x18\x25\x5B\x68\xA0\xA1\x2E\x09\x3F\xF4\x54\x60\x6E\x59\xF9\xC1\xD0\xDD\xC5\x4B\x65\xE8\x62\x8E\x56\x8B\xAD\x7A\xED\x07\xBA\x06\xA4\xA6\x94\x83\xA7\x03\x54\x90\xC5\x76\x9E\x60",
        );


        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0E",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"",
            b"\xC5\xCD\x9D\x18\x50\xC1\x41\xE3\x58\x64\x99\x94\xEE\x70\x1B\x68",
        );

        test_encrypt_decrypt(
            key,
            b"\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0F",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\x44\x12\x92\x34\x93\xC5\x7D\x5D\xE0\xD7\x00\xF7\x53\xCC\xE0\xD1\xD2\xD9\x50\x60\x12\x2E\x9F\x15\xA5\xDD\xBF\xC5\x78\x7E\x50\xB5\xCC\x55\xEE\x50\x7B\xCB\x08\x4E\x47\x9A\xD3\x63\xAC\x36\x6B\x95\xA9\x8C\xA5\xF3\x00\x0B\x14\x79",
        );
    }

    #[test]
    fn dkgs_test_vectors() {
        if ! OCB_IS_SUPPORTED {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        let key =
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x00",
            b"",
            b"",
            b"\x75\x2A\xCD\x21\x32\xC4\x1E\x02\x0E\x41\xFB\x22\x3E\xFD\x77\xB6",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x01",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x20\x1F\xE4\xD8\x9E\xA7\xBD\x1E\xB5\xB1\x57\x7D\xB1\x62\x83\xB8\xAE\xD1\x71\x5A\xD6\xBE\x51\x49",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x02",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"",
            b"\x71\x09\x60\xB9\xEE\x00\xB8\xF4\x4D\x2E\x81\x20\xAA\xBA\x63\xAE",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x03",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"\x08\x4E\x86\x95\x70\x19\x4B\xD2\x50\x32\xFE\x9E\x53\x28\xE4\x5D\x50\x7E\x74\xF3\x36\x6E\x20\xD2",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x04",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\x96\x76\xEE\x37\xFD\x64\x5C\x07\xC0\xD4\xF7\x0A\xAB\xF6\x86\x68\x8E\x39\xB2\xFB\x3F\xC4\xFF\x30\xDC\xD1\x82\x7B\x36\xA2\x98\xD3",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x05",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"",
            b"\x9D\x51\x0F\x56\xED\xF7\x2F\xFA\x34\x96\x9B\xCE\xF9\x1E\x6D\xE9",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x06",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            b"\xD5\xE1\x5A\xA1\xD2\x32\xAB\x57\xF2\x34\x36\x6D\xFF\xB2\x55\x74\xA3\x63\x6A\x5F\x3E\x34\x33\xEA\x45\x90\xCB\xF4\xF9\xAC\x1F\x4D",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x07",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x1C\x4B\x67\x77\xB7\xF1\x37\xC3\x09\x71\xA9\x3D\xE3\xC5\x6C\xC7\x35\x68\x6A\x6F\x77\x03\x14\x2F\xAB\x8A\xCC\x98\x7C\x14\x06\xDF\xF9\x62\x73\xC5\x37\x6E\x62\x10",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x08",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"",
            b"\x96\xE6\x70\xC0\x23\x8F\xB9\x69\xB7\xAC\xE4\xAB\xAF\x74\x38\xC7",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x09",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17",
            b"\x12\x90\xA6\x86\xD8\x25\xF7\x12\xE5\x94\xBE\x40\x39\xC0\x4D\x3E\x44\xF7\xD1\x34\x2B\x84\xFF\xCA\xD6\x8B\xBD\xFA\x04\xB5\x80\xEA\x9A\x01\xE2\xF4\x56\x53\x99\xC3",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0A",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\xFB\xDF\xC1\x1F\x74\x92\x17\xBB\x7F\xAE\x5D\x40\x36\xB8\xF2\x28\x03\x71\x2E\xFF\x9E\xF9\x43\x42\xFE\x1B\x68\x49\x68\xD0\xE3\xE3\x81\xA2\x77\xDA\xAB\x83\x57\x94\x06\xA0\x1E\x26\x75\xA0\x82\xC9",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0B",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"",
            b"\x90\xCD\xA8\xA0\x51\x61\xD2\x87\x33\x61\x37\x4B\x76\xF9\x54\x30",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0C",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            b"\xD1\x32\x0A\xF4\xB6\xFF\x8A\xFE\xEC\xEE\x79\x21\x39\x5D\x4E\x86\x92\x71\x77\x53\xEE\x15\xF5\x03\x8E\xB6\x74\xDA\x43\xD6\xEA\x8D\xBE\x78\x31\xE7\x23\xBE\x47\x1F\x62\xD9\xE7\xF4\x9A\x7D\x3B\x32",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0D",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\x5C\x79\xF1\xC4\xB9\xA2\x04\xED\x33\x23\x61\x6D\x57\x6F\xC5\x00\xE4\xA7\x19\x39\xF0\x3A\x3C\x3D\xE2\xC0\x97\xAF\x2C\x6C\x81\xDC\x3F\x03\x09\xE7\x60\x82\xB1\xF5\x0F\xF8\x52\x29\x59\xFF\xE4\x1F\x37\xEF\x50\x7E\x90\x76\xD3\x2C",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0E",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"",
            b"\x3B\xF1\x58\xB7\xDE\x76\xC5\x15\x1E\xF6\x08\x6A\x82\x5D\x0C\xC4",
        );

        test_encrypt_decrypt(
            key,
            b"\xEE\xDD\xCC\xBB\xAA\x99\x88\x77\x66\x55\x44\x33\x22\x11\x0F",
            b"",
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27",
            b"\x34\xDA\x59\xD2\xEB\x08\xF4\x78\x22\xD4\x8C\x85\xB6\xA1\xD2\x36\x94\xE1\xD3\xDE\x68\x0D\x61\x6D\x7B\x1B\x59\x47\x2C\x13\xE3\x69\xC6\x8D\xCA\x69\x9D\xA1\x68\x6A\x33\x9D\x54\x52\x80\x36\x32\x81\x0B\x08\x40\xE6\x80\x4A\xB0\x20",
        );
    }
}
