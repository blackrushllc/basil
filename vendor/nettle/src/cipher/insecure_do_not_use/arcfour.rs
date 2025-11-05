use nettle_sys::{arcfour_ctx, nettle_arcfour_crypt, nettle_arcfour_set_key};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Result};

/// Ron Rivest's RC4 stream cipher.
///
/// # Note
/// RC4 is no longer considered a secure cipher. Only use it for legacy applications.
pub struct ArcFour {
    context: Box<arcfour_ctx>,
}

impl_zeroing_drop_for!(ArcFour);

impl ArcFour {
    /// Create a new instance with `key`.
    pub fn with_key(key: &[u8]) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_arcfour_set_key(ctx.as_mut_ptr(), key.len(), key.as_ptr());
            transmute(ctx)
        };

        ArcFour { context }
    }

    /// Encrypt/decrypt data from `src` to `dst`.
    pub fn crypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_arcfour_crypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }
}

impl Cipher for ArcFour {
    const BLOCK_SIZE: usize = 1;
    const KEY_SIZE: usize = ::nettle_sys::ARCFOUR_MAX_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<ArcFour> {
        Ok(ArcFour::with_key(key))
    }

    fn with_decrypt_key(key: &[u8]) -> Result<ArcFour> {
        Ok(ArcFour::with_key(key))
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.crypt(dst, src)
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.crypt(dst, src)
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut arcfour_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_arcfour_crypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_arcfour_crypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15\x16\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15\x16"[..]);
        let _ = ArcFour::with_encrypt_key(key).unwrap();
        let _ = ArcFour::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            0x07, 0x08, 0x09, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = ArcFour::with_encrypt_key(&key).unwrap();
        let mut dec = ArcFour::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
