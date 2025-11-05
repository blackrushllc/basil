use nettle_sys::nettle_pbkdf2;

use crate::hash::NettleHash;
use crate::mac::Hmac;
use crate::mac::Mac;

/// Password-based Key derivation function 2 (RFC 6070).
///
/// Derives `key` from a secret `password` and a public `salt`.
pub fn pbkdf2<H: NettleHash>(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    key: &mut [u8],
) {
    let mut hmac = Hmac::<H>::with_key(password);

    unsafe {
        nettle_pbkdf2(
            (&mut hmac as *mut Hmac<H>) as *mut _,
            Some(Hmac::<H>::nettle_update),
            Some(Hmac::<H>::nettle_digest),
            hmac.mac_size(),
            iterations,
            salt.len(),
            salt.as_ptr(),
            key.len(),
            key.as_mut_ptr(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::insecure_do_not_use::Sha1;

    #[test]
    fn rfc_6070_case_1() {
        let password = &b"password"[..];
        let salt = &b"salt"[..];
        let mut key = vec![0u8; 20];

        pbkdf2::<Sha1>(password, salt, 1, &mut key);
        assert_eq!(&key[..], &b"\x0c\x60\xc8\x0f\x96\x1f\x0e\x71\xf3\xa9\xb5\x24\xaf\x60\x12\x06\x2f\xe0\x37\xa6"[..]);
    }

    #[test]
    fn rfc_6070_case_2() {
        let password = &b"password"[..];
        let salt = &b"salt"[..];
        let mut key = vec![0u8; 20];

        pbkdf2::<Sha1>(password, salt, 2, &mut key);
        assert_eq!(&key[..], &b"\xea\x6c\x01\x4d\xc7\x2d\x6f\x8c\xcd\x1e\xd9\x2a\xce\x1d\x41\xf0\xd8\xde\x89\x57"[..]);
    }

    #[test]
    fn rfc_6070_case_3() {
        let password = &b"password"[..];
        let salt = &b"salt"[..];
        let mut key = vec![0u8; 20];

        pbkdf2::<Sha1>(password, salt, 4096, &mut key);
        assert_eq!(&key[..], &b"\x4b\x00\x79\x01\xb7\x65\x48\x9a\xbe\xad\x49\xd9\x26\xf7\x21\xd0\x65\xa4\x29\xc1"[..]);
    }

    #[test]
    fn rfc_6070_case_4() {
        let password = &b"password"[..];
        let salt = &b"salt"[..];
        let mut key = vec![0u8; 20];

        pbkdf2::<Sha1>(password, salt, 16777216, &mut key);
        assert_eq!(&key[..], &b"\xee\xfe\x3d\x61\xcd\x4d\xa4\xe4\xe9\x94\x5b\x3d\x6b\xa2\x15\x8c\x26\x34\xe9\x84"[..]);
    }

    #[test]
    fn rfc_6070_case_5() {
        let password = &b"passwordPASSWORDpassword"[..];
        let salt = &b"saltSALTsaltSALTsaltSALTsaltSALTsalt"[..];
        let mut key = vec![0u8; 25];

        pbkdf2::<Sha1>(password, salt, 4096, &mut key);
        assert_eq!(&key[..], &b"\x3d\x2e\xec\x4f\xe4\x1c\x84\x9b\x80\xc8\xd8\x36\x62\xc0\xe4\x4a\x8b\x29\x1a\x96\x4c\xf2\xf0\x70\x38"[..]);
    }

    #[test]
    fn rfc_6070_case_6() {
        let password = &b"pass\x00word"[..];
        let salt = &b"sa\x00lt"[..];
        let mut key = vec![0u8; 16];

        pbkdf2::<Sha1>(password, salt, 4096, &mut key);
        assert_eq!(&key[..], &b"\x56\xfa\x6a\xa7\x55\x48\x09\x9d\xcc\x37\xd7\xf0\x34\x25\xe0\xc3"[..]);
    }
}
