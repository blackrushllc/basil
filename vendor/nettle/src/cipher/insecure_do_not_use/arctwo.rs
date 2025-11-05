use nettle_sys::{
    arctwo_ctx, nettle_arctwo_decrypt, nettle_arctwo_encrypt,
    nettle_arctwo_set_key, nettle_arctwo_set_key_ekb,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Result};

/// Ron Rivest's RC2 block cipher defined RFC 2268.
///
/// # Note
/// RC2 is no longer considered a secure cipher. Only use it for legacy applications.
pub struct ArcTwo {
    context: Box<arctwo_ctx>,
}

impl_zeroing_drop_for!(ArcTwo);

impl ArcTwo {
    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    pub fn with_key(key: &[u8]) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_arctwo_set_key(ctx.as_mut_ptr(), key.len(), key.as_ptr());
            transmute(ctx)
        };

        ArcTwo { context }
    }

    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    ///
    /// The `ekb` parameter can be used to limit the effective key
    /// size to `ekb` bits. A value of `0` is handled as `1024`.
    pub fn with_key_ekb(key: &[u8], ekb: u32) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_arctwo_set_key_ekb(
                ctx.as_mut_ptr(),
                key.len(),
                key.as_ptr(),
                ekb,
            );
            transmute(ctx)
        };

        ArcTwo { context }
    }
}

impl Cipher for ArcTwo {
    const BLOCK_SIZE: usize = 8;
    const KEY_SIZE: usize = ::nettle_sys::ARCTWO_MAX_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<ArcTwo> {
        Ok(ArcTwo::with_key(key))
    }

    fn with_decrypt_key(key: &[u8]) -> Result<ArcTwo> {
        Ok(ArcTwo::with_key(key))
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_arctwo_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_arctwo_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut arctwo_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_arctwo_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_arctwo_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08"[..]);
        let _ = ArcTwo::with_encrypt_key(key).unwrap();
        let _ = ArcTwo::with_decrypt_key(key).unwrap();
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

        let mut enc = ArcTwo::with_encrypt_key(&key).unwrap();
        let mut dec = ArcTwo::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }

    #[test]
    fn round_trip_ekb() {
        let key = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = ArcTwo::with_key_ekb(&key, 42);
        let mut dec = ArcTwo::with_key_ekb(&key, 42);

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
