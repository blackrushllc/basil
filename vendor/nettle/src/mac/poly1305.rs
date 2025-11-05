use nettle_sys::{
    nettle_poly1305_aes_digest, nettle_poly1305_aes_set_key,
    nettle_poly1305_aes_set_nonce, nettle_poly1305_aes_update,
    poly1305_aes_ctx, POLY1305_AES_DIGEST_SIZE, POLY1305_AES_KEY_SIZE,
    POLY1305_AES_NONCE_SIZE,
};
use std::mem::zeroed;

use crate::{Error, mac::Mac, Result};

/// D.J. Bernsteins Poly1305-AES message authentication code (RFC 7539).
pub struct Poly1305 {
    context: poly1305_aes_ctx,
}

impl Poly1305 {
    /// Create a new MAC instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != POLY1305_AES_KEY_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }
        if nonce.len() != POLY1305_AES_NONCE_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "nonce" });
        }

        unsafe {
            let mut ret: Poly1305 = zeroed();

            nettle_poly1305_aes_set_key(
                &mut ret.context as *mut _,
                key.as_ptr(),
            );

            nettle_poly1305_aes_set_nonce(
                &mut ret.context as *mut _,
                nonce.as_ptr(),
            );
            Ok(ret)
        }
    }
}

impl Mac for Poly1305 {
    fn mac_size(&self) -> usize {
        POLY1305_AES_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_poly1305_aes_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) -> Result<()> {
        if digest.len() != self.mac_size() {
            return Err(
                Error::InvalidArgument { argument_name: "digest" }
            );
        }

        unsafe {
            nettle_poly1305_aes_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn djb_test_vectors() {
        {
            let key = &b"\xec\x07\x4c\x83\x55\x80\x74\x17\x01\x42\x5b\x62\x32\x35\xad\xd6\x85\x1f\xc4\x0c\x34\x67\xac\x0b\xe0\x5c\xc2\x04\x04\xf3\xf7\x00"[..];
            let nonce = &b"\xfb\x44\x73\x50\xc4\xe8\x68\xc5\x2a\xc3\x27\x5c\xf9\xd4\x32\x7e"[..];
            let msg = &b"\xf3\xf6"[..];
            let mut poly1305 =
                Poly1305::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0; poly1305.mac_size()];

            poly1305.update(msg);
            poly1305.digest(&mut mac).unwrap();
            assert_eq!(&mac[..], &b"\xf4\xc6\x33\xc3\x04\x4f\xc1\x45\xf8\x4f\x33\x5c\xb8\x19\x53\xde"[..]);
        }

        {
            let key = &b"\x75\xde\xaa\x25\xc0\x9f\x20\x8e\x1d\xc4\xce\x6b\x5c\xad\x3f\xbf\xa0\xf3\x08\x00\x00\xf4\x64\x00\xd0\xc7\xe9\x07\x6c\x83\x44\x03"[..];
            let nonce = &b"\x61\xee\x09\x21\x8d\x29\xb0\xaa\xed\x7e\x15\x4a\x2c\x55\x09\xcc"[..];
            let msg = &b""[..];
            let res = &b"\xdd\x3f\xab\x22\x51\xf1\x1a\xc7\x59\xf0\x88\x71\x29\xcc\x2e\xe7"[..];
            let mut poly1305 =
                Poly1305::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0; poly1305.mac_size()];

            poly1305.update(msg);
            poly1305.digest(&mut mac).unwrap();
            assert_eq!(&mac[..], res);
        }

        {
            let key = &b"\x6a\xcb\x5f\x61\xa7\x17\x6d\xd3\x20\xc5\xc1\xeb\x2e\xdc\xdc\x74\x48\x44\x3d\x0b\xb0\xd2\x11\x09\xc8\x9a\x10\x0b\x5c\xe2\xc2\x08"[..];
            let nonce = &b"\xae\x21\x2a\x55\x39\x97\x29\x59\x5d\xea\x45\x8b\xc6\x21\xff\x0e"[..];
            let msg = &b"\x66\x3c\xea\x19\x0f\xfb\x83\xd8\x95\x93\xf3\xf4\x76\xb6\xbc\x24\xd7\xe6\x79\x10\x7e\xa2\x6a\xdb\x8c\xaf\x66\x52\xd0\x65\x61\x36"[..];
            let res = &b"\x0e\xe1\xc1\x6b\xb7\x3f\x0f\x4f\xd1\x98\x81\x75\x3c\x01\xcd\xbe"[..];

            let mut poly1305 =
                Poly1305::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0; poly1305.mac_size()];

            poly1305.update(msg);
            poly1305.digest(&mut mac).unwrap();
            assert_eq!(&mac[..], res);
        }

        {
            let key = &b"\xe1\xa5\x66\x8a\x4d\x5b\x66\xa5\xf6\x8c\xc5\x42\x4e\xd5\x98\x2d\x12\x97\x6a\x08\xc4\x42\x6d\x0c\xe8\xa8\x24\x07\xc4\xf4\x82\x07"[..];
            let nonce = &b"\x9a\xe8\x31\xe7\x43\x97\x8d\x3a\x23\x52\x7c\x71\x28\x14\x9e\x3a"[..];
            let msg = &b"\xab\x08\x12\x72\x4a\x7f\x1e\x34\x27\x42\xcb\xed\x37\x4d\x94\xd1\x36\xc6\xb8\x79\x5d\x45\xb3\x81\x98\x30\xf2\xc0\x44\x91\xfa\xf0\x99\x0c\x62\xe4\x8b\x80\x18\xb2\xc3\xe4\xa0\xfa\x31\x34\xcb\x67\xfa\x83\xe1\x58\xc9\x94\xd9\x61\xc4\xcb\x21\x09\x5c\x1b\xf9"[..];
            let res = &b"\x51\x54\xad\x0d\x2c\xb2\x6e\x01\x27\x4f\xc5\x11\x48\x49\x1f\x1b"[..];
            let mut poly1305 =
                Poly1305::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0; poly1305.mac_size()];

            poly1305.update(msg);
            poly1305.digest(&mut mac).unwrap();
            assert_eq!(&mac[..], res);
        }
    }
}
