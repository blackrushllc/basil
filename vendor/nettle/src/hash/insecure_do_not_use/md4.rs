use nettle_sys::{
    md4_ctx, nettle_hash, nettle_md4, nettle_md4_digest, nettle_md4_init,
    nettle_md4_update, MD4_DIGEST_SIZE,
};
use std::default::Default;
use std::mem::zeroed;

use crate::hash::NettleHash;
use crate::hash::Hash;

/// Message-Digest Algorithm 4 (MD4) defined in RFC 1320.
///
/// # Note
/// MD4 is no longer considered a secure cryptographic hash function. Only use it for legacy
/// applications.
pub struct Md4 {
    context: md4_ctx,
}

impl_write_for_hash!(Md4);

impl Clone for Md4 {
    fn clone(&self) -> Self {
        use std::intrinsics::copy_nonoverlapping;

        unsafe {
            let mut ctx: md4_ctx = zeroed();
            copy_nonoverlapping(&self.context, &mut ctx, 1);

            Md4 { context: ctx }
        }
    }
}

impl Default for Md4 {
    fn default() -> Self {
        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_md4_init(&mut ctx as *mut _);
        }

        Md4 { context: ctx }
    }
}

impl Hash for Md4 {
    fn digest_size(&self) -> usize {
        MD4_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_md4_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_md4_digest(
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

impl NettleHash for Md4 {
    type Context = md4_ctx;

    unsafe fn nettle_hash() -> &'static nettle_hash {
        &nettle_md4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_nothing() {
        let _ = Md4::default();
    }

    #[test]
    fn rfc_1320() {
        let mut ctx = Md4::default();
        let mut digest = vec![0u8; ctx.digest_size()];

        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x31\xd6\xcf\xe0\xd1\x6a\xe9\x31\xb7\x3c\x59\xd7\xe0\xc0\x89\xc0"
        );

        ctx.update(b"a");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xbd\xe5\x2c\xb3\x1d\xe3\x3e\x46\x24\x5e\x05\xfb\xdb\xd6\xfb\x24"
        );

        ctx.update(b"abc");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xa4\x48\x01\x7a\xaf\x21\xd8\x52\x5f\xc1\x0a\xe8\x7a\xa6\x72\x9d"
        );

        ctx.update(b"message digest");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xd9\x13\x0a\x81\x64\x54\x9f\xe8\x18\x87\x48\x06\xe1\xc7\x01\x4b"
        );

        ctx.update(b"abcdefghijklmnopqrstuvwxyz");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xd7\x9e\x1c\x30\x8a\xa5\xbb\xcd\xee\xa8\xed\x63\xdf\x41\x2d\xa9"
        );

        ctx.update(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        );
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x04\x3f\x85\x82\xf2\x41\xdb\x35\x1c\xe6\x27\xe1\x53\xe7\xf0\xe4"
        );

        ctx.update(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xe3\x3b\x4d\xdc\x9c\x38\xf2\x19\x9c\x3e\x7b\x16\x4f\xcc\x05\x36"
        );
    }
}
