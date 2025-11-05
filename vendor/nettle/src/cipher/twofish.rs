use nettle_sys::{
    nettle_twofish_decrypt, nettle_twofish_encrypt, nettle_twofish_set_key,
    twofish_ctx,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Result};

/// The Twofish block cipher.
pub struct Twofish {
    context: Box<twofish_ctx>,
}

impl_zeroing_drop_for!(Twofish);

impl Twofish {
    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    pub fn with_key(key: &[u8]) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_twofish_set_key(ctx.as_mut_ptr(), key.len(), key.as_ptr());
            transmute(ctx)
        };

        Twofish { context }
    }
}

impl Cipher for Twofish {
    const BLOCK_SIZE: usize = ::nettle_sys::TWOFISH_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::TWOFISH_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Twofish> {
        Ok(Twofish::with_key(key))
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Twofish> {
        Ok(Twofish::with_key(key))
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_twofish_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_twofish_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut twofish_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_twofish_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_twofish_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08\x01\x02\x03\x04\x05\x06\x07\x08"[..]);
        let _ = Twofish::with_encrypt_key(key).unwrap();
        let _ = Twofish::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x02, 0x03,
            0x04, 0x05, 0x06, 0x07, 0x08,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Twofish::with_encrypt_key(&key).unwrap();
        let mut dec = Twofish::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
