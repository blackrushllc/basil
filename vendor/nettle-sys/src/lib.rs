#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::zeroed;

    #[test]
    fn sha1_nothing() {
        unsafe {
            let mut digest = vec![0u8; SHA1_DIGEST_SIZE as usize];
            let mut ctx = zeroed::<sha1_ctx>();

            nettle_sha1_init(&mut ctx as *mut _);
            nettle_sha1_digest(&mut ctx as *mut _, SHA1_DIGEST_SIZE as usize, digest.as_mut_ptr());
            assert_eq!(digest, b"\xda\x39\xa3\xee\x5e\x6b\x4b\x0d\x32\x55\xbf\xef\x95\x60\x18\x90\xaf\xd8\x07\x09");
        }
    }
}
