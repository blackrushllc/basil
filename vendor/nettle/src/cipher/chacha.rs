use nettle_sys::{
    chacha_ctx, nettle_chacha_crypt, nettle_chacha_set_key,
    nettle_chacha_set_nonce,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};

use crate::{Error, Result};

/// D.J. Bernstein's ChaCha block cipher.
pub struct ChaCha {
    context: Box<chacha_ctx>,
}

impl_zeroing_drop_for!(ChaCha);

impl ChaCha {
    /// ChaCha block size in bytes.
    pub const BLOCK_SIZE: usize = ::nettle_sys::CHACHA_BLOCK_SIZE as usize;

    /// ChaCha key size in bytes.
    pub const KEY_SIZE: usize = ::nettle_sys::CHACHA_KEY_SIZE as usize;

    /// ChaCha nonce size in bytes.
    pub const NONCE_SIZE: usize = ::nettle_sys::CHACHA_NONCE_SIZE as usize;

    /// Create a new instance with `key`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != ChaCha::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }
        if nonce.len() != ChaCha::NONCE_SIZE {
            return Err(Error::InvalidArgument { argument_name: "nonce" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_chacha_set_key(ctx.as_mut_ptr(), key.as_ptr());
            nettle_chacha_set_nonce(ctx.as_mut_ptr(), nonce.as_ptr());
            transmute(ctx)
        };

        Ok(ChaCha { context })
    }

    /// Encrypt/decrypt data from `src` to `dst`.
    pub fn crypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_chacha_crypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key_and_nonce() {
        let key = vec![0; 32];
        let nonce = vec![1; 8];

        let _ = ChaCha::with_key_and_nonce(&key, &nonce).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![0; 32];
        let nonce = vec![1; 8];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = ChaCha::with_key_and_nonce(&key, &nonce).unwrap();
        let mut dec = ChaCha::with_key_and_nonce(&key, &nonce).unwrap();

        enc.crypt(&mut cipher, &input);
        dec.crypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
