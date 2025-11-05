use nettle_sys::{
    nettle_ecc_point_init, nettle_ecc_scalar_init,
    nettle_ecdsa_generate_keypair,
};
use std::mem::zeroed;

use crate::{
    random::Random,
    Result,
    ecc::{Curve, Point, Scalar},
};

/// Generates a new ECDSA key pair for siging.
pub fn generate_keypair<C: Curve, R: Random>(
    random: &mut R,
) -> Result<(Point, Scalar)> {
    unsafe {
        let mut point = zeroed();
        let mut scalar = zeroed();

        nettle_ecc_point_init(&mut point, C::get_curve());
        nettle_ecc_scalar_init(&mut scalar, C::get_curve());
        nettle_ecdsa_generate_keypair(
            &mut point,
            &mut scalar,
            random.context(),
            Some(R::random_impl),
        );

        let point = Point { point: point };
        let scalar = Scalar { scalar: scalar };

        Ok((point, scalar))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecdsa::{sign, verify};
    use crate::{ecc::Secp192r1, random::Yarrow};

    #[test]
    fn gen_keys() {
        let mut rand = Yarrow::default();

        for _ in 0..3 {
            let _ = generate_keypair::<Secp192r1, _>(&mut rand).unwrap();
        }
    }

    #[test]
    fn clone() {
        let mut rand = Yarrow::default();
        let (public, private) =
            generate_keypair::<Secp192r1, _>(&mut rand).unwrap();
        let mut msg = [0u8; 160];

        rand.random(&mut msg);
        let sig = sign(&private, &msg, &mut rand);
        let sig = sig.clone();

        assert!(verify(&public, &msg, &sig));
    }
}
