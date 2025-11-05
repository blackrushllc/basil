//! Bellare & Rogaways Probabilistic Signature Scheme (PSS)

use nettle_sys::{
    __gmpz_clear, __gmpz_init, nettle_rsa_pss_sha256_sign_digest_tr,
    nettle_rsa_pss_sha256_verify_digest, nettle_rsa_pss_sha384_sign_digest_tr,
    nettle_rsa_pss_sha384_verify_digest, nettle_rsa_pss_sha512_sign_digest_tr,
    nettle_rsa_pss_sha512_verify_digest,
};
use std::mem::zeroed;

use crate::hash::{Sha256, Sha384, Sha512};
use crate::rsa::{PrivateKey, PublicKey};
use crate::{helper, Error, hash::Hash, random::Random, Result};

/// A hash function usable for PSS.
pub trait PssHash: Hash {
    /// Internal to `sign_pss`.
    fn sign<R: Random>(
        public: &PublicKey,
        private: &PrivateKey,
        random: &mut R,
        salt: &[u8],
        digest: &[u8],
        signature: &mut [u8],
    ) -> Result<()>;
    /// Internal to `verify_pss`.
    fn verify(
        public: &PublicKey,
        salt_len: usize,
        digest: &[u8],
        signature: &[u8],
    ) -> Result<bool>;
}

impl PssHash for Sha256 {
    fn sign<R: Random>(
        public: &PublicKey,
        private: &PrivateKey,
        random: &mut R,
        salt: &[u8],
        digest: &[u8],
        signature: &mut [u8],
    ) -> Result<()> {
        unsafe {
            let mut sig = zeroed();
            __gmpz_init(&mut sig);

            if nettle_rsa_pss_sha256_sign_digest_tr(
                &public.context,
                &private.context,
                random.context(),
                Some(R::random_impl),
                salt.len(),
                salt.as_ptr(),
                digest.as_ptr(),
                &mut sig,
            ) == 1
            {
                helper::write_gmpz_into_slice(sig, signature, "signature")
            } else {
                __gmpz_clear(&mut sig);

                Err(Error::SigningFailed)
            }
        }
    }

    fn verify(
        public: &PublicKey,
        salt_len: usize,
        digest: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        unsafe {
            let mut sig = helper::convert_buffer_to_gmpz(signature);
            let res = nettle_rsa_pss_sha256_verify_digest(
                &public.context,
                salt_len,
                digest.as_ptr(),
                &mut sig,
            ) == 1;

            __gmpz_clear(&mut sig);
            Ok(res)
        }
    }
}

impl PssHash for Sha384 {
    fn sign<R: Random>(
        public: &PublicKey,
        private: &PrivateKey,
        random: &mut R,
        salt: &[u8],
        digest: &[u8],
        signature: &mut [u8],
    ) -> Result<()> {
        unsafe {
            let mut sig = zeroed();
            __gmpz_init(&mut sig);

            if nettle_rsa_pss_sha384_sign_digest_tr(
                &public.context,
                &private.context,
                random.context(),
                Some(R::random_impl),
                salt.len(),
                salt.as_ptr(),
                digest.as_ptr(),
                &mut sig,
            ) == 1
            {
                helper::write_gmpz_into_slice(sig, signature, "signature")
            } else {
                __gmpz_clear(&mut sig);

                Err(Error::SigningFailed)
            }
        }
    }

    fn verify(
        public: &PublicKey,
        salt_len: usize,
        digest: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        unsafe {
            let mut sig = helper::convert_buffer_to_gmpz(signature);
            let res = nettle_rsa_pss_sha384_verify_digest(
                &public.context,
                salt_len,
                digest.as_ptr(),
                &mut sig,
            ) == 1;

            __gmpz_clear(&mut sig);
            Ok(res)
        }
    }
}

impl PssHash for Sha512 {
    fn sign<R: Random>(
        public: &PublicKey,
        private: &PrivateKey,
        random: &mut R,
        salt: &[u8],
        digest: &[u8],
        signature: &mut [u8],
    ) -> Result<()> {
        unsafe {
            let mut sig = zeroed();
            __gmpz_init(&mut sig);

            if nettle_rsa_pss_sha512_sign_digest_tr(
                &public.context,
                &private.context,
                random.context(),
                Some(R::random_impl),
                salt.len(),
                salt.as_ptr(),
                digest.as_ptr(),
                &mut sig,
            ) == 1
            {
                helper::write_gmpz_into_slice(sig, signature, "signature")
            } else {
                __gmpz_clear(&mut sig);

                Err(Error::SigningFailed)
            }
        }
    }

    fn verify(
        public: &PublicKey,
        salt_len: usize,
        digest: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        unsafe {
            let mut sig = helper::convert_buffer_to_gmpz(signature);
            let res = nettle_rsa_pss_sha512_verify_digest(
                &public.context,
                salt_len,
                digest.as_ptr(),
                &mut sig,
            ) == 1;

            __gmpz_clear(&mut sig);
            Ok(res)
        }
    }
}

/// Signs the message hashed by `hash` using `salt` and the key pair
/// `public`/`private`, producing `signature`.
///
/// Expects `signature` to be the size of the modulo of `public`.
///
/// The message is signed using PSS.
pub fn sign_pss<H: PssHash, R: Random>(
    public: &PublicKey,
    private: &PrivateKey,
    salt: &[u8],
    hash: &mut H,
    random: &mut R,
    signature: &mut [u8],
) -> Result<()> {
    let mut dst = vec![0u8; hash.digest_size()];

    hash.digest(&mut dst);
    H::sign(public, private, random, salt, &dst, signature)
}

/// Verifies `signature` of the data hashed by `hash` using a salt of
/// `salt_len` bytes and the key `public`.
///
/// Returns `true` if the signature is valid.
///
/// Expects the message to be PSS encoded.
pub fn verify_pss<H: PssHash>(
    public: &PublicKey,
    salt_len: usize,
    hash: &mut H,
    signature: &[u8],
) -> Result<bool> {
    let mut dst = vec![0u8; hash.digest_size()];

    hash.digest(&mut dst);
    H::verify(public, salt_len, &dst, signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;

    #[test]
    fn rsa_pss() {
        let mut rnd = Yarrow::default();
        let n = &b"\xbc\xb4\x7b\x2e\x0d\xaf\xcb\xa8\x1f\xf2\xa2\xb5\xcb\x11\x5c\xa7\xe7\x57\x18\x4c\x9d\x72\xbc\xdc\xda\x70\x7a\x14\x6b\x3b\x4e\x29\x98\x9d\xdc\x66\x0b\xd6\x94\x86\x5b\x93\x2b\x71\xca\x24\xa3\x35\xcf\x4d\x33\x9c\x71\x91\x83\xe6\x22\x2e\x4c\x9e\xa6\x87\x5a\xcd\x52\x8a\x49\xba\x21\x86\x3f\xe0\x81\x47\xc3\xa4\x7e\x41\x99\x0b\x51\xa0\x3f\x77\xd2\x21\x37\xf8\xd7\x4c\x43\xa5\xa4\x5f\x4e\x9e\x18\xa2\xd1\x5d\xb0\x51\xdc\x89\x38\x5d\xb9\xcf\x83\x74\xb6\x3a\x8c\xc8\x81\x13\x71\x0e\x6d\x81\x79\x07\x5b\x7d\xc7\x9e\xe7\x6b"[..];
        let e = &b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x01"[..];
        let d = &b"\x38\x3a\x6f\x19\xe1\xea\x27\xfd\x08\xc7\xfb\xc3\xbf\xa6\x84\xbd\x63\x29\x88\x8c\x0b\xbe\x4c\x98\x62\x5e\x71\x81\xf4\x11\xcf\xd0\x85\x31\x44\xa3\x03\x94\x04\xdd\xa4\x1b\xce\x2e\x31\xd5\x88\xec\x57\xc0\xe1\x48\x14\x6f\x0f\xa6\x5b\x39\x00\x8b\xa5\x83\x5f\x82\x9b\xa3\x5a\xe2\xf1\x55\xd6\x1b\x8a\x12\x58\x1b\x99\xc9\x27\xfd\x2f\x22\x25\x2c\x5e\x73\xcb\xa4\xa6\x10\xdb\x39\x73\xe0\x19\xee\x0f\x95\x13\x0d\x43\x19\xed\x41\x34\x32\xf2\xe5\xe2\x0d\x52\x15\xcd\xd2\x7c\x21\x64\x20\x6b\x3f\x80\xed\xee\x51\x93\x8a\x25\xc1"[..];
        let p = &b"\xd2\xa4\xec\x0f\xa2\x22\x6c\xde\x82\xda\x77\x65\x3b\x07\x2c\xd0\x98\x53\x5d\x3e\x90\xed\x4d\x72\x24\xdc\xb8\xcb\x8b\x93\x14\x76\x8d\xc5\x17\xe2\x2d\x7c\x8f\xa1\x3f\x25\x3d\xaa\x74\x65\xa7\x99\x56\x09\x8a\xa4\xcc\x3a\x6e\x35\xe8\xb1\xfc\xc4\xf9\x7e\x77\x4f"[..];
        let q = &b"\xe5\x56\x3b\x14\x5d\xb6\xff\x5a\x16\x28\x0d\x3e\x80\xef\xf0\x2f\x18\x1d\xbd\x03\x32\x4e\xf2\x47\xf5\x96\xa4\xd4\xa7\xb8\xda\xa3\x2b\x99\x34\xe3\xc7\xf4\xdc\xf6\xa3\x10\x54\x62\xde\xc6\x38\x39\x63\x86\x18\x41\x8b\x51\xdb\x02\x69\x3f\xab\xb4\xe6\x83\x87\x25"[..];
        let public = PublicKey::new(n, e).unwrap();
        let private = PrivateKey::new(d, p, q, None).unwrap();

        {
            let salt = &b"\x6f\x28\x41\x16\x6a\x64\x47\x1d\x4f\x0b\x8e\xd0\xdb\xb7\xdb\x32\x16\x1d\xa1\x3b"[..];
            let m = &b"\x12\x48\xf6\x2a\x43\x89\xf4\x2f\x7b\x4b\xb1\x31\x05\x3d\x6c\x88\xa9\x94\xdb\x20\x75\xb9\x12\xcc\xbe\x3e\xa7\xdc\x61\x17\x14\xf1\x4e\x07\x5c\x10\x48\x58\xf2\xf6\xe6\xcf\xd6\xab\xde\xdf\x01\x5a\x82\x1d\x03\x60\x8b\xf4\xeb\xa3\x16\x9a\x67\x25\xec\x42\x2c\xd9\x06\x94\x98\xb5\x51\x5a\x96\x08\xae\x7c\xc3\x0e\x3d\x2e\xcf\xc1\xdb\x68\x25\xf3\xe9\x96\xce\x9a\x50\x92\x92\x6b\xc1\xcf\x61\xaa\x42\xd7\xf2\x40\xe6\xf7\xaa\x0e\xdb\x38\xbf\x81\xaa\x92\x9d\x66\xbb\x5d\x89\x00\x18\x08\x84\x58\x72\x0d\x72\xd5\x69\x24\x7b\x0c"[..];
            let expected = &b"\x7b\x1d\x37\x27\x8e\x54\x98\x98\xd4\x08\x4e\x22\x10\xc4\xa9\x96\x1e\xdf\xe7\xb5\x96\x35\x50\xcc\xa1\x90\x42\x48\xc8\x68\x15\x13\x53\x90\x17\x82\x0f\x0e\x9b\xd0\x74\xb9\xf8\xa0\x67\xb9\xfe\xff\xf7\xf1\xfa\x20\xbf\x2d\x0c\x75\x01\x5f\xf0\x20\xb2\x21\x0c\xc7\xf7\x90\x34\xfe\xdf\x68\xe8\xd4\x4a\x00\x7a\xbf\x4d\xd8\x2c\x26\xe8\xb0\x03\x93\x72\x3a\xea\x15\xab\xfb\xc2\x29\x41\xc8\xcf\x79\x48\x17\x18\xc0\x08\xda\x71\x3f\xb8\xf5\x4c\xb3\xfc\xa8\x90\xbd\xe1\x13\x73\x14\x33\x4b\x9b\x0a\x18\x51\x5b\xfa\x48\xe5\xcc\xd0"[..];

            let mut hsh = Sha256::default();
            let mut sig = vec![0u8; 1024 / 8];

            hsh.update(m);
            assert!(sign_pss(
                &public, &private, salt, &mut hsh, &mut rnd, &mut sig
            )
            .is_ok());
            assert_eq!(sig, expected);
            hsh.update(m);
            assert_eq!(
                verify_pss(&public, salt.len(), &mut hsh, expected).ok(),
                Some(true)
            );
        }

        {
            let salt = &b"\x6f\x28\x41\x16\x6a\x64\x47\x1d\x4f\x0b\x8e\xd0\xdb\xb7\xdb\x32\x16\x1d\xa1\x3b"[..];
            let m = &b"\x10\x34\xe0\x43\xd5\x21\xa2\x9e\x1f\x6d\x84\xef\x8a\x54\x9b\x86\x75\xc5\xeb\xe4\xab\x74\x2e\x72\x43\x2b\xd1\x72\xc4\x60\x40\x35\x8d\x0d\x63\x8e\x74\xbc\x68\x88\xae\xe1\xc4\x54\xef\x6d\x74\x85\xd1\xa0\x89\x80\x7a\x1f\xe3\x2d\x79\x46\x3d\xf3\xf2\xbf\x6a\xe4\xfb\x54\xdd\x34\x45\x01\x6f\xf0\xff\x4e\x5d\xec\xf7\xdc\x90\xfe\x03\x3d\x22\x69\x9f\x08\x1d\x9c\xd7\x2d\x86\x1d\x68\x87\x9a\xc0\x22\xb0\x57\x1c\x90\x19\x3e\x3f\xdb\x5a\x58\xa1\xec\xc6\x77\x57\xd1\x43\xbe\x89\x62\x59\x63\x26\x24\x88\xb4\xd6\x6d\xdf\x58\x31"[..];
            let expected = &b"\x0f\xdc\x48\xfa\x2f\xe5\x6e\xd5\x35\xf7\xe1\xb0\x32\x05\x4c\xcc\x64\xa7\x36\x3e\xb1\x45\xeb\x01\xb2\xe8\x4e\xfa\x35\x88\xde\xf1\xf3\x31\x25\xcf\x71\xa8\xef\x5b\x6f\x2f\x7b\x6a\xe1\xe6\x16\xd9\xaf\xcc\xd7\x37\x2c\x84\x3a\x97\x0f\x16\x5c\x10\x3a\x69\x41\x63\x18\xf5\x57\x4f\xf3\xb2\x86\x99\x2b\x78\xcb\x8e\x77\x0f\x85\xa1\xbd\x0f\x0f\x0b\x3d\xdf\xb7\x77\x12\xb4\x27\x7c\x19\x10\xc6\x8a\xa8\xc6\x00\x6c\xe8\x46\x3a\x45\xc9\x32\xa8\x4c\x23\x34\xf2\xf1\xd4\xf6\x7e\x7e\x55\x01\x07\x56\x86\xcf\x80\x96\xb1\x75\x14\xc6"[..];

            let mut hsh = Sha384::default();
            let mut sig = vec![0u8; 1024 / 8];

            hsh.update(m);
            assert!(sign_pss(
                &public, &private, salt, &mut hsh, &mut rnd, &mut sig
            )
            .is_ok());
            assert_eq!(sig, expected);
            hsh.update(m);
            assert_eq!(
                verify_pss(&public, salt.len(), &mut hsh, expected).ok(),
                Some(true)
            );
        }
        {
            let salt = &b"\x6f\x28\x41\x16\x6a\x64\x47\x1d\x4f\x0b\x8e\xd0\xdb\xb7\xdb\x32\x16\x1d\xa1\x3b"[..];
            let m = &b"\x5d\x95\x51\xdf\x21\x0c\xea\x61\xf9\xae\xc3\xd2\xef\x2f\x57\x16\x69\x2c\x5c\x55\xff\xb1\x3d\xa2\x8a\xfe\xca\xda\x7d\x8b\x2f\xcf\x79\x49\x22\xa6\x1b\x2b\x61\x7e\x19\xe5\x5c\x53\x8d\xf4\x2e\x21\x90\xde\xd2\x4c\x0b\xbb\x94\x3f\xca\xf4\xc5\x53\xd2\x4b\xb3\xf3\xf5\x1b\xd9\xff\xa3\xe1\x52\x6d\x03\xfc\x94\x36\x42\xb1\x5a\x70\x2c\x72\xe2\xb7\xf9\x60\x59\xd2\x69\x0e\x94\x94\x13\x88\x00\xff\xf9\x94\xdc\xb0\x61\x37\xa5\x77\xe0\x7b\xd2\x45\xb8\x70\xc2\x4b\x64\x71\xf3\x60\x70\xa8\x74\xa3\x8e\xd5\x9d\x62\xe7\x4a\xfd\x3b"[..];
            let expected = &b"\x38\x88\x9a\x9f\x36\x46\xf6\x75\x67\x53\x8d\xe1\xdd\xdb\xee\xed\x36\x40\x74\xba\x58\x2f\xcb\x1a\x13\x02\x9c\x00\xf1\x3c\x88\xfc\x76\x9f\xf1\xcb\x6c\x4f\xd0\x51\x8d\x41\xd0\x88\xb5\xfd\x37\xaf\x92\xd1\x63\xe2\x24\xe4\x18\x30\x18\xed\xc9\xa7\xd1\xe4\x64\x8b\x6d\x95\xe2\x1a\xda\xf7\x7b\x39\x01\x50\xb2\x21\xf4\x56\x07\x5e\x5d\x56\x6f\x20\x49\xac\xf7\xb6\x13\x5c\x7a\xfa\x2f\x0c\xd8\x2e\x7d\xb1\xad\x24\x2e\x5b\xe0\xe0\xc8\xd2\x9a\x36\xc8\x29\x2e\x42\x9a\xd6\xaa\x52\xd3\x32\xb2\x9e\xb9\x50\x7a\xe9\xc1\xf3\x8d\xd1"[..];

            let mut hsh = Sha512::default();
            let mut sig = vec![0u8; 1024 / 8];

            hsh.update(m);
            assert!(sign_pss(
                &public, &private, salt, &mut hsh, &mut rnd, &mut sig
            )
            .is_ok());
            assert_eq!(sig, expected);
            hsh.update(m);
            assert_eq!(
                verify_pss(&public, salt.len(), &mut hsh, expected).ok(),
                Some(true)
            );
        }
    }
}
