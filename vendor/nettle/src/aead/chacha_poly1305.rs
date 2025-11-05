use nettle_sys::{
    chacha_poly1305_ctx, nettle_chacha_poly1305_decrypt,
    nettle_chacha_poly1305_digest, nettle_chacha_poly1305_encrypt,
    nettle_chacha_poly1305_set_key, nettle_chacha_poly1305_set_nonce,
    nettle_chacha_poly1305_update,
};
use std::cmp::min;
use std::mem::zeroed;

use crate::errors::Error;
use crate::{aead::Aead, Result};

/// D.J. Bernsteins ChaCha-Poly1305 AEAD stream cipher.
pub struct ChaChaPoly1305 {
    context: chacha_poly1305_ctx,
}

impl ChaChaPoly1305 {
    /// Size of a Poly1305 digest in bytes.
    pub const DIGEST_SIZE: usize =
        ::nettle_sys::CHACHA_POLY1305_DIGEST_SIZE as usize;
    /// Size of the ChaCha key in bytes.
    pub const KEY_SIZE: usize = ::nettle_sys::CHACHA_POLY1305_KEY_SIZE as usize;
    /// Size of the ChaChaPoly1305 nonce in bytes.
    pub const NONCE_SIZE: usize =
        ::nettle_sys::CHACHA_POLY1305_NONCE_SIZE as usize;

    /// Creates a new ChaChaPoly1305 instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != Self::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        if nonce.len() != Self::NONCE_SIZE {
            return Err(Error::InvalidArgument { argument_name: "nonce" });
        }

        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_chacha_poly1305_set_key(&mut ctx as *mut _, key.as_ptr());
            nettle_chacha_poly1305_set_nonce(
                &mut ctx as *mut _,
                nonce.as_ptr(),
            );
        }

        Ok(ChaChaPoly1305 { context: ctx })
    }
}

impl Aead for ChaChaPoly1305 {
    fn digest_size(&self) -> usize {
        ::nettle_sys::CHACHA_POLY1305_DIGEST_SIZE as usize
    }

    fn update(&mut self, ad: &[u8]) {
        unsafe {
            nettle_chacha_poly1305_update(
                &mut self.context as *mut _,
                ad.len(),
                ad.as_ptr(),
            );
        }
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_chacha_poly1305_encrypt(
                &mut self.context as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_chacha_poly1305_decrypt(
                &mut self.context as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_chacha_poly1305_digest(
                &mut self.context as *mut _,
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
    fn round_trip() {
        let mut enc = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let mut dec = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let input_ad = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 5];
        let mut ciphertext = vec![2u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut digest = vec![2u8; ChaChaPoly1305::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut output_digest = vec![3u8; ChaChaPoly1305::DIGEST_SIZE];

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
    fn modify_ad() {
        let mut enc = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let mut dec = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut input_ad = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 5];
        let mut ciphertext = vec![2u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut digest = vec![2u8; ChaChaPoly1305::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut output_digest = vec![3u8; ChaChaPoly1305::DIGEST_SIZE];

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
    fn modify_ciphertext() {
        let mut enc = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let mut dec = ChaChaPoly1305::with_key_and_nonce(
            &vec![1; ChaChaPoly1305::KEY_SIZE],
            &vec![2; ChaChaPoly1305::NONCE_SIZE],
        )
        .unwrap();
        let input_plaintext = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let input_ad = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 5];
        let mut ciphertext = vec![2u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut digest = vec![2u8; ChaChaPoly1305::DIGEST_SIZE];
        let mut output_plaintext = vec![3u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut output_digest = vec![3u8; ChaChaPoly1305::DIGEST_SIZE];

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
    fn streaming_ad() {
        use crate::random::{TestRandom, Yarrow};
        let mut rng = Yarrow::default();
        let mut random_chunk_size = move || -> usize {
            rng.next_usize() % (2 * ChaChaPoly1305::NONCE_SIZE) + 1
        };

        let key = vec![1; ChaChaPoly1305::KEY_SIZE];
        let nonce = vec![2; ChaChaPoly1305::NONCE_SIZE];
        let input_plaintext = vec![1u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let input_ad = vec![1u8; ChaChaPoly1305::NONCE_SIZE + 1];
        let mut ciphertext = vec![2u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut digest = vec![2u8; ChaChaPoly1305::DIGEST_SIZE];

        let mut enc = ChaChaPoly1305::with_key_and_nonce(&key, &nonce).unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            enc.update(d);
        }
        enc.encrypt(&mut ciphertext, &input_plaintext);
        enc.digest(&mut digest);

        let mut output_plaintext = vec![3u8; ChaChaPoly1305::NONCE_SIZE * 10];
        let mut output_digest = vec![3u8; ChaChaPoly1305::DIGEST_SIZE];

        let mut dec = ChaChaPoly1305::with_key_and_nonce(&key, &nonce).unwrap();
        for d in input_ad.chunks(random_chunk_size()) {
            dec.update(d);
        }
        dec.decrypt(&mut output_plaintext, &ciphertext);
        dec.digest(&mut output_digest);

        assert_eq!(input_plaintext, output_plaintext);
        assert_eq!(digest, output_digest);
    }

    #[test]
    fn rfc_7539() {
        let key = b"\x1c\x92\x40\xa5\xeb\x55\xd3\x8a\xf3\x33\x88\x86\x04\xf6\xb5\xf0\x47\x39\x17\xc1\x40\x2b\x80\x09\x9d\xca\x5c\xbc\x20\x70\x75\xc0";
        let cipher = b"\x64\xa0\x86\x15\x75\x86\x1a\xf4\x60\xf0\x62\xc7\x9b\xe6\x43\xbd\x5e\x80\x5c\xfd\x34\x5c\xf3\x89\xf1\x08\x67\x0a\xc7\x6c\x8c\xb2\x4c\x6c\xfc\x18\x75\x5d\x43\xee\xa0\x9e\xe9\x4e\x38\x2d\x26\xb0\xbd\xb7\xb7\x3c\x32\x1b\x01\x00\xd4\xf0\x3b\x7f\x35\x58\x94\xcf\x33\x2f\x83\x0e\x71\x0b\x97\xce\x98\xc8\xa8\x4a\xbd\x0b\x94\x81\x14\xad\x17\x6e\x00\x8d\x33\xbd\x60\xf9\x82\xb1\xff\x37\xc8\x55\x97\x97\xa0\x6e\xf4\xf0\xef\x61\xc1\x86\x32\x4e\x2b\x35\x06\x38\x36\x06\x90\x7b\x6a\x7c\x02\xb0\xf9\xf6\x15\x7b\x53\xc8\x67\xe4\xb9\x16\x6c\x76\x7b\x80\x4d\x46\xa5\x9b\x52\x16\xcd\xe7\xa4\xe9\x90\x40\xc5\xa4\x04\x33\x22\x5e\xe2\x82\xa1\xb0\xa0\x6c\x52\x3e\xaf\x45\x34\xd7\xf8\x3f\xa1\x15\x5b\x00\x47\x71\x8c\xbc\x54\x6a\x0d\x07\x2b\x04\xb3\x56\x4e\xea\x1b\x42\x22\x73\xf5\x48\x27\x1a\x0b\xb2\x31\x60\x53\xfa\x76\x99\x19\x55\xeb\xd6\x31\x59\x43\x4e\xce\xbb\x4e\x46\x6d\xae\x5a\x10\x73\xa6\x72\x76\x27\x09\x7a\x10\x49\xe6\x17\xd9\x1d\x36\x10\x94\xfa\x68\xf0\xff\x77\x98\x71\x30\x30\x5b\xea\xba\x2e\xda\x04\xdf\x99\x7b\x71\x4d\x6c\x6f\x2c\x29\xa6\xad\x5c\xb4\x02\x2b\x02\x70\x9b";
        let nonce = b"\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08";
        let ad = b"\xf3\x33\x88\x86\x00\x00\x00\x00\x00\x00\x4e\x91";
        let tag =
            b"\xee\xad\x9d\x67\x89\x0c\xbb\x22\x39\x23\x36\xfe\xa1\x85\x1f\x38";
        let plain = b"\x49\x6e\x74\x65\x72\x6e\x65\x74\x2d\x44\x72\x61\x66\x74\x73\x20\x61\x72\x65\x20\x64\x72\x61\x66\x74\x20\x64\x6f\x63\x75\x6d\x65\x6e\x74\x73\x20\x76\x61\x6c\x69\x64\x20\x66\x6f\x72\x20\x61\x20\x6d\x61\x78\x69\x6d\x75\x6d\x20\x6f\x66\x20\x73\x69\x78\x20\x6d\x6f\x6e\x74\x68\x73\x20\x61\x6e\x64\x20\x6d\x61\x79\x20\x62\x65\x20\x75\x70\x64\x61\x74\x65\x64\x2c\x20\x72\x65\x70\x6c\x61\x63\x65\x64\x2c\x20\x6f\x72\x20\x6f\x62\x73\x6f\x6c\x65\x74\x65\x64\x20\x62\x79\x20\x6f\x74\x68\x65\x72\x20\x64\x6f\x63\x75\x6d\x65\x6e\x74\x73\x20\x61\x74\x20\x61\x6e\x79\x20\x74\x69\x6d\x65\x2e\x20\x49\x74\x20\x69\x73\x20\x69\x6e\x61\x70\x70\x72\x6f\x70\x72\x69\x61\x74\x65\x20\x74\x6f\x20\x75\x73\x65\x20\x49\x6e\x74\x65\x72\x6e\x65\x74\x2d\x44\x72\x61\x66\x74\x73\x20\x61\x73\x20\x72\x65\x66\x65\x72\x65\x6e\x63\x65\x20\x6d\x61\x74\x65\x72\x69\x61\x6c\x20\x6f\x72\x20\x74\x6f\x20\x63\x69\x74\x65\x20\x74\x68\x65\x6d\x20\x6f\x74\x68\x65\x72\x20\x74\x68\x61\x6e\x20\x61\x73\x20\x2f\xe2\x80\x9c\x77\x6f\x72\x6b\x20\x69\x6e\x20\x70\x72\x6f\x67\x72\x65\x73\x73\x2e\x2f\xe2\x80\x9d";
        let mut c =
            ChaChaPoly1305::with_key_and_nonce(&key[..], &nonce[..]).unwrap();
        let mut got_plain = vec![0u8; plain.len()];
        let mut got_tag = vec![0u8; tag.len()];

        c.update(&ad[..]);
        c.decrypt(&mut got_plain[..], &cipher[..]);
        c.digest(&mut got_tag[..]);

        assert_eq!(&plain[..], &got_plain[..]);
        assert_eq!(&tag[..], &got_tag[..]);
    }
}
