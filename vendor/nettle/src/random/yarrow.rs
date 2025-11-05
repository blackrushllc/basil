use libc::size_t;
use nettle_sys::{
    nettle_yarrow256_init, nettle_yarrow256_is_seeded, nettle_yarrow256_random,
    nettle_yarrow256_seed, yarrow256_ctx,
};
use getrandom::getrandom;
use std::mem::{transmute, zeroed};
use std::os::raw::c_void;
use std::ptr;

use super::Random;

/// Yarrow is a secure CRNG developed by Kelsey et.al.
///
/// Default instances are seeded using `OsRng`.
pub struct Yarrow {
    context: yarrow256_ctx,
}

impl Yarrow {
    /// Create a new CRNG instance for `seed`.
    pub fn from_seed(seed: &[u8]) -> Yarrow {
        unsafe {
            let mut ctx = zeroed();

            nettle_yarrow256_init(&mut ctx as *mut _, 0, ptr::null_mut());
            nettle_yarrow256_seed(
                &mut ctx as *mut _,
                seed.len(),
                seed.as_ptr(),
            );

            Yarrow { context: ctx }
        }
    }
}

impl Random for Yarrow {
    unsafe fn context(&mut self) -> *mut c_void {
        transmute(&mut self.context)
    }

    unsafe extern "C" fn random_impl(
        ctx: *mut c_void, length: size_t, dst: *mut u8,
    ) {
        assert_eq!(nettle_yarrow256_is_seeded(ctx as *mut _), 1);
        nettle_yarrow256_random(ctx as *mut _, length, dst);
    }
}

impl Default for Yarrow {
    fn default() -> Self {
        let mut seed = vec![0u8; 64];
        if let Err(e) = getrandom(&mut seed) {
            panic!("Failed to initialize random generator: {}", e);
        }
        Yarrow::from_seed(&seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_seeded() {
        let mut rng1 = Yarrow::default();
        let mut rng2 = Yarrow::default();
        let mut buf1 = vec![0u8; 100];
        let mut buf2 = vec![0u8; 100];
        let zero = vec![0u8; 100];

        rng1.random(&mut buf1);
        rng2.random(&mut buf2);

        assert_ne!(buf1, buf2);
        assert_ne!(buf1, zero);
        assert_ne!(buf1, zero);
    }

    #[test]
    fn fixed_seed() {
        let seed =
            &b"aaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbcccccccccccccccccccdddd"[..];
        let mut rng1 = Yarrow::from_seed(seed);
        let mut rng2 = Yarrow::from_seed(seed);
        let mut buf1 = vec![0u8; 100];
        let mut buf2 = vec![0u8; 100];
        let zero = vec![0u8; 100];

        rng1.random(&mut buf1);
        rng2.random(&mut buf2);

        assert_eq!(buf1, buf2);
        assert_ne!(buf1, zero);
    }

    #[test]
    fn random_inteface() {
        let seed =
            &b"aaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbcccccccccccccccccccdddd"[..];
        let mut rng1 = Yarrow::from_seed(seed);
        let mut rng2 = Yarrow::from_seed(seed);
        let mut buf1 = vec![0u8; 100];
        let mut buf2 = vec![0u8; 100];
        let zero = vec![0u8; 100];

        rng1.random(&mut buf1);

        unsafe {
            let ctx = rng2.context();
            <Yarrow as Random>::random_impl(ctx, buf2.len(), buf2.as_mut_ptr());
        }

        assert_eq!(buf1, buf2);
        assert_ne!(buf1, zero);
    }
}
