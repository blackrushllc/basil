//! Cryptographic random number generators (CRNG).

use libc::size_t;
use std::os::raw::c_void;

/// A cryptographic random number generator.
pub trait Random {
    /// Returns a pointer to the opaque CRNG state.
    unsafe fn context(&mut self) -> *mut c_void;
    /// Fills the buffer `dst` with `length` random bytes, advancing the CRNG state `ctx`.
    unsafe extern "C" fn random_impl(
        ctx: *mut c_void, length: size_t, dst: *mut u8,
    );

    /// Fills the buffer `random` with random bytes.
    fn random(&mut self, buf: &mut [u8]) {
        unsafe {
            Self::random_impl(self.context(), buf.len(), buf.as_mut_ptr());
        }
    }
}
