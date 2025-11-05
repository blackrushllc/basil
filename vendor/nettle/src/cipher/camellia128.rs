use nettle_sys::{
    camellia128_ctx,
    nettle_camellia128_crypt,
    nettle_camellia128_invert_key,
    nettle_camellia128_set_encrypt_key,
    nettle_camellia_set_decrypt_key, // WTF?
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Error, Result};

/// 128 bit variant of the Camellia block cipher developed by Mitsubishi & NTT, defined in RFC 3713.
pub struct Camellia128 {
    context: Box<camellia128_ctx>,
}

impl_zeroing_drop_for!(Camellia128);

impl Camellia128 {
    /// Creates a new `Camellia128` instance for decryption that uses the same key as `encrypt`.
    ///
    /// The `encrypt` instance must be configured for encryption. This
    /// is faster than calling `with_decrypt_key`.
    pub fn with_inverted_key(encrypt: &Self) -> Self {
        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_camellia128_invert_key(ctx.as_mut_ptr(),
                                          encrypt.context.as_ref() as *const _);
            transmute(ctx)
        };

        Camellia128 { context }
    }

    /// Encrypt/decrypt data from `src` to `dst`.
    pub fn crypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_camellia128_crypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }
}

impl Cipher for Camellia128 {
    const BLOCK_SIZE: usize = ::nettle_sys::CAMELLIA_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::CAMELLIA128_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Camellia128> {
        if key.len() != Camellia128::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_camellia128_set_encrypt_key(ctx.as_mut_ptr(), key.as_ptr());
            transmute(ctx)
        };

        Ok(Camellia128 { context })
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Camellia128> {
        if key.len() != Camellia128::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_camellia_set_decrypt_key(ctx.as_mut_ptr(), key.as_ptr());
            transmute(ctx)
        };

        Ok(Camellia128 { context })
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.crypt(dst, src)
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        self.crypt(dst, src)
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut camellia128_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_camellia128_crypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_camellia128_crypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15\x16"[..]);
        let _ = Camellia128::with_encrypt_key(key).unwrap();
        let _ = Camellia128::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Camellia128::with_encrypt_key(&key).unwrap();
        let mut dec = Camellia128::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }

    #[test]
    fn round_trip_invert() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Camellia128::with_encrypt_key(&key).unwrap();
        let mut dec = Camellia128::with_inverted_key(&enc);

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
