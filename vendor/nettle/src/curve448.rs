//! Elliptic curve Diffie-Hellman using Mike Hamburg's Curve448.

use nettle_sys::{nettle_curve448_mul, nettle_curve448_mul_g};

use crate::{Error, random::Random, Result};

/// Whether or not this algorithm is supported by the version of
/// Nettle we are linked against at build time.
///
/// If this is false, then [`mul_g`], and [`mul`] will fail on all
/// inputs.
pub const IS_SUPPORTED: bool = nettle_sys::config::HAVE_CV448;

/// Size of the public and private keys in bytes.
pub const CURVE448_SIZE: usize = ::nettle_sys::CURVE448_SIZE as usize;

/// Generate a new X448 private key.
pub fn private_key<R>(rng: &mut R) -> Box<[u8]>
where
    R: Random,
{
    let mut ret = vec![0u8; CURVE448_SIZE].into_boxed_slice();

    // Section 5 of RFC7748:
    //
    // > [..] for X448, set the two least significant bits of the first
    // > byte to 0, and the most significant bit of the last byte to 1.
    // > This means that the resulting integer is of the form 2^447 plus
    // > four times a value between 0 and 2^445 - 1 (inclusive).
    rng.random(&mut ret[..]);
    ret[0] &= 0b1111_1100;
    ret[CURVE448_SIZE - 1] |= 0b1000_0000;

    ret
}

/// Derive DH public key.
///
/// Computes the public key `q` for a given secret `n`. Returns an
/// error if `q` or `n` are not `CURVE448_SIZE` bytes long, or if
/// Curve448 is not supported by Nettle (see [`IS_SUPPORTED`]).
pub fn mul_g(q: &mut [u8], n: &[u8]) -> Result<()> {
    if ! nettle_sys::config::HAVE_CV448 {
        return Err(Error::KeyGenerationFailed);
    }

    if q.len() != CURVE448_SIZE {
        return Err(Error::InvalidArgument { argument_name: "q" });
    }
    if n.len() != CURVE448_SIZE {
        return Err(Error::InvalidArgument { argument_name: "n" });
    }

    unsafe {
        nettle_curve448_mul_g(q.as_mut_ptr(), n.as_ptr());
    }

    Ok(())
}

/// Derive DH shared secret.
///
/// Computes the shared secret `q` for our private key `n` and the
/// other parties public key `p`.  Returns an error if `q`, `n` or `p`
/// are not `CURVE448_SIZE` bytes long, or if Curve448 is not
/// supported by Nettle (see [`IS_SUPPORTED`]).
pub fn mul(q: &mut [u8], n: &[u8], p: &[u8]) -> Result<()> {
    if ! nettle_sys::config::HAVE_CV448 {
        return Err(Error::EncryptionFailed);
    }

    if q.len() != CURVE448_SIZE {
        return Err(Error::InvalidArgument { argument_name: "q" });
    }
    if n.len() != CURVE448_SIZE {
        return Err(Error::InvalidArgument { argument_name: "n" });
    }
    if p.len() != CURVE448_SIZE {
        return Err(Error::InvalidArgument { argument_name: "p" });
    }

    unsafe {
        nettle_curve448_mul(q.as_mut_ptr(), n.as_ptr(), p.as_ptr());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;

    #[test]
    fn rfc7748() {
        if ! nettle_sys::config::HAVE_CV448 {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        let alice_priv =
            &b"\x9a\x8f\x49\x25\xd1\x51\x9f\x57\x75\xcf\x46\xb0\x4b\x58\
               \x00\xd4\xee\x9e\xe8\xba\xe8\xbc\x55\x65\xd4\x98\xc2\x8d\
               \xd9\xc9\xba\xf5\x74\xa9\x41\x97\x44\x89\x73\x91\x00\x63\
               \x82\xa6\xf1\x27\xab\x1d\x9a\xc2\xd8\xc0\xa5\x98\x72\x6b"[..];
        let alice_pub =
            &b"\x9b\x08\xf7\xcc\x31\xb7\xe3\xe6\x7d\x22\xd5\xae\xa1\x21\
               \x07\x4a\x27\x3b\xd2\xb8\x3d\xe0\x9c\x63\xfa\xa7\x3d\x2c\
               \x22\xc5\xd9\xbb\xc8\x36\x64\x72\x41\xd9\x53\xd4\x0c\x5b\
               \x12\xda\x88\x12\x0d\x53\x17\x7f\x80\xe5\x32\xc4\x1f\xa0"[..];
        let bob_priv =
            &b"\x1c\x30\x6a\x7a\xc2\xa0\xe2\xe0\x99\x0b\x29\x44\x70\xcb\
               \xa3\x39\xe6\x45\x37\x72\xb0\x75\x81\x1d\x8f\xad\x0d\x1d\
               \x69\x27\xc1\x20\xbb\x5e\xe8\x97\x2b\x0d\x3e\x21\x37\x4c\
               \x9c\x92\x1b\x09\xd1\xb0\x36\x6f\x10\xb6\x51\x73\x99\x2d"[..];
        let bob_pub =
            &b"\x3e\xb7\xa8\x29\xb0\xcd\x20\xf5\xbc\xfc\x0b\x59\x9b\x6f\
               \xec\xcf\x6d\xa4\x62\x71\x07\xbd\xb0\xd4\xf3\x45\xb4\x30\
               \x27\xd8\xb9\x72\xfc\x3e\x34\xfb\x42\x32\xa1\x3c\xa7\x06\
               \xdc\xb5\x7a\xec\x3d\xae\x07\xbd\xc1\xc6\x7b\xf3\x36\x09"[..];
        let shared =
            &b"\x07\xff\xf4\x18\x1a\xc6\xcc\x95\xec\x1c\x16\xa9\x4a\x0f\
               \x74\xd1\x2d\xa2\x32\xce\x40\xa7\x75\x52\x28\x1d\x28\x2b\
               \xb6\x0c\x0b\x56\xfd\x24\x64\xc3\x35\x54\x39\x36\x52\x1c\
               \x24\x40\x30\x85\xd5\x9a\x44\x9a\x50\x37\x51\x4a\x87\x9d"[..];
        let mut tmp = vec![0u8; CURVE448_SIZE];

        assert!(mul_g(&mut tmp, alice_priv).is_ok());
        assert_eq!(tmp, alice_pub);

        assert!(mul_g(&mut tmp, bob_priv).is_ok());
        assert_eq!(tmp, bob_pub);

        assert!(mul(&mut tmp, alice_priv, bob_pub).is_ok());
        assert_eq!(tmp, shared);

        assert!(mul(&mut tmp, bob_priv, alice_pub).is_ok());
        assert_eq!(tmp, shared);
    }

    #[test]
    fn smoke_test() {
        if ! nettle_sys::config::HAVE_CV448 {
            eprintln!("This version of Nettle does not support the operation");
            return;
        }

        let mut rng = Yarrow::default();
        let sk1 = private_key(&mut rng);
        let sk2 = private_key(&mut rng);

        let mut pk1 = vec![0u8; CURVE448_SIZE];
        let mut pk2 = vec![0u8; CURVE448_SIZE];

        assert!(mul_g(&mut pk1[..], &sk1[..]).is_ok());
        assert!(mul_g(&mut pk2[..], &sk2[..]).is_ok());

        let mut sh1 = vec![0u8; CURVE448_SIZE];
        let mut sh2 = vec![0u8; CURVE448_SIZE];

        assert!(mul(&mut sh1[..], &sk1[..], &pk2[..]).is_ok());
        assert!(mul(&mut sh2[..], &sk2[..], &pk1[..]).is_ok());

        assert_eq!(sh1, sh2);
    }
}
