//! Authenticated encryption interface definitions.

use crate::cipher::Cipher;

/// A AEAD mode of operation.
pub trait Aead {
    /// Adds associated data `ad`.
    ///
    /// Can be invoked multiple times *prior to invoking*
    /// [`Aead::encrypt`] or [`Aead::decrypt`] to provide the
    /// additional authenticated data in a streaming fashion.
    fn update(&mut self, ad: &[u8]);

    /// Encrypts one block `src` to `dst`.
    fn encrypt(&mut self, dst: &mut [u8], src: &[u8]);
    /// Decrypts one block `src` to `dst`.
    fn decrypt(&mut self, dst: &mut [u8], src: &[u8]);

    /// Produce the digest.
    fn digest(&mut self, digest: &mut [u8]);

    /// Length of the digest in bytes.
    fn digest_size(&self) -> usize;
}

/// Internal interface to Nettle's AEAD modes.
///
/// Some of Nettle's AEAD modes require additional authenticated data
/// to be updated in multiples of the ciphers block size (except for
/// the last chunk), and will either silently mis-compute the digest
/// (EAX, OCB), or abort(2) (GCM).
///
/// For these modes, we use [`AdStream`] to stream the AAD, and trait
/// [`AeadInternal`] as raw interface.
pub trait AeadInternal {
    /// Adds associated data `ad`.
    ///
    /// Note: can be invoked multiple times to stream the additional
    /// authenticated data, but the length of all but the last chunk
    /// **MUST BE** of a multiple of the cipher's block size.  Failure
    /// to adhere to that may lead to Nettle aborting or silently
    /// mis-computing the digest.
    ///
    /// Note: **MUST NOT BE** invoked after invoking
    /// [`AeadInternal::_encrypt`] or [`AeadInternal::_decrypt`].
    /// Failure to adhere to that may lead to Nettle aborting or
    /// silently mis-computing.
    fn update_internal(&mut self, ad: &[u8]);

    /// Encrypts one block `src` to `dst`.
    fn encrypt_internal(&mut self, dst: &mut [u8], src: &[u8]);
    /// Decrypts one block `src` to `dst`.
    fn decrypt_internal(&mut self, dst: &mut [u8], src: &[u8]);

    /// Produce the digest.
    fn digest_internal(&mut self, digest: &mut [u8]);

    /// Length of the digest in bytes.
    fn digest_size_internal(&self) -> usize;
}

/// Streaming AAD support for EAX, OCB, and GCM.
///
/// Some of Nettle's AEAD modes require additional authenticated data
/// to be updated in multiples of the ciphers block size (except for
/// the last chunk), and will either silently mis-compute the digest
/// (EAX, OCB), or abort(2) (GCM).
///
/// For these modes, we use [`AdStream`] to stream the AAD, and trait
/// [`AeadInternal`] as raw interface.
pub struct AdStream<C: Cipher> {
    buffer: Vec<u8>,
    finalized: bool,
    cipher: std::marker::PhantomData<*const C>,
}

impl<C: Cipher> Default for AdStream<C> {
    fn default() -> Self {
        AdStream {
            buffer: Vec::with_capacity(C::BLOCK_SIZE),
            finalized: false,
            cipher: std::marker::PhantomData,
        }
    }
}

impl<C: Cipher> AdStream<C> {
    /// Stream the given AAD.
    ///
    /// Note: additional data provided after invoking
    /// [`AdStream::finalize`] is **discarded** and an error is
    /// emitted to stderr.  This will be an error in future versions.
    pub fn update(&mut self, aad: &[u8], context: &mut dyn AeadInternal) {
        self.stream(aad, false, context);
    }

    /// Finalize the AAD streaming.
    ///
    /// After calling this function, no further AAD may be streamed.
    /// This function is idempotent.
    pub fn finalize(&mut self, context: &mut dyn AeadInternal) {
        if ! self.finalized {
            self.stream(&[], true, context);
        }
    }

    /// Stream the given AAD and finalize streaming.
    ///
    /// After calling this function with `last = true`, no further AAD
    /// may be streamed.
    ///
    /// Note: additional data provided after invoking
    /// [`AdStream::finalize`] or this function with `last = true` is
    /// **discarded** and an error is emitted to stderr.  This will be
    /// an error in future versions.
    fn stream(&mut self, mut data: &[u8], last: bool,
              context: &mut dyn AeadInternal)
    {
        debug_assert!(self.buffer.len() < C::BLOCK_SIZE);

        // XXX: This should return an error or be made impossible
        // using type states.
        if self.finalized {
            eprintln!("nettle::aead: AAD supplied after cipher use");
            return;
        }

        if self.buffer.len() + data.len() < C::BLOCK_SIZE {
            // No full block.  Buffer.
            self.buffer.extend_from_slice(data);
        } else if self.buffer.is_empty() {
            // No data in buffer, update digest with all whole blocks
            // from `data`.
            let n_chunks = data.len() / C::BLOCK_SIZE;
            let n_chunks_len = n_chunks * C::BLOCK_SIZE;
            context.update_internal(&data[..n_chunks_len]);

            // Finally, buffer the rest, if any.
            self.buffer.extend_from_slice(&data[n_chunks_len..]);
        } else {
            // Some buffered data.  Fill up to the BLOCK_SIZE and
            // update digest from that.
            let missing = (C::BLOCK_SIZE - self.buffer.len()).min(data.len());
            self.buffer.extend_from_slice(&data[..missing]);
            debug_assert_eq!(self.buffer.len(), C::BLOCK_SIZE);
            context.update_internal(&self.buffer[..]);
            self.buffer.clear();
            data = &data[missing..];

            // Update digest with all whole blocks from `data`.
            let n_chunks = data.len() / C::BLOCK_SIZE;
            let n_chunks_len = n_chunks * C::BLOCK_SIZE;
            context.update_internal(&data[..n_chunks_len]);

            // Finally, buffer the rest, if any.
            self.buffer.extend_from_slice(&data[n_chunks_len..]);
        }

        if last {
            if ! self.buffer.is_empty() {
                context.update_internal(&self.buffer[..]);
                self.buffer.clear();
            }
            self.finalized = true;
        }

        debug_assert!(self.buffer.len() < C::BLOCK_SIZE);
    }
}
