use nettle_sys::{
    nettle_hash, nettle_ripemd160, nettle_ripemd160_digest,
    nettle_ripemd160_init, nettle_ripemd160_update, ripemd160_ctx,
    RIPEMD160_DIGEST_SIZE,
};
use std::default::Default;
use std::mem::zeroed;

use crate::hash::NettleHash;
use crate::hash::Hash;

/// RIPEMD-160 defined in ISO/IEC 10118-3:1998.
///
/// # Note
/// RIPEMD-160 is no longer considered a secure cryptographic hash function. Only use it for legacy
/// applications.
pub struct Ripemd160 {
    context: ripemd160_ctx,
}

impl_write_for_hash!(Ripemd160);

impl Clone for Ripemd160 {
    fn clone(&self) -> Self {
        use std::intrinsics::copy_nonoverlapping;

        unsafe {
            let mut ctx: ripemd160_ctx = zeroed();
            copy_nonoverlapping(&self.context, &mut ctx, 1);

            Ripemd160 { context: ctx }
        }
    }
}

impl Default for Ripemd160 {
    fn default() -> Self {
        let mut ctx = unsafe { zeroed() };

        unsafe {
            nettle_ripemd160_init(&mut ctx as *mut _);
        }

        Ripemd160 { context: ctx }
    }
}

impl Hash for Ripemd160 {
    fn digest_size(&self) -> usize {
        RIPEMD160_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_ripemd160_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) {
        unsafe {
            nettle_ripemd160_digest(
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

impl NettleHash for Ripemd160 {
    type Context = ripemd160_ctx;

    unsafe fn nettle_hash() -> &'static nettle_hash {
        &nettle_ripemd160
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_nothing() {
        let _ = Ripemd160::default();
    }

    #[test]
    fn bosselaers_test_vectors() {
        let mut ctx = Ripemd160::default();
        let mut digest = vec![0u8; ctx.digest_size()];

        ctx.digest(&mut digest);
        assert_eq!(digest, b"\x9c\x11\x85\xa5\xc5\xe9\xfc\x54\x61\x28\x08\x97\x7e\xe8\xf5\x48\xb2\x25\x8d\x31");

        ctx.update(b"a");
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\x0b\xdc\x9d\x2d\x25\x6b\x3e\xe9\xda\xae\x34\x7b\xe6\xf4\xdc\x83\x5a\x46\x7f\xfe");

        ctx.update(b"abc");
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\x8e\xb2\x08\xf7\xe0\x5d\x98\x7a\x9b\x04\x4a\x8e\x98\xc6\xb0\x87\xf1\x5a\x0b\xfc");

        ctx.update(b"message digest");
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\x5d\x06\x89\xef\x49\xd2\xfa\xe5\x72\xb8\x81\xb1\x23\xa8\x5f\xfa\x21\x59\x5f\x36");

        ctx.update(b"abcdefghijklmnopqrstuvwxyz");
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\xf7\x1c\x27\x10\x9c\x69\x2c\x1b\x56\xbb\xdc\xeb\x5b\x9d\x28\x65\xb3\x70\x8d\xbc");

        ctx.update(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        );
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\xb0\xe2\x0b\x6e\x31\x16\x64\x02\x86\xed\x3a\x87\xa5\x71\x30\x79\xb2\x1f\x51\x89");

        ctx.update(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890");
        ctx.digest(&mut digest);
        assert_eq!(digest, b"\x9b\x75\x2e\x45\x57\x3d\x4b\x39\xf4\xdb\xd3\x32\x3c\xab\x82\xbf\x63\x32\x6b\xfb");
    }
}
