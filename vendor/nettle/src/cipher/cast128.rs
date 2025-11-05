use nettle_sys::{
    cast128_ctx, nettle_cast128_decrypt, nettle_cast128_encrypt,
    nettle_cast5_set_key,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Result};

/// The CAST-128 block cipher defined in RFC 2144.
pub struct Cast128 {
    context: Box<cast128_ctx>,
}

impl_zeroing_drop_for!(Cast128);

impl Cast128 {
    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    pub fn with_key(key: &[u8]) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_cast5_set_key(ctx.as_mut_ptr(), key.len(), key.as_ptr());
            transmute(ctx)
        };

        Cast128 { context }
    }
}

impl Cipher for Cast128 {
    const BLOCK_SIZE: usize = ::nettle_sys::CAST128_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::CAST5_MAX_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Cast128> {
        Ok(Cast128::with_key(key))
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Cast128> {
        Ok(Cast128::with_key(key))
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_cast128_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_cast128_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut cast128_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_cast128_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_cast128_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08"[..]);
        let _ = Cast128::with_encrypt_key(key).unwrap();
        let _ = Cast128::with_decrypt_key(key).unwrap();
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

        let mut enc = Cast128::with_encrypt_key(&key).unwrap();
        let mut dec = Cast128::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
