use nettle_sys::{
    __gmpz_add_ui, __gmpz_clear, __gmpz_cmp_ui, __gmpz_fdiv_r, __gmpz_init,
    __gmpz_invert, __gmpz_mul, __gmpz_set, __gmpz_sub_ui,
    nettle_mpz_set_str_256_u, nettle_mpz_sizeinbase_256_u,
    nettle_rsa_generate_keypair, nettle_rsa_private_key_clear,
    nettle_rsa_private_key_init, nettle_rsa_private_key_prepare,
    nettle_rsa_public_key_clear, nettle_rsa_public_key_init,
    nettle_rsa_public_key_prepare, rsa_private_key, rsa_public_key,
};
use std::mem::zeroed;

use crate::helper::convert_gmpz_to_buffer;
use crate::{Error, random::Random, Result};

/// A public RSA key.
pub struct PublicKey {
    pub(crate) context: rsa_public_key,
    pub(crate) modulo_bytes: usize,
}

impl PublicKey {
    /// Creates a new RSA public key with the modulo `n` and the
    /// public exponent `e`.
    ///
    /// Both are expected to be big endian integers.
    pub fn new(n: &[u8], e: &[u8]) -> Result<PublicKey> {
        unsafe {
            let mut ctx: rsa_public_key = zeroed();

            nettle_rsa_public_key_init(&mut ctx as *mut _);
            nettle_mpz_set_str_256_u(
                &mut ctx.e[0] as *mut _,
                e.len(),
                e.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.n[0] as *mut _,
                n.len(),
                n.as_ptr(),
            );

            let mut ret = PublicKey { context: ctx, modulo_bytes: n.len() };

            if nettle_rsa_public_key_prepare(&mut ret.context as *mut _) == 1 {
                Ok(ret)
            } else {
                Err(Error::InvalidArgument { argument_name: "" })
            }
        }
    }

    /// Returns the modulo as a big endian integer.
    pub fn n(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.context.n[0])
    }

    /// Returns the public exponent as a big endian integer.
    pub fn e(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.context.e[0])
    }
}

impl Clone for PublicKey {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret = zeroed();

            nettle_rsa_public_key_init(&mut ret);
            __gmpz_set(&mut ret.e[0], &self.context.e[0]);
            __gmpz_set(&mut ret.n[0], &self.context.n[0]);

            PublicKey { context: ret, modulo_bytes: self.modulo_bytes }
        }
    }
}

impl Drop for PublicKey {
    fn drop(&mut self) {
        unsafe {
            nettle_rsa_public_key_clear(&mut self.context as *mut _);
        }
    }
}

/// A private RSA key.
pub struct PrivateKey {
    pub(crate) context: rsa_private_key,
}

impl PrivateKey {
    /// Creates a new private key with the private exponent `d` and
    /// the two primes `p`/`q`.
    ///
    /// If the inverse `inv` for `q` in Z_p is `None` the function
    /// computes it.
    ///
    /// # Notes
    ///
    /// This function does time-variable computation on the private
    /// key that may leak it. See `new_crt` for a time-constant way to
    /// create a PrivateKey.
    pub fn new<'a, O: Into<Option<&'a [u8]>>>(
        d: &[u8],
        p: &[u8],
        q: &[u8],
        inv: O,
    ) -> Result<Self> {
        unsafe {
            let mut ctx: rsa_private_key = zeroed();

            nettle_rsa_private_key_init(&mut ctx as *mut _);
            nettle_mpz_set_str_256_u(
                &mut ctx.d[0] as *mut _,
                d.len(),
                d.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.p[0] as *mut _,
                p.len(),
                p.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.q[0] as *mut _,
                q.len(),
                q.as_ptr(),
            );

            __gmpz_sub_ui(&mut ctx.p[0] as *mut _, &ctx.p[0] as *const _, 1);
            __gmpz_fdiv_r(
                &mut ctx.a[0] as *mut _,
                &ctx.d[0] as *const _,
                &ctx.p[0] as *const _,
            );
            __gmpz_add_ui(&mut ctx.p[0] as *mut _, &ctx.p[0] as *const _, 1);

            __gmpz_sub_ui(&mut ctx.q[0] as *mut _, &ctx.q[0] as *const _, 1);
            __gmpz_fdiv_r(
                &mut ctx.b[0] as *mut _,
                &ctx.d[0] as *const _,
                &ctx.q[0] as *const _,
            );
            __gmpz_add_ui(&mut ctx.q[0] as *mut _, &ctx.q[0] as *const _, 1);

            if let Some(inv) = inv.into() {
                nettle_mpz_set_str_256_u(
                    &mut ctx.c[0] as *mut _,
                    inv.len(),
                    inv.as_ptr(),
                );
            } else {
                __gmpz_invert(
                    &mut ctx.c[0] as *mut _,
                    &ctx.q[0] as *const _,
                    &ctx.p[0] as *const _,
                );
            }

            let mut ret = PrivateKey { context: ctx };

            if nettle_rsa_private_key_prepare(&mut ret.context as *mut _) == 1 {
                Ok(ret)
            } else {
                Err(Error::InvalidArgument { argument_name: "" })
            }
        }
    }

    /// Creates a new private key with the two primes `p`/`q`.
    ///
    /// The private exponent `d` is given as `dp = d mod p - 1` and
    /// `dq = d mod q - 1`. If the inverse `inv` of `q` in Z_p is
    /// `None` the function computes it.
    ///
    /// # Notes
    ///
    /// If `inv` is `None` the function does time-variable computation
    /// on the private key that may leak it.
    pub fn new_crt<'a, O: Into<Option<&'a [u8]>>>(
        dp: &[u8],
        dq: &[u8],
        p: &[u8],
        q: &[u8],
        inv: O,
    ) -> Result<Self> {
        unsafe {
            let mut ctx: rsa_private_key = zeroed();

            // d isn't used in nettle
            nettle_rsa_private_key_init(&mut ctx as *mut _);
            nettle_mpz_set_str_256_u(
                &mut ctx.p[0] as *mut _,
                p.len(),
                p.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.q[0] as *mut _,
                q.len(),
                q.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.a[0] as *mut _,
                dp.len(),
                dp.as_ptr(),
            );
            nettle_mpz_set_str_256_u(
                &mut ctx.b[0] as *mut _,
                dq.len(),
                dq.as_ptr(),
            );

            if let Some(inv) = inv.into() {
                nettle_mpz_set_str_256_u(
                    &mut ctx.c[0] as *mut _,
                    inv.len(),
                    inv.as_ptr(),
                );
            } else {
                __gmpz_invert(
                    &mut ctx.c[0] as *mut _,
                    &ctx.q[0] as *const _,
                    &ctx.p[0] as *const _,
                );
            }

            let mut ret = PrivateKey { context: ctx };

            if nettle_rsa_private_key_prepare(&mut ret.context as *mut _) == 1 {
                Ok(ret)
            } else {
                Err(Error::InvalidArgument { argument_name: "" })
            }
        }
    }

    /// Computes the public key to this private key.
    ///
    /// This only works if the private key was created with `new`
    /// instead of `new_crt`.
    ///
    /// # Notes
    ///
    /// This function does time-variable computation on the private
    /// key that may leak it.
    pub fn public_key(&self) -> Result<PublicKey> {
        unsafe {
            let mut p_minus_1 = zeroed();
            let mut q_minus_1 = zeroed();
            let mut phi = zeroed();
            let mut ret: rsa_public_key = zeroed();

            nettle_rsa_public_key_init(&mut ret as *mut _);
            __gmpz_init(&mut p_minus_1);
            __gmpz_init(&mut q_minus_1);
            __gmpz_init(&mut phi);

            __gmpz_sub_ui(&mut p_minus_1, &self.context.p[0], 1);
            __gmpz_sub_ui(&mut q_minus_1, &self.context.q[0], 1);
            __gmpz_mul(&mut phi, &p_minus_1, &q_minus_1);
            __gmpz_invert(&mut ret.e[0], &self.context.d[0], &phi);
            __gmpz_mul(&mut ret.n[0], &self.context.p[0], &self.context.q[0]);

            __gmpz_clear(&mut p_minus_1);
            __gmpz_clear(&mut q_minus_1);
            __gmpz_clear(&mut phi);

            let len = nettle_mpz_sizeinbase_256_u(&mut ret.n[0]);
            let mut ret = PublicKey { context: ret, modulo_bytes: len };

            if __gmpz_cmp_ui(&ret.context.e[0], 0) == 0 {
                Err(Error::InvalidArgument { argument_name: "self" })
            } else if nettle_rsa_public_key_prepare(&mut ret.context as *mut _)
                == 1
            {
                Ok(ret)
            } else {
                Err(Error::InvalidArgument { argument_name: "" })
            }
        }
    }

    /// Returns the tuple `a`, `b`, `c`.
    ///
    /// `a` and `b` are the two RSA primes `p`/`q` with `a` < `b` and
    /// `c = a^-1 mod b`.
    pub fn as_rfc4880(&self) -> (Box<[u8]>, Box<[u8]>, Box<[u8]>) {
        let p = convert_gmpz_to_buffer(self.context.p[0]);
        let q = convert_gmpz_to_buffer(self.context.q[0]);

        if p < q {
            unsafe {
                // RFC4880: u == p^-1 mod q
                // Nettle: c == q^-1 mod p
                let mut inv = zeroed();

                __gmpz_init(&mut inv);
                __gmpz_invert(
                    &mut inv as *mut _,
                    &self.context.p[0] as *const _,
                    &self.context.q[0] as *const _,
                );

                let tmp = convert_gmpz_to_buffer(inv);
                __gmpz_clear(&mut inv);

                (p, q, tmp)
            }
        } else {
            let inv = convert_gmpz_to_buffer(self.context.c[0]);

            (q, p, inv)
        }
    }

    /// Returns the primes `p`/`q` as big endian integers.
    pub fn primes(&self) -> (Box<[u8]>, Box<[u8]>) {
        let p = convert_gmpz_to_buffer(self.context.p[0]);
        let q = convert_gmpz_to_buffer(self.context.q[0]);

        (p, q)
    }

    /// Returns the private exponent `d` as pair of big endian integers `dp = d mod p - 1` and `dq
    /// = d mod q - 1` and the inverse of `q` in Z_p as third value.
    pub fn d_crt(&self) -> (Box<[u8]>, Box<[u8]>, Box<[u8]>) {
        let dp = convert_gmpz_to_buffer(self.context.a[0]);
        let dq = convert_gmpz_to_buffer(self.context.b[0]);
        let inv = convert_gmpz_to_buffer(self.context.c[0]);

        (dp, dq, inv)
    }

    /// Returns the private exponent `d` as big endian integer.
    pub fn d(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.context.d[0])
    }
}

impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret = zeroed();

            nettle_rsa_private_key_init(&mut ret);
            __gmpz_set(&mut ret.p[0], &self.context.p[0]);
            __gmpz_set(&mut ret.q[0], &self.context.q[0]);
            __gmpz_set(&mut ret.a[0], &self.context.a[0]);
            __gmpz_set(&mut ret.b[0], &self.context.b[0]);

            PrivateKey { context: ret }
        }
    }
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        unsafe {
            nettle_rsa_private_key_clear(&mut self.context as *mut _);
        }
    }
}

/// Generates a fresh RSA key pair usable for signing and encryption.
///
/// The public modulo `n` will be `modulo_size` bits large. The public
/// exponent is fixed to `0x10001`.
pub fn generate_keypair<R: Random>(
    random: &mut R,
    modulo_size: u32,
) -> Result<(PublicKey, PrivateKey)> {
    use std::ptr;

    let e = [0x01, 0x00, 0x01];

    unsafe {
        let mut public_ctx: rsa_public_key = zeroed();
        let mut private_ctx: rsa_private_key = zeroed();

        nettle_rsa_private_key_init(&mut private_ctx as *mut _);
        nettle_rsa_public_key_init(&mut public_ctx as *mut _);
        nettle_mpz_set_str_256_u(
            &mut public_ctx.e[0] as *mut _,
            e.len(),
            e.as_ptr(),
        );

        if nettle_rsa_generate_keypair(
            &mut public_ctx as *mut _,
            &mut private_ctx as *mut _,
            random.context(),
            Some(R::random_impl),
            ptr::null_mut(),
            None,
            modulo_size,
            0,
        ) == 1
        {
            Ok((
                PublicKey {
                    context: public_ctx,
                    modulo_bytes: modulo_size as usize / 8,
                },
                PrivateKey { context: private_ctx },
            ))
        } else {
            nettle_rsa_public_key_clear(&mut public_ctx as *mut _);
            nettle_rsa_private_key_clear(&mut private_ctx as *mut _);
            Err(Error::KeyGenerationFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_public_key() {
        let n = &b"\xcc\xc3\x86\x4e\x2b\x3d\x2d\x08\x89\x66\x90\x49\x75\x4f\xb8\x53\xa0\xbe\x04\xd0\x74\x49\x85\xe8\xc7\xf3\x84\x3b\xce\xa0\x12\xc2\x2a\x19\x18\x94\x6e\xda\x69\xb1\x1b\xed\x91\x96\xba\xb2\x58\x0f\x4c\xa6\x5f\xd3\x49\x08\xc5\x40\x58\xe9\xd1\x7a\xb0\x57\xed\x75\xad\xef\x51\x71\x5a\x8d\x9d\x43\x04\x32\x36\xff\xc1\x1f\xf8\x89\x95\x7d\x92\xcd\x72\x60\xb0\xa1\xe6\x20\x63\x1e\x03\x4b\x07\x48\x2d\xd6\xe5\xf6\xad\x30\xa9\x87\x81\x2a\x82\x00\x03\x9f\x53\x05\x71\x11\x65\xf0\xf8\xa8\x6e\xe7\x31\xb8\xd1\xf0\x58\xae\xad\x69\x72\x36\x80\x3b\xdd\x03\x8c\xa6\xc4\xf6\x22\xd7\xd4\x58\x92\xbc\xa0\x26\xd8\xd9\x49\xbe\xfe\x80\x35\x88\x02\xff\x7a\x5a\x40\xc2\xc6\x72\xa9\x3c\xf3\xf0\x61\x60\x45\x3c\xf7\xa1\x96\x66\x54\xde\x9d\xd0\xcb\xeb\xa7\x81\x3b\xda\x29\x0f\xbe\x03\xb0\xfe\xcc\xaf\x19\xdd\xcd\x15\x86\x49\xf8\xc0\xa5\x08\x3b\x2a\x42\x3d\xe2\x00\x33\x6c\x40\x70\x8c\xfc\x4a\x3e\x83\x1e\x7a\x53\x0c\x07\x0f\xc6\xfd\x31\xd5\xf8\x1e\xa7\x73\x47\x67\xf5\x76\x0a\xb1\x02\xa4\xd0\xcb\x79\xb2\xeb\x9c\x93\xa0\x6b\x23\xd2\xa7\x5b\xbb\x40\xa0\x95"[..];
        let e = &b"\x01\x00\x01"[..];

        assert!(PublicKey::new(n, e).is_ok());
        assert!(PublicKey::new(e, n).is_err());

        let key1 = PublicKey::new(n, e).unwrap();
        assert_eq!(&(*key1.e())[..], e);
        assert_eq!(&(*key1.n())[..], n);
    }

    #[test]
    fn load_private_key() {
        let p = &b"\xf8\xb6\x0f\xca\xc8\x11\xf3\x6e\x68\x2f\x87\xd0\xbb\xb0\x46\x7e\x3f\x20\x6e\xf8\x61\xc1\xc8\x64\xa5\xfc\xda\xc2\x01\x4b\xf5\x11\x16\xa6\xd4\x9d\x45\x26\x37\x1e\x32\x1a\x60\x71\x11\xf9\x1a\xb3\x83\x50\xfa\xdc\x95\x96\x73\xbc\x50\xb6\x6d\xc4\x18\x0e\x1d\x0a\x31\xf2\x63\xae\xb5\xad\xb3\xfc\x8f\x1e\x76\x3b\x38\xfa\x2d\x4c\x48\x04\x59\xae\x5f\x49\xc0\x7a\xea\x0c\x37\xeb\xb9\xa6\x23\xff\xf2\x84\x70\x21\xba\x60\xf1\x9c\x85\xe4\xe3\x68\x74\xef\x92\x10\xb1\xcb\x50\x4a\xd2\xdf\xcb\x3a\xbb\xdd\x1c\x1c\x25\xe1\x0c\x45"[..];
        let q = &b"\xd2\xc3\xc0\x24\x48\x44\x7d\xe2\x43\x13\xf2\x51\x58\x07\x07\x6a\x7b\xc1\x37\x09\x3d\xc7\xcf\x04\x3d\xfc\xee\x91\x20\x3b\xe7\x94\x3d\xb0\x43\xb8\x19\x56\x77\x0e\xca\x89\xb1\x15\xa5\xbf\x8e\xa7\xe6\xa0\xf8\x89\x71\xef\xc8\x29\xba\xfe\x2e\xd0\xb2\x4d\xf2\xf5\xf0\x0a\x51\x84\x45\x99\x3b\x7d\x43\xa1\x0e\x5f\x2a\x35\x3e\x21\xb2\xe6\x5d\x7d\x37\xa9\xb9\xcb\x29\x9f\x83\x3e\xe7\x8c\xe4\x9a\x1e\xc5\x12\xa6\x5e\x75\x79\x54\x6c\x0d\x0e\x37\x02\x93\xe6\x94\x50\x25\x62\x0d\xf9\x4a\x85\xb2\x8a\xdb\x9b\xcd\xc9\x48\x90\x11"[..];
        let d = &b"\xb9\x82\x12\x75\x53\x3b\x13\x47\x7e\xa3\xca\xe8\xa2\x3d\x5d\x33\x76\x97\x70\x69\x22\x51\x51\xde\x38\xf8\x67\xbe\x7f\x4e\x0a\x70\x9a\x0f\x2e\x73\x9b\x92\x88\xda\x8a\x00\xcb\x3b\x91\x5c\xed\xe6\xbe\x7c\xa4\xa8\x18\xac\xb7\x77\xba\x95\xea\xd0\x8a\x5e\xbe\xa5\x34\xb5\x72\x45\x8d\xd2\x6a\xbd\x42\x59\xf3\xf5\xf5\x13\x6f\xe9\xc5\xe3\x84\x52\x58\xe3\x3c\x63\x10\xc8\x1a\xc4\x20\x1b\xa3\x37\x43\x40\x44\x0b\x1d\x90\xce\xa6\x12\xe1\x5d\xf1\x81\x56\xa0\xb2\x77\x63\x59\xc5\xdc\xa1\x4c\x08\x6c\xc5\x92\x7d\x7a\xfa\x95\xe3\x27\xee\x1e\x93\x15\x83\x67\x16\xb4\x25\x66\x2f\x73\x9b\x27\x6e\x09\x72\x8d\xc3\x51\x53\x1c\x58\xc5\xec\x62\x57\xc0\x08\xab\xe6\xf9\x6f\xfb\xf1\xb3\x36\xa5\x0d\x3b\x22\x3a\xa5\xf1\x43\xc9\xf5\x0d\xd3\x40\xba\x9e\xdb\x99\x73\xa1\xfc\xe3\xe7\x43\x50\x00\x3d\x4c\x19\x43\xb8\x5c\x54\x9d\xdc\xe9\xba\x32\x9b\x59\x6c\x13\x04\x19\xe9\x51\x61\x86\xbd\x29\x1d\x5c\x6a\xca\xc0\x65\x38\x3e\x5f\xf6\x41\xfe\xcb\xad\x19\x59\xc2\x6b\x1c\x78\xa5\x63\xeb\xd1\x2a\xc2\x65\x3a\x7d\xc8\x0f\x00\x93\x78\xf2\x7c\x39\xce\xba\xb3\x81"[..];
        let inv = &b"\x9c\xa9\x1f\x96\x80\x62\x7f\xb0\xfe\x26\xa8\x82\xfe\x57\x14\xd0\xcc\x9f\xb1\xdd\xf8\x36\xcb\xd8\x25\x6d\x45\xd3\x99\xfb\x85\x2f\x57\x70\x84\xc8\xfd\x12\x52\x2c\xf1\x32\xac\x4c\x25\x02\x17\xed\xf2\x2e\xac\x23\xdd\xec\x9b\xed\x3f\xf6\xb0\xc4\x99\xef\x14\xac\xcd\x4a\x30\xa8\xc5\xff\x47\x8e\x28\xfa\x99\x41\x1c\x4f\xc0\xdd\x59\x1d\x26\x94\x76\x9a\x0b\x64\x7f\xc1\x6f\xf8\x99\x5e\x19\xe3\x70\x43\x9b\x27\x58\x7e\x24\x15\x34\x4f\x2b\x74\x31\xa0\xc9\x41\x20\x41\x57\x44\x62\x04\xd9\xf0\x29\xb4\x43\x34\x4d\x09\xbe\xd3"[..];
        let dp = &b"\xd7\x10\xa0\x21\x4a\xd6\x72\xf9\x32\xf0\x7e\xf2\x19\x8a\xb9\xba\x6b\x9e\x01\x37\x99\x58\xf7\x8f\x49\x15\x98\x99\x10\x83\xfd\x3b\xb4\xa9\xb8\xca\xf2\x43\xb4\x7f\xd0\xf5\x8c\x15\xda\x63\xdd\x1a\x03\xe0\x9f\xbc\xe5\x41\x7f\x7d\x05\x12\x6b\x62\x99\x26\x83\x28\x10\xbe\xb8\x42\x18\x6a\x47\x6c\x8a\xd9\xdd\x85\x22\xa4\xfb\x4c\xae\x2d\xcc\xdb\x03\x1d\x04\x6b\x85\x3d\xe1\x91\x5e\x2c\xf3\x8e\x5c\xc8\xf7\x29\xc3\x40\x17\x4d\xb1\x5e\x96\xe2\xf9\xd3\x3c\x2c\x61\x82\x1d\x41\x46\x8e\x2c\xef\xf2\x09\xa6\x48\x14\xdd\x41\x3d"[..];
        let dq = &b"\x3d\x1d\xbe\x4d\xc7\x16\xf5\x59\xf3\x11\x89\xa6\xc4\xdb\xe4\xee\x9f\xcc\x3b\x65\x56\xa0\xe6\x0a\xd8\xde\xcb\x5f\x11\xf5\xcd\x05\x53\x8e\x15\x55\x01\x23\xed\x8a\x40\x79\x04\xfc\xbd\xff\x7a\x22\x7d\xe2\x17\xaf\xe3\x76\x20\x98\xd9\x3c\x73\xde\xd8\x95\x79\xea\x8f\x1c\xae\xde\xa8\x3f\xf2\xdc\x34\x0c\x33\xda\xac\xa6\x9b\xc9\xb1\xd2\xd3\x31\x48\x3e\xd3\x3d\x72\x68\x6a\xea\x86\xc2\x78\x57\xa8\xac\x84\xe7\xf1\x64\x27\x36\xaf\xce\x69\x5c\xd2\x46\x89\x0a\x60\xe3\x36\x37\xcf\x1e\x28\xfe\xc8\xfd\x3d\x84\xc2\xe5\x63\xc1"[..];
        let n = &b"\xcc\xc3\x86\x4e\x2b\x3d\x2d\x08\x89\x66\x90\x49\x75\x4f\xb8\x53\xa0\xbe\x04\xd0\x74\x49\x85\xe8\xc7\xf3\x84\x3b\xce\xa0\x12\xc2\x2a\x19\x18\x94\x6e\xda\x69\xb1\x1b\xed\x91\x96\xba\xb2\x58\x0f\x4c\xa6\x5f\xd3\x49\x08\xc5\x40\x58\xe9\xd1\x7a\xb0\x57\xed\x75\xad\xef\x51\x71\x5a\x8d\x9d\x43\x04\x32\x36\xff\xc1\x1f\xf8\x89\x95\x7d\x92\xcd\x72\x60\xb0\xa1\xe6\x20\x63\x1e\x03\x4b\x07\x48\x2d\xd6\xe5\xf6\xad\x30\xa9\x87\x81\x2a\x82\x00\x03\x9f\x53\x05\x71\x11\x65\xf0\xf8\xa8\x6e\xe7\x31\xb8\xd1\xf0\x58\xae\xad\x69\x72\x36\x80\x3b\xdd\x03\x8c\xa6\xc4\xf6\x22\xd7\xd4\x58\x92\xbc\xa0\x26\xd8\xd9\x49\xbe\xfe\x80\x35\x88\x02\xff\x7a\x5a\x40\xc2\xc6\x72\xa9\x3c\xf3\xf0\x61\x60\x45\x3c\xf7\xa1\x96\x66\x54\xde\x9d\xd0\xcb\xeb\xa7\x81\x3b\xda\x29\x0f\xbe\x03\xb0\xfe\xcc\xaf\x19\xdd\xcd\x15\x86\x49\xf8\xc0\xa5\x08\x3b\x2a\x42\x3d\xe2\x00\x33\x6c\x40\x70\x8c\xfc\x4a\x3e\x83\x1e\x7a\x53\x0c\x07\x0f\xc6\xfd\x31\xd5\xf8\x1e\xa7\x73\x47\x67\xf5\x76\x0a\xb1\x02\xa4\xd0\xcb\x79\xb2\xeb\x9c\x93\xa0\x6b\x23\xd2\xa7\x5b\xbb\x40\xa0\x95"[..];
        let e = &b"\x01\x00\x01"[..];

        assert!(PrivateKey::new(d, p, q, None).is_ok());
        assert!(PrivateKey::new(d, p, q, inv).is_ok());
        assert!(PrivateKey::new_crt(dp, dq, p, q, None).is_ok());
        assert!(PrivateKey::new_crt(dp, dq, p, q, inv).is_ok());

        let key1 = PrivateKey::new(d, p, q, None).unwrap();
        assert_eq!(&(*key1.d())[..], d);

        let (p1, q1) = key1.primes();
        assert_eq!(&(*p1)[..], p);
        assert_eq!(&(*q1)[..], q);

        let key2 = PrivateKey::new_crt(dp, dq, p, q, None).unwrap();
        let (dp1, dq1, inv1) = key2.d_crt();
        assert_eq!(&(*dp1)[..], dp);
        assert_eq!(&(*dq1)[..], dq);
        assert_eq!(&(*inv1)[..], inv);

        let (p2, q2) = key2.primes();
        assert_eq!(&(*p2)[..], p);
        assert_eq!(&(*q2)[..], q);

        let pubkey1 = key1.public_key().unwrap();
        assert_eq!(&pubkey1.n()[..], n);
        assert_eq!(&pubkey1.e()[..], e);

        assert!(key2.public_key().is_err());
    }

    #[test]
    fn generate_keypairs() {
        use crate::random::Yarrow;

        let mut rng = Yarrow::default();
        assert!(generate_keypair(&mut rng, 1024).is_ok());
        assert!(generate_keypair(&mut rng, 2048).is_ok());
        assert!(generate_keypair(&mut rng, 4096).is_ok());
    }

    #[test]
    fn clone() {
        use crate::random::Yarrow;

        let mut rng = Yarrow::default();
        let (public, private) = generate_keypair(&mut rng, 1024).unwrap();
        let _ = public.clone();
        let _ = private.clone();
    }
}
