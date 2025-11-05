use nettle_sys::{
    __gmpz_set, dsa_signature, nettle_dsa_signature_clear,
    nettle_dsa_signature_init, nettle_mpz_set_str_256_u,
};
use std::mem::zeroed;

use crate::helper::convert_gmpz_to_buffer;

/// (EC)DSA signature.
pub struct Signature {
    pub(crate) signature: dsa_signature,
}

impl Signature {
    /// Create a new signature structure.
    ///
    /// Both `r` and `s` need to be big endian integer.
    pub fn new(r: &[u8], s: &[u8]) -> Signature {
        unsafe {
            let mut ret = zeroed();

            nettle_dsa_signature_init(&mut ret);
            nettle_mpz_set_str_256_u(&mut ret.r[0], r.len(), r.as_ptr());
            nettle_mpz_set_str_256_u(&mut ret.s[0], s.len(), s.as_ptr());

            Signature { signature: ret }
        }
    }

    /// Returns `r` as big endian integer.
    pub fn r(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.signature.r[0])
    }

    /// Returns `s` as big endian integer.
    pub fn s(&self) -> Box<[u8]> {
        convert_gmpz_to_buffer(self.signature.s[0])
    }
}

impl Clone for Signature {
    fn clone(&self) -> Self {
        unsafe {
            let mut ret = zeroed();

            nettle_dsa_signature_init(&mut ret);
            __gmpz_set(&mut ret.r[0], &self.signature.r[0]);
            __gmpz_set(&mut ret.s[0], &self.signature.s[0]);

            Signature { signature: ret }
        }
    }
}

impl Drop for Signature {
    fn drop(&mut self) {
        unsafe {
            nettle_dsa_signature_clear(&mut self.signature as *mut _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_signature() {
        let r = &['r' as u8; 12];
        let s = &['s' as u8; 12];
        let sig = Signature::new(r, s);

        assert_eq!(sig.r().as_ref(), r);
        assert_eq!(sig.s().as_ref(), s);
    }

}
