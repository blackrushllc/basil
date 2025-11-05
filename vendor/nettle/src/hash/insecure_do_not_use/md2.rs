use nettle_sys::{
    md2_ctx, nettle_hash, nettle_md2, nettle_md2_digest, nettle_md2_init,
    nettle_md2_update, MD2_DIGEST_SIZE,
};
use std::default::Default;
use std::mem::zeroed;

use crate::hash::NettleHash;
use crate::hash::Hash;

/// Message-Digest Algorithm 2 (MD2) defined in RFC 1319.
///
/// # Note
/// MD2 is no longer considered a secure cryptographic hash function. Only use it for legacy
/// applications.
pub struct Md2 {
    context: md2_ctx,
}

impl_write_for_hash!(Md2);

impl Clone for Md2 {
    fn clone(&self) -> Self {
        use std::intrinsics::copy_nonoverlapping;

        unsafe {
            let mut ctx: md2_ctx = zeroed();
            copy_nonoverlapping(&self.context, &mut ctx, 1);

            Md2 { context: ctx }
        }
    }
}

impl Default for Md2 {
    fn default() -> Self {
        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_md2_init(&mut ctx as *mut _);
        }

        Md2 { context: ctx }
    }
}

impl Hash for Md2 {
    fn digest_size(&self) -> usize {
        MD2_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_md2_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_md2_digest(
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

impl NettleHash for Md2 {
    type Context = md2_ctx;

    unsafe fn nettle_hash() -> &'static nettle_hash {
        &nettle_md2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_nothing() {
        let _ = Md2::default();
    }

    #[test]
    fn rfc_1319() {
        let mut ctx = Md2::default();
        let mut digest = vec![0u8; ctx.digest_size()];

        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x83\x50\xe5\xa3\xe2\x4c\x15\x3d\xf2\x27\x5c\x9f\x80\x69\x27\x73"
        );

        ctx.update(b"a");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x32\xec\x01\xec\x4a\x6d\xac\x72\xc0\xab\x96\xfb\x34\xc0\xb5\xd1"
        );

        ctx.update(b"abc");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xda\x85\x3b\x0d\x3f\x88\xd9\x9b\x30\x28\x3a\x69\xe6\xde\xd6\xbb"
        );

        ctx.update(b"message digest");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xab\x4f\x49\x6b\xfb\x2a\x53\x0b\x21\x9f\xf3\x30\x31\xfe\x06\xb0"
        );

        ctx.update(b"abcdefghijklmnopqrstuvwxyz");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x4e\x8d\xdf\xf3\x65\x02\x92\xab\x5a\x41\x08\xc3\xaa\x47\x94\x0b"
        );

        ctx.update(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        );
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xda\x33\xde\xf2\xa4\x2d\xf1\x39\x75\x35\x28\x46\xc3\x03\x38\xcd"
        );

        ctx.update(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xd5\x97\x6f\x79\xd8\x3d\x3a\x0d\xc9\x80\x6c\x3c\x66\xf3\xef\xd8"
        );
    }
}
