use nettle_sys::{
    gcm_ctx, gcm_key, nettle_gcm_decrypt, nettle_gcm_digest,
    nettle_gcm_encrypt, nettle_gcm_set_iv, nettle_gcm_set_key,
    nettle_gcm_update,
};
use std::cmp::min;
use std::mem::zeroed;

use crate::{
    Result,
    aead::{Aead, AdStream, AeadInternal},
    cipher::{BlockSizeIs16, Cipher},
};

/// Galois/Counter mode (NIST SP800-38D).
pub struct Gcm<C: Cipher + BlockSizeIs16> {
    inner: GcmInner<C>,
    ad_stream: AdStream<C>,
}

struct GcmInner<C: Cipher + BlockSizeIs16> {
    cipher: C,
    key: gcm_key,
    context: gcm_ctx,
}

impl<C: Cipher + BlockSizeIs16> Gcm<C> {
    /// Size of a GCM digest in bytes.
    pub const DIGEST_SIZE: usize = ::nettle_sys::GCM_DIGEST_SIZE as usize;

    /// Creates a new GCM instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        let mut ctx = unsafe { zeroed() };
        let mut key_ctx = unsafe { zeroed() };
        let mut cipher = C::with_encrypt_key(key)?;
        let enc_func = C::raw_encrypt_function().ptr();
        let cipher_ctx = cipher.context();

        unsafe {
            nettle_gcm_set_key(
                &mut key_ctx as *mut _,
                cipher_ctx as *const _,
                enc_func,
            );
            nettle_gcm_set_iv(
                &mut ctx as *mut _,
                &key_ctx as *const _,
                nonce.len(),
                nonce.as_ptr(),
            );
        }

        Ok(Gcm {
            inner: GcmInner {
                cipher: cipher, key: key_ctx, context: ctx,
            },
            ad_stream: Default::default(),
        })
    }
}

impl<C: Cipher + BlockSizeIs16> Aead for Gcm<C> {
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

impl<C: Cipher + BlockSizeIs16> AeadInternal for GcmInner<C> {
    fn digest_size_internal(&self) -> usize {
        ::nettle_sys::GCM_DIGEST_SIZE as usize
    }

    fn update_internal(&mut self, ad: &[u8]) {
        unsafe {
            nettle_gcm_update(
                &mut self.context as *mut _,
                &self.key as *const _,
                ad.len(),
                ad.as_ptr(),
            );
        }
    }

    fn encrypt_internal(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_gcm_encrypt(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.cipher.context() as *const _,
                C::raw_encrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn decrypt_internal(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_gcm_decrypt(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.cipher.context() as *const _,
                C::raw_encrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn digest_internal(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_gcm_digest(
                &mut self.context as *mut _,
                &self.key as *const _,
                self.cipher.context() as *const _,
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

    #[test]
    fn round_trip_gcm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Twofish>::DIGEST_SIZE];

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
    fn modify_ad_gcm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Twofish>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; Twofish::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Twofish>::DIGEST_SIZE];

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
    fn round_trip_gcm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes128>::DIGEST_SIZE];

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
    fn modify_ad_gcm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes128>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; Aes128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes128>::DIGEST_SIZE];

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
    fn round_trip_gcm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes192>::DIGEST_SIZE];

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
    fn modify_ad_gcm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes192>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; Aes192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes192>::DIGEST_SIZE];

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
    fn round_trip_gcm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes256>::DIGEST_SIZE];

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
    fn modify_ad_gcm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes256>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; Aes256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes256>::DIGEST_SIZE];

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
    fn round_trip_gcm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia128>::DIGEST_SIZE];

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
    fn modify_ad_gcm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia128>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; Camellia128::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia128>::DIGEST_SIZE];

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
    fn round_trip_gcm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia192>::DIGEST_SIZE];

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
    fn modify_ad_gcm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia192>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; Camellia192::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia192>::DIGEST_SIZE];

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
    fn round_trip_gcm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia256>::DIGEST_SIZE];

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
    fn modify_ad_gcm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia256>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; Camellia256::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Camellia256>::DIGEST_SIZE];

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
    fn round_trip_gcm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Serpent>::DIGEST_SIZE];

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
    fn modify_ad_gcm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Serpent>::DIGEST_SIZE];

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
    fn modify_ciphertext_gcm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let mut dec = Gcm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; Serpent::BLOCK_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Serpent>::DIGEST_SIZE];

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
    fn gcm_aes128_streaming_ad() {
        use crate::cipher::Aes128;
        use crate::random::{TestRandom, Yarrow};
        let mut rng = Yarrow::default();
        let mut random_chunk_size = move || -> usize {
            rng.next_usize() % (2 * Aes128::BLOCK_SIZE) + 1
        };

        let key = vec![1; Aes128::KEY_SIZE];
        let nonce = vec![2; Aes128::BLOCK_SIZE];
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE + 1];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Gcm::<Aes128>::DIGEST_SIZE];

        let mut enc = Gcm::<Aes128>::with_key_and_nonce(&key, &nonce).unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            enc.update(d);
        }
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Gcm::<Aes128>::DIGEST_SIZE];

        let mut dec = Gcm::<Aes128>::with_key_and_nonce(&key, &nonce).unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            dec.update(d);
        }
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }
}
