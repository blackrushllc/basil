//! Elliptic curve Diffie-Hellman using D.J. Bernstein's Curve25519.

use nettle_sys::{nettle_curve25519_mul, nettle_curve25519_mul_g};

use crate::{Error, random::Random, Result};

/// Size of the public and private keys in bytes.
pub const CURVE25519_SIZE: usize = ::nettle_sys::CURVE25519_SIZE as usize;

/// Generate a new X25519 private key.
pub fn private_key<R>(rng: &mut R) -> Box<[u8]>
where
    R: Random,
{
    let mut ret = vec![0u8; CURVE25519_SIZE].into_boxed_slice();

    // Curve25519 Paper, Sec. 3:
    // A user can, for example, generate 32 uniform random bytes, clear bits 0, 1, 2 of the first
    // byte, clear bit 7 of the last byte, and set bit 6 of the last byte.
    rng.random(&mut ret[..]);
    ret[0] &= 0b1111_1000;
    ret[CURVE25519_SIZE - 1] &= !0b1000_0000;
    ret[CURVE25519_SIZE - 1] |= 0b0100_0000;

    ret
}

/// Derive DH public key.
///
/// Computes the public key `q` for a given secret `n`. Returns an
/// error if `q` or `n` are not [`CURVE25519_SIZE`] bytes long.
pub fn mul_g(q: &mut [u8], n: &[u8]) -> Result<()> {
    if q.len() != CURVE25519_SIZE {
        return Err(Error::InvalidArgument { argument_name: "q" });
    }
    if n.len() != CURVE25519_SIZE {
        return Err(Error::InvalidArgument { argument_name: "n" });
    }

    unsafe {
        nettle_curve25519_mul_g(q.as_mut_ptr(), n.as_ptr());
    }

    Ok(())
}

/// Derive DH shared secret.
///
/// Computes the shared secret `q` for our private key `n` and the
/// other parties public key `p`.  Returns an error if `q`, `n`, or
/// `p` are not [`CURVE25519_SIZE`] bytes long.
pub fn mul(q: &mut [u8], n: &[u8], p: &[u8]) -> Result<()> {
    if q.len() != CURVE25519_SIZE {
        return Err(Error::InvalidArgument { argument_name: "q" });
    }
    if n.len() != CURVE25519_SIZE {
        return Err(Error::InvalidArgument { argument_name: "n" });
    }
    if p.len() != CURVE25519_SIZE {
        return Err(Error::InvalidArgument { argument_name: "p" });
    }

    unsafe {
        nettle_curve25519_mul(q.as_mut_ptr(), n.as_ptr(), p.as_ptr());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;

    #[test]
    fn rfc7748() {
        let alice_priv = &b"\x77\x07\x6d\x0a\x73\x18\xa5\x7d\x3c\x16\xc1\x72\x51\xb2\x66\x45\xdf\x4c\x2f\x87\xeb\xc0\x99\x2a\xb1\x77\xfb\xa5\x1d\xb9\x2c\x2a"[..];
        let alice_pub = &b"\x85\x20\xf0\x09\x89\x30\xa7\x54\x74\x8b\x7d\xdc\xb4\x3e\xf7\x5a\x0d\xbf\x3a\x0d\x26\x38\x1a\xf4\xeb\xa4\xa9\x8e\xaa\x9b\x4e\x6a"[..];
        let bob_priv = &b"\x5d\xab\x08\x7e\x62\x4a\x8a\x4b\x79\xe1\x7f\x8b\x83\x80\x0e\xe6\x6f\x3b\xb1\x29\x26\x18\xb6\xfd\x1c\x2f\x8b\x27\xff\x88\xe0\xeb"[..];
        let bob_pub = &b"\xde\x9e\xdb\x7d\x7b\x7d\xc1\xb4\xd3\x5b\x61\xc2\xec\xe4\x35\x37\x3f\x83\x43\xc8\x5b\x78\x67\x4d\xad\xfc\x7e\x14\x6f\x88\x2b\x4f"[..];
        let shared = &b"\x4a\x5d\x9d\x5b\xa4\xce\x2d\xe1\x72\x8e\x3b\xf4\x80\x35\x0f\x25\xe0\x7e\x21\xc9\x47\xd1\x9e\x33\x76\xf0\x9b\x3c\x1e\x16\x17\x42"[..];
        let mut tmp = vec![0u8; CURVE25519_SIZE];

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
        let mut rng = Yarrow::default();
        let sk1 = private_key(&mut rng);
        let sk2 = private_key(&mut rng);

        let mut pk1 = vec![0u8; CURVE25519_SIZE];
        let mut pk2 = vec![0u8; CURVE25519_SIZE];

        assert!(mul_g(&mut pk1[..], &sk1[..]).is_ok());
        assert!(mul_g(&mut pk2[..], &sk2[..]).is_ok());

        let mut sh1 = vec![0u8; CURVE25519_SIZE];
        let mut sh2 = vec![0u8; CURVE25519_SIZE];

        assert!(mul(&mut sh1[..], &sk1[..], &pk2[..]).is_ok());
        assert!(mul(&mut sh2[..], &sk2[..], &pk1[..]).is_ok());

        assert_eq!(sh1, sh2);
    }
}
