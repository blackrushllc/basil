use nettle_sys::{
    md5_ctx, nettle_hash, nettle_md5, nettle_md5_digest, nettle_md5_init,
    nettle_md5_update, MD5_DIGEST_SIZE,
};
use std::default::Default;
use std::mem::zeroed;

use crate::hash::NettleHash;
use crate::hash::Hash;

/// Message-Digest Algorithm 5 (MD5) defined in RFC 1321.
///
/// # Note
/// MD5 is no longer considered a secure cryptographic hash function. Only use it for legacy
/// applications.
pub struct Md5 {
    context: md5_ctx,
}

impl_write_for_hash!(Md5);

impl Clone for Md5 {
    fn clone(&self) -> Self {
        use std::intrinsics::copy_nonoverlapping;

        unsafe {
            let mut ctx: md5_ctx = zeroed();
            copy_nonoverlapping(&self.context, &mut ctx, 1);

            Md5 { context: ctx }
        }
    }
}

impl Default for Md5 {
    fn default() -> Self {
        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_md5_init(&mut ctx as *mut _);
        }

        Md5 { context: ctx }
    }
}

impl Hash for Md5 {
    fn digest_size(&self) -> usize {
        MD5_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_md5_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_md5_digest(
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

impl NettleHash for Md5 {
    type Context = md5_ctx;

    unsafe fn nettle_hash() -> &'static nettle_hash {
        &nettle_md5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_nothing() {
        let _ = Md5::default();
    }

    #[test]
    fn rfc_1321() {
        let mut ctx = Md5::default();
        let mut digest = vec![0u8; ctx.digest_size()];

        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xd4\x1d\x8c\xd9\x8f\x00\xb2\x04\xe9\x80\x09\x98\xec\xf8\x42\x7e"
        );

        ctx.update(b"a");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x0c\xc1\x75\xb9\xc0\xf1\xb6\xa8\x31\xc3\x99\xe2\x69\x77\x26\x61"
        );

        ctx.update(b"abc");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x90\x01\x50\x98\x3c\xd2\x4f\xb0\xd6\x96\x3f\x7d\x28\xe1\x7f\x72"
        );

        ctx.update(b"message digest");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xf9\x6b\x69\x7d\x7c\xb7\x93\x8d\x52\x5a\x2f\x31\xaa\xf1\x61\xd0"
        );

        ctx.update(b"abcdefghijklmnopqrstuvwxyz");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xc3\xfc\xd3\xd7\x61\x92\xe4\x00\x7d\xfb\x49\x6c\xca\x67\xe1\x3b"
        );

        ctx.update(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        );
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\xd1\x74\xab\x98\xd2\x77\xd9\xf5\xa5\x61\x1c\x2c\x9f\x41\x9d\x9f"
        );

        ctx.update(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890");
        ctx.digest(&mut digest);
        assert_eq!(
            digest,
            b"\x57\xed\xf4\xa2\x2b\xe3\xc9\x55\xac\x49\xda\x2e\x21\x07\xb6\x7a"
        );
    }
}
