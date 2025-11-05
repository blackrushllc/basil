// Stubs used when Nettle doesn't support OCB.

#[repr(C)]
pub struct ocb_ctx {}

#[repr(C)]
pub struct ocb_key {}

pub const OCB_BLOCK_SIZE: u32 = 16;
pub const OCB_DIGEST_SIZE: u32 = 16;
pub const OCB_MAX_NONCE_SIZE: u32 = 15;

pub unsafe fn nettle_ocb_decrypt(
    _ctx: *mut ocb_ctx,
    _key: *const ocb_key,
    _encrypt_ctx: *const libc::c_void,
    _encrypt: nettle_cipher_func,
    _decrypt_ctx: *const libc::c_void,
    _decrypt: nettle_cipher_func,
    _length: usize,
    _dst: *mut u8,
    _src: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ocb_digest(
    _ctx: *mut ocb_ctx,
    _key: *const ocb_key,
    _cipher: *const libc::c_void,
    _f: nettle_cipher_func,
    _length: usize,
    _digest: *mut u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ocb_encrypt(
    _ctx: *mut ocb_ctx,
    _key: *const ocb_key,
    _cipher: *const libc::c_void,
    _f: nettle_cipher_func,
    _length: usize,
    _dst: *mut u8,
    _src: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ocb_set_key(
    _key: *mut ocb_key,
    _cipher: *const libc::c_void,
    _f: nettle_cipher_func
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ocb_set_nonce(
    _ctx: *mut ocb_ctx,
    _cipher: *const libc::c_void,
    _f: nettle_cipher_func,
    _tag_length: usize,
    _nonce_length: usize,
    _nonce: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}

pub unsafe fn nettle_ocb_update(
    _ctx: *mut ocb_ctx,
    _key: *const ocb_key,
    _cipher: *const libc::c_void,
    _f: nettle_cipher_func,
    _length: usize,
    _data: *const u8
) {
    unimplemented!("This version of Nettle does not support the operation");
}
