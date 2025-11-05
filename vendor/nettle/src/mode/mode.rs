use crate::Result;

/// Block cipher mode of operation.
///
/// Block modes govern how a block cipher processes data spanning multiple blocks.
pub trait Mode {
    /// Block size of the underlying cipher in bytes.
    fn block_size(&self) -> usize;

    /// Encrypt a single block `src` using the initialization vector
    /// `iv` to a ciphertext block `dst`.
    ///
    /// Both `iv` and `dst` are updated.  The buffer `iv`, `dst` and
    /// `src` are expected to be at least as large as the block size
    /// of the underlying cipher.
    fn encrypt(
        &mut self,
        iv: &mut [u8],
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()>;

    /// Decrypt  a   single   ciphertext  block   `src`  using   the
    /// initialization vector `iv` to a plaintext block `dst`.
    ///
    /// Both `iv` and `dst` are updated.  The buffer `iv`, `dst` and
    /// `src` are expected to be at least as large as the block size
    /// of the underlying cipher.
    fn decrypt(
        &mut self,
        iv: &mut [u8],
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<()>;
}
