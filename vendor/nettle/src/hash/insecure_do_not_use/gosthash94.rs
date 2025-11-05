use nettle_sys::{
    gosthash94_ctx, nettle_gosthash94, nettle_gosthash94_digest,
    nettle_gosthash94_init, nettle_gosthash94_update, nettle_hash,
    GOSTHASH94_DIGEST_SIZE,
};
use std::default::Default;
use std::mem::zeroed;

use crate::hash::NettleHash;
use crate::hash::Hash;

/// GOST R 34.11-94 (GOSTHASH94) defined in RFC 5831.
///
/// # Note
/// GOSTHASH94 is no longer considered a secure cryptographic hash function. Only use it for legacy
/// applications.
pub struct GostHash94 {
    context: gosthash94_ctx,
}

impl_write_for_hash!(GostHash94);

impl Clone for GostHash94 {
    fn clone(&self) -> Self {
        use std::intrinsics::copy_nonoverlapping;

        unsafe {
            let mut ctx: gosthash94_ctx = zeroed();
            copy_nonoverlapping(&self.context, &mut ctx, 1);

            GostHash94 { context: ctx }
        }
    }
}

impl Default for GostHash94 {
    fn default() -> Self {
        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_gosthash94_init(&mut ctx as *mut _);
        }

        GostHash94 { context: ctx }
    }
}

impl Hash for GostHash94 {
    fn digest_size(&self) -> usize {
        GOSTHASH94_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_gosthash94_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_gosthash94_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }
    }

    fn box_clone(&self) -> Box<dyn Hash> {
        Box::new(self.clone())
    }
}

impl NettleHash for GostHash94 {
    type Context = gosthash94_ctx;

    unsafe fn nettle_hash() -> &'static nettle_hash {
        &nettle_gosthash94
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_nothing() {
        let _ = GostHash94::default();
    }
}
