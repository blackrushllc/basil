use nettle_sys::{
    ecc_curve, nettle_ecc_bit_size, nettle_get_secp_192r1,
    nettle_get_secp_224r1, nettle_get_secp_256r1, nettle_get_secp_384r1,
    nettle_get_secp_521r1,
};

/// Elliptic curve for ECDSA.
pub trait Curve {
    /// Returns a pointer to the Nettle curve structure.
    unsafe fn get_curve() -> *const ecc_curve;

    /// Returns the size of the underlying groups prime in bits.
    unsafe fn bit_size() -> u32 {
        nettle_ecc_bit_size(Self::get_curve())
    }
}

/// NIST secp192r1 a.k.a. P-192.
pub struct Secp192r1;

impl Curve for Secp192r1 {
    unsafe fn get_curve() -> *const ecc_curve {
        nettle_get_secp_192r1()
    }
}

/// NIST secp224r1 a.k.a. P-224.
pub struct Secp224r1;

impl Curve for Secp224r1 {
    unsafe fn get_curve() -> *const ecc_curve {
        nettle_get_secp_224r1()
    }
}

/// NIST secp256r1 a.k.a. P-256.
pub struct Secp256r1;

impl Curve for Secp256r1 {
    unsafe fn get_curve() -> *const ecc_curve {
        nettle_get_secp_256r1()
    }
}

/// NIST secp384r1 a.k.a. P-384.
pub struct Secp384r1;

impl Curve for Secp384r1 {
    unsafe fn get_curve() -> *const ecc_curve {
        nettle_get_secp_384r1()
    }
}

/// NIST secp521r1 a.k.a. P-521.
pub struct Secp521r1;

impl Curve for Secp521r1 {
    unsafe fn get_curve() -> *const ecc_curve {
        nettle_get_secp_521r1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve() {
        unsafe {
            assert_eq!(Secp192r1::get_curve(), nettle_get_secp_192r1());
            assert_eq!(Secp224r1::get_curve(), nettle_get_secp_224r1());
            assert_eq!(Secp256r1::get_curve(), nettle_get_secp_256r1());
            assert_eq!(Secp384r1::get_curve(), nettle_get_secp_384r1());
            assert_eq!(Secp521r1::get_curve(), nettle_get_secp_521r1());
        }
    }

    #[test]
    fn bit_size() {
        unsafe {
            assert_eq!(Secp192r1::bit_size(), 192);
            assert_eq!(Secp224r1::bit_size(), 224);
            assert_eq!(Secp256r1::bit_size(), 256);
            assert_eq!(Secp384r1::bit_size(), 384);
            assert_eq!(Secp521r1::bit_size(), 521);
        }
    }
}
