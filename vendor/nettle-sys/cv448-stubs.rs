// Stubs used when Nettle doesn't support Curve 448.

pub const CURVE448_SIZE: u32 = 56;
pub const ED448_KEY_SIZE: u32 = 57;
pub const ED448_SIGNATURE_SIZE: u32 = 114;

pub unsafe fn nettle_curve448_mul(
    _q: *mut u8,
    _n: *const u8,
    _p: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_curve448_mul_g(
    _q: *mut u8,
    _n: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ed448_shake256_public_key(
    _pub_: *mut u8,
    _priv_: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ed448_shake256_sign(
    _pub_: *const u8,
    _priv_: *const u8,
    _length: usize,
    _msg: *const u8,
    _signature: *mut u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ed448_shake256_verify(
    _pub_: *const u8,
    _length: usize,
    _msg: *const u8,
    _signature: *const u8
) -> libc::c_int  {
    unimplemented!("This version of Nettle does not support the operation");
}
