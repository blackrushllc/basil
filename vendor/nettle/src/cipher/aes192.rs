use nettle_sys::{
    aes192_ctx, nettle_aes192_decrypt, nettle_aes192_encrypt,
    nettle_aes192_invert_key, nettle_aes192_set_decrypt_key,
    nettle_aes192_set_encrypt_key,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Error, Result};

/// 192 bit variant of the Advanced Encryption Standard (AES, formerly RIJNDAEL) defined in FIPS 197.
pub struct Aes192 {
    context: Box<aes192_ctx>,
}

impl_zeroing_drop_for!(Aes192);

impl Aes192 {
    /// Creates a new `Aes192` instance for decryption that uses the same key as `encrypt`.
    ///
    /// The `encrypt` instance must be configured for encryption. This
    /// is faster than calling `with_decrypt_key`.
    pub fn with_inverted_key(encrypt: &Self) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_aes192_invert_key(ctx.as_mut_ptr(),
                                     encrypt.context.as_ref() as *const _);
            transmute(ctx)
        };

        Aes192 { context }
    }
}

impl Cipher for Aes192 {
    const BLOCK_SIZE: usize = ::nettle_sys::AES_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::AES192_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Aes192> {
        if key.len() != Aes192::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_aes192_set_encrypt_key(ctx.as_mut_ptr(), key.as_ptr());
            transmute(ctx)
        };

        Ok(Aes192 { context })
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Aes192> {
        if key.len() != Aes192::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_aes192_set_decrypt_key(ctx.as_mut_ptr(), key.as_ptr());
            transmute(ctx)
        };

        Ok(Aes192 { context })
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_aes192_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_aes192_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut aes192_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_aes192_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_aes192_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15\x16\x09\x10\x11\x12\x13\x14\x15\x16"[..]);
        let _ = Aes192::with_encrypt_key(key).unwrap();
        let _ = Aes192::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x09, 0x10, 0x11, 0x12, 0x13, 0x14,
            0x15, 0x16,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Aes192::with_encrypt_key(&key).unwrap();
        let mut dec = Aes192::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }

    #[test]
    fn round_trip_invert() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x09, 0x10, 0x11, 0x12, 0x13, 0x14,
            0x15, 0x16,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Aes192::with_encrypt_key(&key).unwrap();
        let mut dec = Aes192::with_inverted_key(&enc);

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
