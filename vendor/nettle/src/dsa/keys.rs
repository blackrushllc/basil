use nettle_sys::{
    __gmpz_clear, __gmpz_init, __gmpz_init_set, mpz_t,
    nettle_dsa_generate_keypair,
};
use std::mem::zeroed;

use crate::helper::{convert_buffer_to_gmpz, convert_gmpz_to_buffer};
use crate::random::Random;

use super::Params;

/// Public DSA key.
pub struct PublicKey {
    pub(crate) public: mpz_t,
}

impl PublicKey {
    /// Creates a new public key.
    pub fn new(y: &[u8]) -> PublicKey {
        PublicKey { public: [convert_buffer_to_gmpz(y)] }
    }

    /// Returns the public key `y` as big endian number.
    pub fn as_bytes(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.public[0])
    }
}

impl Clone for PublicKey {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret: mpz_t = zeroed();

            __gmpz_init_set(&mut ret[0], &self.public[0]);
            PublicKey { public: ret }
        }
    }
}

impl Drop for PublicKey {
    fn drop(&mut self) {
        unsafe {
            __gmpz_clear(&mut self.public[0] as *mut _);
        }
    }
}

/// Private DSA key.
pub struct PrivateKey {
    pub(crate) private: mpz_t,
}

impl PrivateKey {
    /// Creates a new private key sructure.
    ///
    /// The secret exponent `x` must be a big endian integer.
    pub fn new(x: &[u8]) -> PrivateKey {
        PrivateKey { private: [convert_buffer_to_gmpz(x)] }
    }

    /// Returns the secret exponent `x` as bit endian integer.
    pub fn as_bytes(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.private[0])
    }
}

impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret: mpz_t = zeroed();

            __gmpz_init_set(&mut ret[0], &self.private[0]);
            PrivateKey { private: ret }
        }
    }
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        unsafe {
            __gmpz_clear(&mut self.private[0] as *mut _);
        }
    }
}

/// Generates a fresh DSA key pair.
///
/// Generator and primes must be supplied via `params`. Entrophy is
/// gathered using `random`.
pub fn generate_keypair<R: Random>(
    params: &Params,
    random: &mut R,
) -> (PublicKey, PrivateKey) {
    unsafe {
        let mut public: mpz_t = zeroed();
        let mut private: mpz_t = zeroed();

        __gmpz_init(&mut public[0] as *mut _);
        __gmpz_init(&mut private[0] as *mut _);

        nettle_dsa_generate_keypair(
            &params.params,
            &mut public[0],
            &mut private[0],
            random.context(),
            Some(R::random_impl),
        );

        let ret_pub = PublicKey { public: public };
        let ret_key = PrivateKey { private: private };

        (ret_pub, ret_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsa::{sign, verify, Params};
    use crate::random::Yarrow;

    #[test]
    fn generate_key() {
        let mut rand = Yarrow::default();
        let params = Params::generate(&mut rand, 1024, 160).unwrap();

        for _ in 0..3 {
            let _ = generate_keypair(&params, &mut rand);
        }
    }

    #[test]
    fn new_public_key() {
        let y = &['y' as u8; 12];
        let public = PublicKey::new(y);
        assert_eq!(public.as_bytes().as_ref(), y);
    }

    #[test]
    fn new_private_key() {
        let x = &['x' as u8; 12];
        let private = PrivateKey::new(x);
        assert_eq!(private.as_bytes().as_ref(), x);
    }

    #[test]
    fn clone() {
        let mut rand = Yarrow::default();
        let params = Params::generate(&mut rand, 1024, 160).unwrap();
        let (public, private) = generate_keypair(&params, &mut rand);

        let public = public.clone();
        let private = private.clone();
        let params = params.clone();
        let mut msg = [0u8; 160];

        rand.random(&mut msg);
        let sig = sign(&params, &private, &msg, &mut rand).unwrap();
        let sig = sig.clone();

        assert!(verify(&params, &public, &msg, &sig));
    }
}
