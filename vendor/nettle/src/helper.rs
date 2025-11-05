use crate::{Error, Result};
use nettle_sys::{
    __gmpz_clear, __mpz_struct, nettle_mpz_get_str_256,
    nettle_mpz_init_set_str_256_u, nettle_mpz_sizeinbase_256_u,
};
use std::mem::zeroed;

pub fn convert_gmpz_to_buffer(mut mpz: __mpz_struct) -> Box<[u8]> {
    unsafe {
        let mut ret = vec![0u8; nettle_mpz_sizeinbase_256_u(&mut mpz)];

        nettle_mpz_get_str_256(ret.len(), ret.as_mut_ptr(), &mut mpz);

        while ret.len() > 1 && ret[0] == 0 {
            ret.remove(0);
        }
        ret.into()
    }
}

/// Convert the buffer to an mpz number.
///
/// This inits the mpz number. Take care not to init twice!
pub fn convert_buffer_to_gmpz(buf: &[u8]) -> __mpz_struct {
    unsafe {
        let mut ret = zeroed();

        nettle_mpz_init_set_str_256_u(&mut ret, buf.len(), buf.as_ptr());
        ret
    }
}

pub fn write_gmpz_into_slice(
    mut mpz: __mpz_struct,
    slice: &mut [u8],
    arg: &'static str,
) -> Result<()> {
    unsafe {
        if nettle_mpz_sizeinbase_256_u(&mut mpz as *mut _) > slice.len() {
            __gmpz_clear(&mut mpz);

            Err(Error::InvalidArgument { argument_name: arg })
        } else {
            nettle_mpz_get_str_256(slice.len(), slice.as_mut_ptr(), &mut mpz);
            __gmpz_clear(&mut mpz);

            Ok(())
        }
    }
}
