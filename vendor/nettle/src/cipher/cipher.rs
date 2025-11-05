use std::os::raw::c_void;

use crate::Result;

use super::{
    Aes128, Aes192, Aes256, Camellia128, Camellia192, Camellia256, Serpent,
    Twofish,
};

/// Used internally for cipher modes.
pub struct RawCipherFunctionPointer {
    inner:
        Option<unsafe extern "C" fn(*const c_void, usize, *mut u8, *const u8)>,
}

impl RawCipherFunctionPointer {
    #[doc(hidden)]
    pub fn new<T>(
        f: unsafe extern "C" fn(T, usize, *mut u8, *const u8),
    ) -> Self {
        let ret: extern "C" fn(*const c_void, usize, *mut u8, *const u8) =
            unsafe { ::std::mem::transmute(f as *const c_void) };
        RawCipherFunctionPointer { inner: Some(ret) }
    }

    #[doc(hidden)]
    pub fn ptr(
        &self,
    ) -> Option<unsafe extern "C" fn(*const c_void, usize, *mut u8, *const u8)>
    {
        self.inner
    }
}

/// Symmetric block or stream cipher.
///
/// Symmetric cipher encrypt and decrypt data using the same key.
pub trait Cipher: Sized {
    /// Block size in bytes.
    const BLOCK_SIZE: usize;
    /// Maximal key size in bytes.
    const KEY_SIZE: usize;

    /// Creates a new cipher instance for decryption.
    ///
    /// The `key` parameter must have size `KEY_SIZE`.
    fn with_decrypt_key(key: &[u8]) -> Result<Self>;

    /// Creates a new cipher instance for encryption.
    ///
    /// The `key` parameter must have size `KEY_SIZE`.
    fn with_encrypt_key(key: &[u8]) -> Result<Self>;

    /// Decrypt `src` into `dst`.
    ///
    /// Both must have the same length. That length must be a multiple
    /// of `BLOCK_SIZE`. Blocks are processed in ECB mode.
    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]);

    /// Encrypt `src` into `dst`.
    ///
    /// Both must have the same length. That length must be a multiple
    /// of `BLOCK_SIZE`. Blocks are processed in ECB mode.
    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]);

    /// Returns a pointer to the C context struct of the cipher instance.
    ///
    /// Used internally by block modi.
    fn context(&mut self) -> *mut c_void;

    /// Pointer to the *_decrypt C function.
    ///
    /// Used internally for block modi.
    fn raw_decrypt_function() -> RawCipherFunctionPointer;

    /// Pointer to the *_encrypt C function.
    ///
    /// Used internally for block modi.
    fn raw_encrypt_function() -> RawCipherFunctionPointer;
}

/// Marker trait for ciphers with 16 byte block size.
pub trait BlockSizeIs16: Cipher {}

impl BlockSizeIs16 for Aes128 {}
impl BlockSizeIs16 for Aes192 {}
impl BlockSizeIs16 for Aes256 {}
impl BlockSizeIs16 for Twofish {}
impl BlockSizeIs16 for Camellia128 {}
impl BlockSizeIs16 for Camellia192 {}
impl BlockSizeIs16 for Camellia256 {}
impl BlockSizeIs16 for Serpent {}
