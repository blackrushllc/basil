use nettle_sys::{
    __gmpz_set, dsa_params, nettle_dsa_generate_params,
    nettle_dsa_params_clear, nettle_dsa_params_init,
    nettle_mpz_set_str_256_u,
};
use std::mem::zeroed;
use std::ptr;

use crate::helper::convert_gmpz_to_buffer;
use crate::{Error, random::Random, Result};

/// Public parameters for DSA signatures.
///
/// DSA uses the ring Z/pqZ (p,q are primes) and a generator `g` of that group.
pub struct Params {
    pub(crate) params: dsa_params,
}

impl Params {
    /// Create a new DSA parameter structure with primes `p` and `q` and generator `g`.
    pub fn new(p: &[u8], q: &[u8], g: &[u8]) -> Params {
        unsafe {
            let mut ret: dsa_params = zeroed();

            nettle_dsa_params_init(&mut ret as *mut _);
            nettle_mpz_set_str_256_u(&mut ret.p[0], p.len(), p.as_ptr());
            nettle_mpz_set_str_256_u(&mut ret.q[0], q.len(), q.as_ptr());
            nettle_mpz_set_str_256_u(&mut ret.g[0], g.len(), g.as_ptr());

            Params { params: ret }
        }
    }

    /// Generates a fresh set of parameters.
    ///
    /// `p_bits` and `q_bits` are the size of the primes. FIPS-186 expects `q_bits = 160`
    /// and `p_bits = 512 + 64l` for `l = [0-8]`. The Nettle documentation recommends 1024.
    pub fn generate<R: Random>(
        random: &mut R,
        p_bits: usize,
        q_bits: usize,
    ) -> Result<Params> {
        unsafe {
            let mut ret = zeroed();

            nettle_dsa_params_init(&mut ret as *mut _);
            if nettle_dsa_generate_params(
                &mut ret as *mut _,
                random.context(),
                Some(R::random_impl),
                ptr::null_mut(),
                None,
                p_bits as u32,
                q_bits as u32,
            ) == 1
            {
                Ok(Params { params: ret })
            } else {
                nettle_dsa_params_clear(&mut ret as *mut _);

                Err(Error::InvalidBitSizes)
            }
        }
    }

    /// Returns the primes `p` and `q` ad big endian integers.
    pub fn primes(&self) -> (Box<[u8]>, Box<[u8]>) {
        let p = convert_gmpz_to_buffer(self.params.p[0]);
        let q = convert_gmpz_to_buffer(self.params.q[0]);

        (p, q)
    }

    /// Returns the generator `g` as big endian integer.
    pub fn g(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.params.g[0])
    }
}

impl Clone for Params {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret = zeroed();

            nettle_dsa_params_init(&mut ret);
            __gmpz_set(&mut ret.p[0], &self.params.p[0]);
            __gmpz_set(&mut ret.q[0], &self.params.q[0]);
            __gmpz_set(&mut ret.g[0], &self.params.g[0]);

            Params { params: ret }
        }
    }
}

impl Drop for Params {
    fn drop(&mut self) {
        unsafe {
            nettle_dsa_params_clear(&mut self.params as *mut _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;

    #[test]
    fn new_params() {
        let p = &['p' as u8; 12];
        let q = &['q' as u8; 12];
        let g = &['g' as u8; 12];
        let params = Params::new(p, q, g);

        assert_eq!(params.primes().0.as_ref(), p);
        assert_eq!(params.primes().1.as_ref(), q);
        assert_eq!(params.g().as_ref(), g);
    }

    #[test]
    fn generate_params() {
        let mut rand = Yarrow::default();
        let _ = Params::generate(&mut rand, 1024, 160).unwrap();
    }

    #[test]
    #[should_panic]
    fn generate_params_failing() {
        let mut rand = Yarrow::default();
        let _ = Params::generate(&mut rand, 1024, 16).unwrap();
    }
}
