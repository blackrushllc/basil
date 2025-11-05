use nettle_sys::{
    __gmpz_clear, __gmpz_init, ecc_point, ecc_scalar, nettle_ecc_point_clear,
    nettle_ecc_point_get, nettle_ecc_point_init, nettle_ecc_point_set,
    nettle_ecc_scalar_clear, nettle_ecc_scalar_get, nettle_ecc_scalar_init,
    nettle_ecc_scalar_set,
};
use std::mem::zeroed;

use crate::errors::Error;
use crate::{helper, ecc::Curve, random::Random, Result};

/// Secret scalar.
pub struct Scalar {
    pub(crate) scalar: ecc_scalar,
}

impl Scalar {
    /// Creates a new private scalar based on the big endian integer `num`.
    pub fn new<C: Curve>(num: &[u8]) -> Result<Scalar> {
        unsafe {
            let mut scalar: ecc_scalar = zeroed();

            nettle_ecc_scalar_init(&mut scalar as *mut _, C::get_curve());
            let mut mpz = helper::convert_buffer_to_gmpz(num);

            if nettle_ecc_scalar_set(&mut scalar as *mut _, &mut mpz) == 1 {
                __gmpz_clear(&mut mpz as *mut _);

                Ok(Scalar { scalar: scalar })
            } else {
                __gmpz_clear(&mut mpz as *mut _);
                nettle_ecc_scalar_clear(&mut scalar as *mut _);

                Err(Error::InvalidArgument { argument_name: "num" })
            }
        }
    }

    /// Creates a new random scalar between 1 and the group order of `C`.
    pub fn new_random<C, R>(rng: &mut R) -> Self
    where
        C: Curve,
        R: Random,
    {
        let bits = unsafe { C::bit_size() };
        let bytes = if bits % 8 > 0 { 1 } else { 0 } + bits / 8;
        let mut buf = vec![0u8; bytes as usize];

        loop {
            rng.random(&mut buf);

            if let Ok(sc) = Scalar::new::<C>(&buf) {
                return sc
            }
        }
    }

    /// Returns the private scalar as big endian integer.
    pub fn as_bytes(&self) -> Box<[u8]> {
        unsafe {
            let mut mpz = zeroed();

            __gmpz_init(&mut mpz as *mut _);
            nettle_ecc_scalar_get(&self.scalar as *const _, &mut mpz);

            let ret = helper::convert_gmpz_to_buffer(mpz);
            __gmpz_clear(&mut mpz);
            ret
        }
    }
}

impl Clone for Scalar {
    fn clone(&self) -> Self {
        unsafe {
            // XXX: no nettle_ecc_scalar_copy()
            let buf = self.as_bytes();
            let mut mpz = helper::convert_buffer_to_gmpz(&*buf);
            let mut ret: ecc_scalar = zeroed();

            nettle_ecc_scalar_init(&mut ret, self.scalar.ecc);
            assert_eq!(nettle_ecc_scalar_set(&mut ret, &mut mpz), 1);
            __gmpz_clear(&mut mpz);

            Scalar { scalar: ret }
        }
    }
}

impl Drop for Scalar {
    fn drop(&mut self) {
        unsafe {
            nettle_ecc_scalar_clear(&mut self.scalar as *mut _);
        }
    }
}

/// Public point.
pub struct Point {
    pub(crate) point: ecc_point,
}

impl Point {
    /// Creates a new point on `C` with coordinates `x` & `y`.
    ///
    /// Can fail if the given point is not on the curve.
    pub fn new<C: Curve>(x: &[u8], y: &[u8]) -> Result<Point> {
        unsafe {
            let mut point: ecc_point = zeroed();
            nettle_ecc_point_init(&mut point as *mut _, C::get_curve());

            let mut x_mpz = helper::convert_buffer_to_gmpz(x);
            let mut y_mpz = helper::convert_buffer_to_gmpz(y);

            if nettle_ecc_point_set(
                &mut point as *mut _,
                &mut x_mpz,
                &mut y_mpz,
            ) == 1
            {
                __gmpz_clear(&mut x_mpz as *mut _);
                __gmpz_clear(&mut y_mpz as *mut _);

                Ok(Point { point: point })
            } else {
                nettle_ecc_point_clear(&mut point as *mut _);

                __gmpz_clear(&mut x_mpz as *mut _);
                __gmpz_clear(&mut y_mpz as *mut _);

                Err(Error::InvalidArgument { argument_name: "x or y" })
            }
        }
    }

    /// Returns the points coordinates as big endian integers.
    pub fn as_bytes(&self) -> (Box<[u8]>, Box<[u8]>) {
        unsafe {
            let mut x_mpz = zeroed();
            let mut y_mpz = zeroed();

            __gmpz_init(&mut x_mpz as *mut _);
            __gmpz_init(&mut y_mpz as *mut _);
            nettle_ecc_point_get(
                &self.point as *const _,
                &mut x_mpz,
                &mut y_mpz,
            );

            let x_ret = helper::convert_gmpz_to_buffer(x_mpz);
            let y_ret = helper::convert_gmpz_to_buffer(y_mpz);
            __gmpz_clear(&mut x_mpz as *mut _);
            __gmpz_clear(&mut y_mpz as *mut _);

            (x_ret, y_ret)
        }
    }
}

impl Clone for Point {
    fn clone(&self) -> Self {
        unsafe {
            let (buf_x, buf_y) = self.as_bytes();
            let mut mpz_x = helper::convert_buffer_to_gmpz(&*buf_x);
            let mut mpz_y = helper::convert_buffer_to_gmpz(&*buf_y);
            let mut ret: ecc_point = zeroed();

            nettle_ecc_point_init(&mut ret, self.point.ecc);
            assert_eq!(
                nettle_ecc_point_set(&mut ret, &mut mpz_x, &mut mpz_y),
                1
            );
            __gmpz_clear(&mut mpz_x);
            __gmpz_clear(&mut mpz_y);

            Point { point: ret }
        }
    }
}

impl Drop for Point {
    fn drop(&mut self) {
        unsafe {
            nettle_ecc_point_clear(&mut self.point as *mut _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;
    use crate::ecc::{Secp192r1, Secp224r1, Secp256r1, Secp384r1, Secp521r1};

    #[test]
    fn random_scalar() {
        let mut rng = Yarrow::default();

        let sc1 = Scalar::new_random::<Secp192r1, _>(&mut rng).as_bytes();
        let sc2 = Scalar::new_random::<Secp192r1, _>(&mut rng).as_bytes();
        assert_ne!(sc1, sc2);

        let sc1 = Scalar::new_random::<Secp224r1, _>(&mut rng).as_bytes();
        let sc2 = Scalar::new_random::<Secp224r1, _>(&mut rng).as_bytes();
        assert_ne!(sc1, sc2);

        let sc1 = Scalar::new_random::<Secp256r1, _>(&mut rng).as_bytes();
        let sc2 = Scalar::new_random::<Secp256r1, _>(&mut rng).as_bytes();
        assert_ne!(sc1, sc2);

        let sc1 = Scalar::new_random::<Secp384r1, _>(&mut rng).as_bytes();
        let sc2 = Scalar::new_random::<Secp384r1, _>(&mut rng).as_bytes();
        assert_ne!(sc1, sc2);

        let sc1 = Scalar::new_random::<Secp521r1, _>(&mut rng).as_bytes();
        let sc2 = Scalar::new_random::<Secp521r1, _>(&mut rng).as_bytes();
        assert_ne!(sc1, sc2);
    }

    #[test]
    fn clone_scalar() {
        let mut rng = Yarrow::default();

        let sc1 = Scalar::new_random::<Secp192r1, _>(&mut rng);
        let sc2 = sc1.clone();
        assert_eq!(sc1.as_bytes(), sc2.as_bytes());
    }

    #[test]
    fn clone_point() {
        // From ecdh::tests::nist_p_192 {
        let point1 = Point::new::<Secp192r1>(
            &b"\x42\xea\x6d\xd9\x96\x9d\xd2\xa6\x1f\xea\x1a\xac\x7f\x8e\x98\xed\xcc\x89\x6c\x6e\x55\x85\x7c\xc0"[..],
            &b"\xdf\xbe\x5d\x7c\x61\xfa\xc8\x8b\x11\x81\x1b\xde\x32\x8e\x8a\x0d\x12\xbf\x01\xa9\xd2\x04\xb5\x23"[..]).unwrap();

        let point2 = point1.clone();
        assert_eq!(point1.as_bytes(), point2.as_bytes());
    }

    #[test]
    #[should_panic]
    fn point_new_not_on_curve() {
        let _ = Point::new::<Secp192r1>(
            &b"\x00\xea\x6d\xd9\x96\x9d\xd2\xa6\x1f\xea\x1a\xac\x7f\x8e\x98\xed\xcc\x89\x6c\x6e\x55\x85\x7c\xc0"[..],
            &b"\xdf\xbe\x5d\x7c\x61\xfa\xc8\x8b\x11\x81\x1b\xde\x32\x8e\x8a\x0d\x12\xbf\x01\xa9\xd2\x04\xb5\x23"[..]).unwrap();
    }
}
