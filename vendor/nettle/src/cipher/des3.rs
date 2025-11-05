use nettle_sys::{
    des3_ctx, nettle_des3_decrypt, nettle_des3_encrypt, nettle_des3_set_key,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};
use std::os::raw::c_void;

use crate::cipher::RawCipherFunctionPointer;
use crate::{cipher::Cipher, Error, Result};

/// Triple DES in EDE configuration defined in RFC 1851.
pub struct Des3 {
    context: Box<des3_ctx>,
}

impl_zeroing_drop_for!(Des3);

impl Des3 {
    /// Creates a new instance with `key` that can be used for both encryption and decryption.
    pub fn with_key(key: &[u8]) -> Result<Self> {
        if key.len() != Des3::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_des3_set_key(ctx.as_mut_ptr(), key.as_ptr());
            transmute(ctx)
        };

        Ok(Des3 { context })
    }
}

impl Cipher for Des3 {
    const BLOCK_SIZE: usize = ::nettle_sys::DES3_BLOCK_SIZE as usize;
    const KEY_SIZE: usize = ::nettle_sys::DES3_KEY_SIZE as usize;

    fn with_encrypt_key(key: &[u8]) -> Result<Des3> {
        Des3::with_key(key)
    }

    fn with_decrypt_key(key: &[u8]) -> Result<Des3> {
        Des3::with_key(key)
    }

    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_des3_encrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_des3_decrypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }

    fn context(&mut self) -> *mut c_void {
        (self.context.as_mut() as *mut des3_ctx) as *mut c_void
    }

    fn raw_encrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_des3_encrypt)
    }

    fn raw_decrypt_function() -> RawCipherFunctionPointer {
        RawCipherFunctionPointer::new(nettle_des3_decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key() {
        let key = &(b"\x01\x02\x03\x04\x05\x06\x07\x08\x01\x02\x03\x04\x05\x06\x07\x08\x01\x02\x03\x04\x05\x06\x07\x08"[..]);
        let _ = Des3::with_encrypt_key(key).unwrap();
        let _ = Des3::with_decrypt_key(key).unwrap();
    }

    #[test]
    fn round_trip() {
        let key = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x02, 0x03,
            0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            0x07, 0x08,
        ];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Des3::with_encrypt_key(&key).unwrap();
        let mut dec = Des3::with_decrypt_key(&key).unwrap();

        enc.encrypt(&mut cipher, &input);
        dec.decrypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
