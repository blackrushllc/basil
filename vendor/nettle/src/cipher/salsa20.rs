use nettle_sys::{
    nettle_salsa20_128_set_key, nettle_salsa20_256_set_key,
    nettle_salsa20_crypt, nettle_salsa20_set_nonce, salsa20_ctx,
};
use std::cmp::min;
use std::mem::{MaybeUninit, transmute};

use crate::{Error, Result};

#[allow(non_camel_case_types)]
/// 128 bit variant of D.J. Bernstein's Salsa20 block cipher.
pub struct Salsa20_128 {
    context: Box<salsa20_ctx>,
}

impl_zeroing_drop_for!(Salsa20_128);

impl Salsa20_128 {
    /// Salsa20 block size in bytes.
    pub const BLOCK_SIZE: usize = ::nettle_sys::SALSA20_BLOCK_SIZE as usize;

    /// Salsa20 key size in bytes.
    pub const KEY_SIZE: usize = ::nettle_sys::SALSA20_128_KEY_SIZE as usize;

    /// Salsa20 nonce size in bytes.
    pub const NONCE_SIZE: usize = ::nettle_sys::SALSA20_NONCE_SIZE as usize;

    /// Create a new instance with `key`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != Salsa20_128::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }
        if nonce.len() != Salsa20_128::NONCE_SIZE {
            return Err(Error::InvalidArgument { argument_name: "nonce" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_salsa20_128_set_key(ctx.as_mut_ptr(), key.as_ptr());
            nettle_salsa20_set_nonce(ctx.as_mut_ptr(), nonce.as_ptr());
            transmute(ctx)
        };

        Ok(Salsa20_128 { context })
    }

    /// Encrypt/decrypt data from `src` to `dst`.
    pub fn crypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_salsa20_crypt(
                self.context.as_mut() as *mut _,
                min(src.len(), dst.len()),
                dst.as_mut_ptr(),
                src.as_ptr(),
            )
        };
    }
}

#[allow(non_camel_case_types)]
/// 256 bit variant of D.J. Bernstein's Salsa20 block cipher.
pub struct Salsa20_256 {
    context: Box<salsa20_ctx>,
}

impl_zeroing_drop_for!(Salsa20_256);

impl Salsa20_256 {
    /// Salsa20 block size in bytes.
    pub const BLOCK_SIZE: usize = ::nettle_sys::SALSA20_BLOCK_SIZE as usize;

    /// Salsa20 key size in bytes.
    pub const KEY_SIZE: usize = ::nettle_sys::SALSA20_256_KEY_SIZE as usize;

    /// Salsa20 nonce size in bytes.
    pub const NONCE_SIZE: usize = ::nettle_sys::SALSA20_NONCE_SIZE as usize;

    /// Create a new instance with `key`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != Salsa20_256::KEY_SIZE {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }
        if nonce.len() != Salsa20_256::NONCE_SIZE {
            return Err(Error::InvalidArgument { argument_name: "nonce" });
        }

        let context = unsafe {
            let mut ctx = Box::new(MaybeUninit::uninit());
            nettle_salsa20_256_set_key(ctx.as_mut_ptr(), key.as_ptr());
            nettle_salsa20_set_nonce(ctx.as_mut_ptr(), nonce.as_ptr());
            transmute(ctx)
        };

        Ok(Salsa20_256 { context })
    }

    /// Encrypt/decrypt data from `src` to `dst`.
    pub fn crypt(&mut self, dst: &mut [u8], src: &[u8]) {
        unsafe {
            nettle_salsa20_crypt(
                self.context.as_mut() as *mut _,
                min(dst.len(), src.len()),
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
    fn salsa128_set_key_and_nonce() {
        let key = vec![0; 16];
        let nonce = vec![1; 8];

        let _ = Salsa20_128::with_key_and_nonce(&key, &nonce).unwrap();
    }

    #[test]
    fn salsa128_round_trip() {
        let key = vec![0; 16];
        let nonce = vec![1; 8];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Salsa20_128::with_key_and_nonce(&key, &nonce).unwrap();
        let mut dec = Salsa20_128::with_key_and_nonce(&key, &nonce).unwrap();

        enc.crypt(&mut cipher, &input);
        dec.crypt(&mut output, &cipher);

        assert_eq!(output, input);
    }

    #[test]
    fn salsa256_set_key_and_nonce() {
        let key = vec![0; 32];
        let nonce = vec![1; 8];

        let _ = Salsa20_256::with_key_and_nonce(&key, &nonce).unwrap();
    }

    #[test]
    fn salsa256_round_trip() {
        let key = vec![0; 32];
        let nonce = vec![1; 8];
        let input = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16,
        ];
        let mut cipher = vec![0; 16];
        let mut output = vec![0; 16];

        let mut enc = Salsa20_256::with_key_and_nonce(&key, &nonce).unwrap();
        let mut dec = Salsa20_256::with_key_and_nonce(&key, &nonce).unwrap();

        enc.crypt(&mut cipher, &input);
        dec.crypt(&mut output, &cipher);

        assert_eq!(output, input);
    }
}
