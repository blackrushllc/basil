use nettle_sys::{
    blowfish_ctx, nettle_blowfish_decrypt, nettle_blowfish_encrypt,
    nettle_blowfish_set_key,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Result};

/// The Blowfish block cipher.
///
/// Blowfish is defined in B. Schneiers 1993 paper "Description of a New
/// Variable-Length Key, 64-Bit Block Cipher (Blowfish)" published in "Fast Software Encryption,
/// Cambridge Security Workshop Proceedings" (December 1993), Springer-Verlag, 1994, pp. 191-204.
pub struct Blowfish {
    context: Box<blowfish_ctx>,
}

impl_zeroing_drop_for!(Blowfish);

impl Blowfish {
    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    pub fn with_key(key: &[u8]) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_blowfish_set_key(ctx.as_mut_ptr(), key.len(), key.as_ptr());
            transmute(ctx)
        };

        Blowfish { context }
    }
}

impl Cipher for Blowfish {
    const BLOCK_SIZE: usize = ::nettle_sys::BLOWFISH_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::BLOWFISH_MAX_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Blowfish> {
        Ok(Blowfish::with_key(key))
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Blowfish> {
        Ok(Blowfish::with_key(key))
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_blowfish_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_blowfish_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut blowfish_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_blowfish_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_blowfish_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08"[..]);
        let _ = Blowfish::with_encrypt_key(key).unwrap();
        let _ = Blowfish::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Blowfish::with_encrypt_key(&key).unwrap();
        let mut dec = Blowfish::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
