use nettle_sys::{
    ccm_ctx, nettle_ccm_decrypt, nettle_ccm_digest, nettle_ccm_encrypt,
    nettle_ccm_set_nonce, nettle_ccm_update,
};
use std::cmp::min;
use std::mem::zeroed;

use crate::{aead::Aead, cipher::{BlockSizeIs16, Cipher}, Result};

/// Counter with CBC-MAC mode (NIST SP800-38C).
///
/// CCM is a generic AEAD mode for block cipher with 128 bit block size.
pub struct Ccm<C: Cipher + BlockSizeIs16> {
    cipher: C,
    context: ccm_ctx,
}

impl<C: Cipher + BlockSizeIs16> Ccm<C> {
    /// Recommended size of the CCM digest in bytes.
    pub const DIGEST_SIZE: usize = ::nettle_sys::CCM_DIGEST_SIZE as usize;

    /// Creates a new instance with secret `key` and public `nonce`.
    ///
    /// The instance expect additional data of `ad_len` bytes, a
    /// overall message of `msg_len` and will produce a digest of
    /// `digest_len` bytes.
    pub fn with_key_and_nonce(
        key: &[u8],
        nonce: &[u8],
        ad_len: usize,
        msg_len: usize,
        digest_len: usize,
    ) -> Result<Self> {
        let mut ctx = unsafe { zeroed() };
        let mut cipher = C::with_encrypt_key(key)?;
        let enc_func = C::raw_encrypt_function().ptr();
        let cipher_ctx = cipher.context();

        unsafe {
            nettle_ccm_set_nonce(
                &mut ctx as *mut _,
                cipher_ctx as *const _,
                enc_func,
                nonce.len(),
                nonce.as_ptr(),
                ad_len,
                msg_len,
                digest_len,
            );
        }

        Ok(Ccm { cipher: cipher, context: ctx })
    }
}

impl<C: Cipher + BlockSizeIs16> Aead for Ccm<C> {
    fn digest_size(&self) -> usize {
        ::nettle_sys::CCM_DIGEST_SIZE as usize
    }

    fn update(&mut self, ad: &[u8]) {
        unsafe {
            nettle_ccm_update(
                &mut self.context as *mut _,
                self.cipher.context() as *const _,
                C::raw_encrypt_function().ptr(),
                ad.len(),
                ad.as_ptr(),
            );
        }
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_ccm_encrypt(
                &mut self.context as *mut _,
                self.cipher.context() as *const _,
                C::raw_encrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_ccm_decrypt(
                &mut self.context as *mut _,
                self.cipher.context() as *const _,
                C::raw_encrypt_function().ptr(),
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_ccm_digest(
                &mut self.context as *mut _,
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
    fn round_trip_ccm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Twofish>::DIGEST_SIZE];

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
    fn modify_ad_ccm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Twofish>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_twofish() {
        use crate::cipher::Twofish;
        let mut enc = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Twofish>::with_key_and_nonce(
            &vec![1; Twofish::KEY_SIZE],
            &vec![2; 14],
            Twofish::BLOCK_SIZE * 5,
            Twofish::BLOCK_SIZE * 10,
            Ccm::<Twofish>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Twofish::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Twofish::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Twofish::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Twofish>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Twofish::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Twofish>::DIGEST_SIZE];

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
    fn round_trip_ccm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes128>::DIGEST_SIZE];

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
    fn modify_ad_ccm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes128>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_aes128() {
        use crate::cipher::Aes128;
        let mut enc = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            Aes128::BLOCK_SIZE * 5,
            Aes128::BLOCK_SIZE * 10,
            Ccm::<Aes128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes128>::DIGEST_SIZE];

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
    fn round_trip_ccm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes192>::DIGEST_SIZE];

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
    fn modify_ad_ccm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes192>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_aes192() {
        use crate::cipher::Aes192;
        let mut enc = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes192>::with_key_and_nonce(
            &vec![1; Aes192::KEY_SIZE],
            &vec![2; 14],
            Aes192::BLOCK_SIZE * 5,
            Aes192::BLOCK_SIZE * 10,
            Ccm::<Aes192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes192>::DIGEST_SIZE];

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
    fn round_trip_ccm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes256>::DIGEST_SIZE];

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
    fn modify_ad_ccm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes256>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_aes256() {
        use crate::cipher::Aes256;
        let mut enc = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Aes256>::with_key_and_nonce(
            &vec![1; Aes256::KEY_SIZE],
            &vec![2; 14],
            Aes256::BLOCK_SIZE * 5,
            Aes256::BLOCK_SIZE * 10,
            Ccm::<Aes256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Aes256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Aes256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes256>::DIGEST_SIZE];

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
    fn round_trip_ccm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia128>::DIGEST_SIZE];

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
    fn modify_ad_ccm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia128>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_camellia128() {
        use crate::cipher::Camellia128;
        let mut enc = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia128>::with_key_and_nonce(
            &vec![1; Camellia128::KEY_SIZE],
            &vec![2; 14],
            Camellia128::BLOCK_SIZE * 5,
            Camellia128::BLOCK_SIZE * 10,
            Ccm::<Camellia128>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia128::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia128>::DIGEST_SIZE];

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
    fn round_trip_ccm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia192>::DIGEST_SIZE];

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
    fn modify_ad_ccm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia192>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_camellia192() {
        use crate::cipher::Camellia192;
        let mut enc = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia192>::with_key_and_nonce(
            &vec![1; Camellia192::KEY_SIZE],
            &vec![2; 14],
            Camellia192::BLOCK_SIZE * 5,
            Camellia192::BLOCK_SIZE * 10,
            Ccm::<Camellia192>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia192::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia192::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia192::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia192>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia192::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia192>::DIGEST_SIZE];

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
    fn round_trip_ccm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia256>::DIGEST_SIZE];

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
    fn modify_ad_ccm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia256>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_camellia256() {
        use crate::cipher::Camellia256;
        let mut enc = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Camellia256>::with_key_and_nonce(
            &vec![1; Camellia256::KEY_SIZE],
            &vec![2; 14],
            Camellia256::BLOCK_SIZE * 5,
            Camellia256::BLOCK_SIZE * 10,
            Ccm::<Camellia256>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Camellia256::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Camellia256::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Camellia256::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Camellia256>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Camellia256::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Camellia256>::DIGEST_SIZE];

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
    fn round_trip_ccm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Serpent>::DIGEST_SIZE];

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
    fn modify_ad_ccm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let mut input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Serpent>::DIGEST_SIZE];

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
    fn modify_ciphertext_ccm_serpent() {
        use crate::cipher::Serpent;
        let mut enc = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let mut dec = Ccm::<Serpent>::with_key_and_nonce(
            &vec![1; Serpent::KEY_SIZE],
            &vec![2; 14],
            Serpent::BLOCK_SIZE * 5,
            Serpent::BLOCK_SIZE * 10,
            Ccm::<Serpent>::DIGEST_SIZE,
        )
        .unwrap();
        let input_plaintext = vec![1u8; Serpent::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Serpent::BLOCK_SIZE * 5];
        let mut ciphertext = vec![2u8; Serpent::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Serpent>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Serpent::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Serpent>::DIGEST_SIZE];

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
    fn ccm_aes128_streaming_ad() {
        use crate::cipher::Aes128;
        use crate::random::{TestRandom, Yarrow};
        let mut rng = Yarrow::default();
        let mut random_chunk_size = move || -> usize {
            rng.next_usize() % (2 * Aes128::BLOCK_SIZE) + 1
        };

        let input_plaintext = vec![1u8; Aes128::BLOCK_SIZE * 10];
        let input_ad = vec![1u8; Aes128::BLOCK_SIZE + 1];
        let mut ciphertext = vec![2u8; Aes128::BLOCK_SIZE * 10];
        let mut digest = vec![2u8; Ccm::<Aes128>::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; Aes128::BLOCK_SIZE * 10];
        let mut output_digest = vec![3u8; Ccm::<Aes128>::DIGEST_SIZE];
        let mut enc = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            input_ad.len(),
            input_plaintext.len(),
            Ccm::<Aes128>::DIGEST_SIZE,
        ).unwrap();
        let mut dec = Ccm::<Aes128>::with_key_and_nonce(
            &vec![1; Aes128::KEY_SIZE],
            &vec![2; 14],
            input_ad.len(),
            input_plaintext.len(),
            Ccm::<Aes128>::DIGEST_SIZE,
        ).unwrap();

        for d in input_ad.chunks(random_chunk_size()) {
            enc.update(d);
        }
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        for d in input_ad.chunks(random_chunk_size()) {
            dec.update(d);
        }
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }
}
