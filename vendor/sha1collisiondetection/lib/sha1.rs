#![allow(non_camel_case_types, non_snake_case)]

use crate::ubc_check::{sha1_dvs, ubc_check, TestT};

pub type __uint32_t = u32; // libc::uint32_t, but that is deprecated.
pub type __uint64_t = u64; // libc::uint64_t, but that is deprecated.
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type collision_block_callback
    =
    Option<unsafe fn(_: uint64_t, _: *const uint32_t,
                                _: *const uint32_t, _: *const uint32_t,
                                _: *const uint32_t) -> ()>;
#[derive(Clone)]
pub struct SHA1_CTX {
    pub total: uint64_t,
    pub ihv: [uint32_t; 5],
    buffer: Buffer,
    pub found_collision: bool,
    pub safe_hash: bool,
    pub detect_coll: bool,
    pub ubc_check: bool,
    pub reduced_round_coll: bool,
    pub callback: collision_block_callback,
    pub ihv1: [uint32_t; 5],
    pub ihv2: [uint32_t; 5],
    pub m1: [uint32_t; 80],
    pub m2: [uint32_t; 80],
    pub states: [[uint32_t; 5]; 80],
}

#[repr(C)]
union Buffer {
    bytes: [u8; 64],
    u32s: [u32; 16],
}

impl Clone for Buffer {
    fn clone(&self) -> Buffer {
        Buffer { bytes: self.as_bytes().clone(), }
    }
}

impl Buffer {
    fn as_bytes(&self) -> &[u8; 64] {
        unsafe {
            &self.bytes
        }
    }

    fn mut_bytes(&mut self) -> &mut [u8; 64] {
        unsafe {
            &mut self.bytes
        }
    }

    fn as_u32s(&self) -> &[u32; 16] {
        unsafe {
            &self.u32s
        }
    }
}

#[inline]
fn rotate_right(x: uint32_t, n: uint32_t) -> uint32_t {
    x.rotate_right(n)
}

#[inline]
fn rotate_left(x: uint32_t, n: uint32_t) -> uint32_t {
    x.rotate_left(n)
}

#[inline]
fn sha1_bswap32(x: uint32_t) -> uint32_t {
    x.swap_bytes()
}

#[inline]
fn maybe_bswap32(x: uint32_t) -> uint32_t {
    if cfg!(target_endian = "big") {
        x
    } else if cfg!(target_endian = "little") {
        sha1_bswap32(x)
    } else {
        unimplemented!()
    }
}
#[inline]
fn sha1_mix(W: &mut [uint32_t; 80], t: usize) -> uint32_t {
    rotate_left(W[t.wrapping_sub(3)] ^
                W[t.wrapping_sub(8)] ^
                W[t.wrapping_sub(14)] ^
                W[t.wrapping_sub(16)],
                1)
}
#[inline]
fn sha1_f1(b: uint32_t, c: uint32_t,
                             d: uint32_t) -> uint32_t {
    return d ^ b & (c ^ d);
}
#[inline]
fn sha1_f2(b: uint32_t, c: uint32_t,
                             d: uint32_t) -> uint32_t {
    return b ^ c ^ d;
}
#[inline]
fn sha1_f3(b: uint32_t, c: uint32_t,
                             d: uint32_t) -> uint32_t {
    return (b & c).wrapping_add(d & (b ^ c));
}
#[inline]
fn sha1_f4(b: uint32_t, c: uint32_t,
                             d: uint32_t) -> uint32_t {
    return b ^ c ^ d;
}
#[inline]
fn hashclash_sha1compress_round1_step(a: uint32_t,
                                                        b: &mut uint32_t,
                                                        c: uint32_t,
                                                        d: uint32_t,
                                                        e: &mut uint32_t,
                                                        m: &[uint32_t; 80],
                                                        t: usize) {
    *e = e.wrapping_add(rotate_left(a, 5).wrapping_add(sha1_f1(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x5a827999).wrapping_add(m[t]));
    *b = rotate_left(*b, 30);
}
#[inline]
fn hashclash_sha1compress_round2_step(a: uint32_t,
                                                        b: &mut uint32_t,
                                                        c: uint32_t,
                                                        d: uint32_t,
                                                        e: &mut uint32_t,
                                                        m: &[uint32_t; 80],
                                                        t: usize) {
    *e = e.wrapping_add(rotate_left(a, 5).wrapping_add(sha1_f2(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x6ed9eba1).wrapping_add(m[t]));
    *b = rotate_left(*b, 30);
}
#[inline]
fn hashclash_sha1compress_round3_step(a: uint32_t,
                                                        b: &mut uint32_t,
                                                        c: uint32_t,
                                                        d: uint32_t,
                                                        e: &mut uint32_t,
                                                        m: &[uint32_t; 80],
                                                        t: usize) {
    *e = e.wrapping_add(rotate_left(a, 5).wrapping_add(sha1_f3(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x8f1bbcdc).wrapping_add(m[t]));
    *b = rotate_left(*b, 30);
}
#[inline]
fn hashclash_sha1compress_round4_step(a: uint32_t,
                                                        b: &mut uint32_t,
                                                        c: uint32_t,
                                                        d: uint32_t,
                                                        e: &mut uint32_t,
                                                        m: &[uint32_t; 80],
                                                        t: usize) {
    *e = e.wrapping_add(rotate_left(a, 5).wrapping_add(sha1_f4(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0xca62c1d6).wrapping_add(m[t]));
    *b = rotate_left(*b, 30);
}
#[inline]
fn hashclash_sha1compress_round1_step_bw(a: uint32_t,
                                                           b: &mut uint32_t,
                                                           c: uint32_t,
                                                           d: uint32_t,
                                                           e: &mut uint32_t,
                                                           m: &[uint32_t; 80],
                                                           t: usize) {
    *b = rotate_right(*b, 30);
    *e = e.wrapping_sub(rotate_left(a, 5).wrapping_add(sha1_f1(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x5a827999).wrapping_add(m[t]));
}
#[inline]
fn hashclash_sha1compress_round2_step_bw(a: uint32_t,
                                                           b: &mut uint32_t,
                                                           c: uint32_t,
                                                           d: uint32_t,
                                                           e: &mut uint32_t,
                                                           m: &[uint32_t; 80],
                                                           t: usize) {
    *b = rotate_right(*b, 30);
    *e = e.wrapping_sub(rotate_left(a, 5).wrapping_add(sha1_f2(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x6ed9eba1).wrapping_add(m[t]));
}
#[inline]
fn hashclash_sha1compress_round3_step_bw(a: uint32_t,
                                                           b: &mut uint32_t,
                                                           c: uint32_t,
                                                           d: uint32_t,
                                                           e: &mut uint32_t,
                                                           m: &[uint32_t; 80],
                                                           t: usize) {
    *b = rotate_right(*b, 30);
    *e = e.wrapping_sub(rotate_left(a, 5).wrapping_add(sha1_f3(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0x8f1bbcdc).wrapping_add(m[t]));
}
#[inline]
fn hashclash_sha1compress_round4_step_bw(a: uint32_t,
                                                           b: &mut uint32_t,
                                                           c: uint32_t,
                                                           d: uint32_t,
                                                           e: &mut uint32_t,
                                                           m: &[uint32_t; 80],
                                                           t: usize) {
    *b = rotate_right(*b, 30);
    *e = e.wrapping_sub(rotate_left(a, 5).wrapping_add(sha1_f4(*b,
                                                                                       c,
                                                                                       d)).wrapping_add(0xca62c1d6).wrapping_add(m[t]));
}
#[inline]
fn sha1compress_full_round1_step_load(a: uint32_t,
                                                        b: &mut uint32_t,
                                                        c: uint32_t,
                                                        d: uint32_t,
                                                        e: &mut uint32_t,
                                                        m: &[uint32_t; 16],
                                                        W: &mut [uint32_t; 80],
                                                        t: usize,
                                                        temp: &mut uint32_t) {
    *temp = maybe_bswap32(m[t]);
    W[t] = *temp;
    *e = e.wrapping_add((*temp).wrapping_add(rotate_left(a, 5)).wrapping_add(sha1_f1(*b,
                                                                                                             c,
                                                                                                             d)).wrapping_add(0x5a827999));
    *b = rotate_left(*b, 30);
}
#[inline]
fn sha1compress_full_round1_step_expand(a: uint32_t,
                                                          b: &mut uint32_t,
                                                          c: uint32_t,
                                                          d: uint32_t,
                                                          e: &mut uint32_t,
                                                          W: &mut [uint32_t; 80],
                                                          t: usize,
                                                          temp: &mut uint32_t) {
    *temp = sha1_mix(W, t);
    W[t] = *temp;
    *e = e.wrapping_add((*temp).wrapping_add(rotate_left(a, 5)).wrapping_add(sha1_f1(*b,
                                                                                                             c,
                                                                                                             d)).wrapping_add(0x5a827999));
    *b = rotate_left(*b, 30);
}
#[inline]
fn sha1compress_full_round2_step(a: uint32_t,
                                                   b: &mut uint32_t,
                                                   c: uint32_t,
                                                   d: uint32_t,
                                                   e: &mut uint32_t,
                                                   W: &mut [uint32_t; 80],
                                                   t: usize,
                                                   temp: &mut uint32_t) {
    *temp = sha1_mix(W, t);
    W[t] = *temp;
    *e = e.wrapping_add((*temp).wrapping_add(rotate_left(a, 5)).wrapping_add(sha1_f2(*b,
                                                                                                             c,
                                                                                                             d)).wrapping_add(0x6ed9eba1));
    *b = rotate_left(*b, 30);
}
#[inline]
fn sha1compress_full_round3_step(a: uint32_t,
                                                   b: &mut uint32_t,
                                                   c: uint32_t,
                                                   d: uint32_t,
                                                   e: &mut uint32_t,
                                                   W: &mut [uint32_t; 80],
                                                   t: usize,
                                                   temp: &mut uint32_t) {
    *temp = sha1_mix(W, t);
    W[t] = *temp;
    *e = e.wrapping_add((*temp).wrapping_add(rotate_left(a, 5)).wrapping_add(sha1_f3(*b,
                                                                                                             c,
                                                                                                             d)).wrapping_add(0x8f1bbcdc));
    *b = rotate_left(*b, 30);
}
#[inline]
fn sha1compress_full_round4_step(a: uint32_t,
                                                   b: &mut uint32_t,
                                                   c: uint32_t,
                                                   d: uint32_t,
                                                   e: &mut uint32_t,
                                                   W: &mut [uint32_t; 80],
                                                   t: usize,
                                                   temp: &mut uint32_t) {
    *temp = sha1_mix(W, t);
    W[t] = *temp;
    *e = e.wrapping_add((*temp).wrapping_add(rotate_left(a, 5)).wrapping_add(sha1_f4(*b,
                                                                                                             c,
                                                                                                             d)).wrapping_add(0xca62c1d6));
    *b = rotate_left(*b, 30);
}
/*BUILDNOCOLLDETECTSHA1COMPRESSION*/
fn sha1_compression_W(ihv: &mut [uint32_t; 5],
                                        W: &[uint32_t; 80]) {
    let mut a: uint32_t = ihv[0];
    let mut b: uint32_t = ihv[1];
    let mut c: uint32_t = ihv[2];
    let mut d: uint32_t = ihv[3];
    let mut e: uint32_t = ihv[4];

    hashclash_sha1compress_round1_step(a, &mut b, c, d, &mut e, W,
                                       0);
    hashclash_sha1compress_round1_step(e, &mut a, b, c, &mut d, W,
                                       1);
    hashclash_sha1compress_round1_step(d, &mut e, a, b, &mut c, W,
                                       2);
    hashclash_sha1compress_round1_step(c, &mut d, e, a, &mut b, W,
                                       3);
    hashclash_sha1compress_round1_step(b, &mut c, d, e, &mut a, W,
                                       4);
    hashclash_sha1compress_round1_step(a, &mut b, c, d, &mut e, W,
                                       5);
    hashclash_sha1compress_round1_step(e, &mut a, b, c, &mut d, W,
                                       6);
    hashclash_sha1compress_round1_step(d, &mut e, a, b, &mut c, W,
                                       7);
    hashclash_sha1compress_round1_step(c, &mut d, e, a, &mut b, W,
                                       8);
    hashclash_sha1compress_round1_step(b, &mut c, d, e, &mut a, W,
                                       9);
    hashclash_sha1compress_round1_step(a, &mut b, c, d, &mut e, W,
                                       10);
    hashclash_sha1compress_round1_step(e, &mut a, b, c, &mut d, W,
                                       11);
    hashclash_sha1compress_round1_step(d, &mut e, a, b, &mut c, W,
                                       12);
    hashclash_sha1compress_round1_step(c, &mut d, e, a, &mut b, W,
                                       13);
    hashclash_sha1compress_round1_step(b, &mut c, d, e, &mut a, W,
                                       14);
    hashclash_sha1compress_round1_step(a, &mut b, c, d, &mut e, W,
                                       15);
    hashclash_sha1compress_round1_step(e, &mut a, b, c, &mut d, W,
                                       16);
    hashclash_sha1compress_round1_step(d, &mut e, a, b, &mut c, W,
                                       17);
    hashclash_sha1compress_round1_step(c, &mut d, e, a, &mut b, W,
                                       18);
    hashclash_sha1compress_round1_step(b, &mut c, d, e, &mut a, W,
                                       19);
    hashclash_sha1compress_round2_step(a, &mut b, c, d, &mut e, W,
                                       20);
    hashclash_sha1compress_round2_step(e, &mut a, b, c, &mut d, W,
                                       21);
    hashclash_sha1compress_round2_step(d, &mut e, a, b, &mut c, W,
                                       22);
    hashclash_sha1compress_round2_step(c, &mut d, e, a, &mut b, W,
                                       23);
    hashclash_sha1compress_round2_step(b, &mut c, d, e, &mut a, W,
                                       24);
    hashclash_sha1compress_round2_step(a, &mut b, c, d, &mut e, W,
                                       25);
    hashclash_sha1compress_round2_step(e, &mut a, b, c, &mut d, W,
                                       26);
    hashclash_sha1compress_round2_step(d, &mut e, a, b, &mut c, W,
                                       27);
    hashclash_sha1compress_round2_step(c, &mut d, e, a, &mut b, W,
                                       28);
    hashclash_sha1compress_round2_step(b, &mut c, d, e, &mut a, W,
                                       29);
    hashclash_sha1compress_round2_step(a, &mut b, c, d, &mut e, W,
                                       30);
    hashclash_sha1compress_round2_step(e, &mut a, b, c, &mut d, W,
                                       31);
    hashclash_sha1compress_round2_step(d, &mut e, a, b, &mut c, W,
                                       32);
    hashclash_sha1compress_round2_step(c, &mut d, e, a, &mut b, W,
                                       33);
    hashclash_sha1compress_round2_step(b, &mut c, d, e, &mut a, W,
                                       34);
    hashclash_sha1compress_round2_step(a, &mut b, c, d, &mut e, W,
                                       35);
    hashclash_sha1compress_round2_step(e, &mut a, b, c, &mut d, W,
                                       36);
    hashclash_sha1compress_round2_step(d, &mut e, a, b, &mut c, W,
                                       37);
    hashclash_sha1compress_round2_step(c, &mut d, e, a, &mut b, W,
                                       38);
    hashclash_sha1compress_round2_step(b, &mut c, d, e, &mut a, W,
                                       39);
    hashclash_sha1compress_round3_step(a, &mut b, c, d, &mut e, W,
                                       40);
    hashclash_sha1compress_round3_step(e, &mut a, b, c, &mut d, W,
                                       41);
    hashclash_sha1compress_round3_step(d, &mut e, a, b, &mut c, W,
                                       42);
    hashclash_sha1compress_round3_step(c, &mut d, e, a, &mut b, W,
                                       43);
    hashclash_sha1compress_round3_step(b, &mut c, d, e, &mut a, W,
                                       44);
    hashclash_sha1compress_round3_step(a, &mut b, c, d, &mut e, W,
                                       45);
    hashclash_sha1compress_round3_step(e, &mut a, b, c, &mut d, W,
                                       46);
    hashclash_sha1compress_round3_step(d, &mut e, a, b, &mut c, W,
                                       47);
    hashclash_sha1compress_round3_step(c, &mut d, e, a, &mut b, W,
                                       48);
    hashclash_sha1compress_round3_step(b, &mut c, d, e, &mut a, W,
                                       49);
    hashclash_sha1compress_round3_step(a, &mut b, c, d, &mut e, W,
                                       50);
    hashclash_sha1compress_round3_step(e, &mut a, b, c, &mut d, W,
                                       51);
    hashclash_sha1compress_round3_step(d, &mut e, a, b, &mut c, W,
                                       52);
    hashclash_sha1compress_round3_step(c, &mut d, e, a, &mut b, W,
                                       53);
    hashclash_sha1compress_round3_step(b, &mut c, d, e, &mut a, W,
                                       54);
    hashclash_sha1compress_round3_step(a, &mut b, c, d, &mut e, W,
                                       55);
    hashclash_sha1compress_round3_step(e, &mut a, b, c, &mut d, W,
                                       56);
    hashclash_sha1compress_round3_step(d, &mut e, a, b, &mut c, W,
                                       57);
    hashclash_sha1compress_round3_step(c, &mut d, e, a, &mut b, W,
                                       58);
    hashclash_sha1compress_round3_step(b, &mut c, d, e, &mut a, W,
                                       59);
    hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, W,
                                       60);
    hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, W,
                                       61);
    hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, W,
                                       62);
    hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, W,
                                       63);
    hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, W,
                                       64);
    hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, W,
                                       65);
    hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, W,
                                       66);
    hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, W,
                                       67);
    hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, W,
                                       68);
    hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, W,
                                       69);
    hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, W,
                                       70);
    hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, W,
                                       71);
    hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, W,
                                       72);
    hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, W,
                                       73);
    hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, W,
                                       74);
    hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, W,
                                       75);
    hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, W,
                                       76);
    hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, W,
                                       77);
    hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, W,
                                       78);
    hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, W,
                                       79);

    let fresh0 = &mut ihv[0];
    *fresh0 = fresh0.wrapping_add(a);
    let fresh1 = &mut ihv[1];
    *fresh1 = fresh1.wrapping_add(b);
    let fresh2 = &mut ihv[2];
    *fresh2 = fresh2.wrapping_add(c);
    let fresh3 = &mut ihv[3];
    *fresh3 = fresh3.wrapping_add(d);
    let fresh4 = &mut ihv[4];
    *fresh4 = fresh4.wrapping_add(e);
}

fn sha1_compression_states(ihv: &mut [uint32_t; 5],
                                                 m: &[uint32_t; 16],
                                                 W: &mut [uint32_t; 80],
                                                 states:
                                                     &mut [[uint32_t; 5]; 80]) {
    let mut a: uint32_t = ihv[0];
    let mut b: uint32_t = ihv[1];
    let mut c: uint32_t = ihv[2];
    let mut d: uint32_t = ihv[3];
    let mut e: uint32_t = ihv[4];

    let mut temp: uint32_t = 0;
    sha1compress_full_round1_step_load(a, &mut b, c, d, &mut e, m,
                                       W,
                                       0, &mut temp);
    sha1compress_full_round1_step_load(e, &mut a, b, c, &mut d, m,
                                       W,
                                       1, &mut temp);
    sha1compress_full_round1_step_load(d, &mut e, a, b, &mut c, m,
                                       W,
                                       2, &mut temp);
    sha1compress_full_round1_step_load(c, &mut d, e, a, &mut b, m,
                                       W,
                                       3, &mut temp);
    sha1compress_full_round1_step_load(b, &mut c, d, e, &mut a, m,
                                       W,
                                       4, &mut temp);
    sha1compress_full_round1_step_load(a, &mut b, c, d, &mut e, m,
                                       W,
                                       5, &mut temp);
    sha1compress_full_round1_step_load(e, &mut a, b, c, &mut d, m,
                                       W,
                                       6, &mut temp);
    sha1compress_full_round1_step_load(d, &mut e, a, b, &mut c, m,
                                       W,
                                       7, &mut temp);
    sha1compress_full_round1_step_load(c, &mut d, e, a, &mut b, m,
                                       W,
                                       8, &mut temp);
    sha1compress_full_round1_step_load(b, &mut c, d, e, &mut a, m,
                                       W,
                                       9, &mut temp);
    sha1compress_full_round1_step_load(a, &mut b, c, d, &mut e, m,
                                       W,
                                       10,
                                       &mut temp);
    sha1compress_full_round1_step_load(e, &mut a, b, c, &mut d, m,
                                       W,
                                       11,
                                       &mut temp);
    sha1compress_full_round1_step_load(d, &mut e, a, b, &mut c, m,
                                       W,
                                       12,
                                       &mut temp);
    sha1compress_full_round1_step_load(c, &mut d, e, a, &mut b, m,
                                       W,
                                       13,
                                       &mut temp);
    sha1compress_full_round1_step_load(b, &mut c, d, e, &mut a, m,
                                       W,
                                       14,
                                       &mut temp);
    sha1compress_full_round1_step_load(a, &mut b, c, d, &mut e, m,
                                       W,
                                       15,
                                       &mut temp);
    sha1compress_full_round1_step_expand(e, &mut a, b, c, &mut d,
                                         W,
                                         16,
                                         &mut temp);
    sha1compress_full_round1_step_expand(d, &mut e, a, b, &mut c,
                                         W,
                                         17,
                                         &mut temp);
    sha1compress_full_round1_step_expand(c, &mut d, e, a, &mut b,
                                         W,
                                         18,
                                         &mut temp);
    sha1compress_full_round1_step_expand(b, &mut c, d, e, &mut a,
                                         W,
                                         19,
                                         &mut temp);
    sha1compress_full_round2_step(a, &mut b, c, d, &mut e,
                                  W,
                                  20, &mut temp);
    sha1compress_full_round2_step(e, &mut a, b, c, &mut d,
                                  W,
                                  21, &mut temp);
    sha1compress_full_round2_step(d, &mut e, a, b, &mut c,
                                  W,
                                  22, &mut temp);
    sha1compress_full_round2_step(c, &mut d, e, a, &mut b,
                                  W,
                                  23, &mut temp);
    sha1compress_full_round2_step(b, &mut c, d, e, &mut a,
                                  W,
                                  24, &mut temp);
    sha1compress_full_round2_step(a, &mut b, c, d, &mut e,
                                  W,
                                  25, &mut temp);
    sha1compress_full_round2_step(e, &mut a, b, c, &mut d,
                                  W,
                                  26, &mut temp);
    sha1compress_full_round2_step(d, &mut e, a, b, &mut c,
                                  W,
                                  27, &mut temp);
    sha1compress_full_round2_step(c, &mut d, e, a, &mut b,
                                  W,
                                  28, &mut temp);
    sha1compress_full_round2_step(b, &mut c, d, e, &mut a,
                                  W,
                                  29, &mut temp);
    sha1compress_full_round2_step(a, &mut b, c, d, &mut e,
                                  W,
                                  30, &mut temp);
    sha1compress_full_round2_step(e, &mut a, b, c, &mut d,
                                  W,
                                  31, &mut temp);
    sha1compress_full_round2_step(d, &mut e, a, b, &mut c,
                                  W,
                                  32, &mut temp);
    sha1compress_full_round2_step(c, &mut d, e, a, &mut b,
                                  W,
                                  33, &mut temp);
    sha1compress_full_round2_step(b, &mut c, d, e, &mut a,
                                  W,
                                  34, &mut temp);
    sha1compress_full_round2_step(a, &mut b, c, d, &mut e,
                                  W,
                                  35, &mut temp);
    sha1compress_full_round2_step(e, &mut a, b, c, &mut d,
                                  W,
                                  36, &mut temp);
    sha1compress_full_round2_step(d, &mut e, a, b, &mut c,
                                  W,
                                  37, &mut temp);
    sha1compress_full_round2_step(c, &mut d, e, a, &mut b,
                                  W,
                                  38, &mut temp);
    sha1compress_full_round2_step(b, &mut c, d, e, &mut a,
                                  W,
                                  39, &mut temp);
    sha1compress_full_round3_step(a, &mut b, c, d, &mut e,
                                  W,
                                  40, &mut temp);
    sha1compress_full_round3_step(e, &mut a, b, c, &mut d,
                                  W,
                                  41, &mut temp);
    sha1compress_full_round3_step(d, &mut e, a, b, &mut c,
                                  W,
                                  42, &mut temp);
    sha1compress_full_round3_step(c, &mut d, e, a, &mut b,
                                  W,
                                  43, &mut temp);
    sha1compress_full_round3_step(b, &mut c, d, e, &mut a,
                                  W,
                                  44, &mut temp);
    sha1compress_full_round3_step(a, &mut b, c, d, &mut e,
                                  W,
                                  45, &mut temp);
    sha1compress_full_round3_step(e, &mut a, b, c, &mut d,
                                  W,
                                  46, &mut temp);
    sha1compress_full_round3_step(d, &mut e, a, b, &mut c,
                                  W,
                                  47, &mut temp);
    sha1compress_full_round3_step(c, &mut d, e, a, &mut b,
                                  W,
                                  48, &mut temp);
    sha1compress_full_round3_step(b, &mut c, d, e, &mut a,
                                  W,
                                  49, &mut temp);
    sha1compress_full_round3_step(a, &mut b, c, d, &mut e,
                                  W,
                                  50, &mut temp);
    sha1compress_full_round3_step(e, &mut a, b, c, &mut d,
                                  W,
                                  51, &mut temp);
    sha1compress_full_round3_step(d, &mut e, a, b, &mut c,
                                  W,
                                  52, &mut temp);
    sha1compress_full_round3_step(c, &mut d, e, a, &mut b,
                                  W,
                                  53, &mut temp);
    sha1compress_full_round3_step(b, &mut c, d, e, &mut a,
                                  W,
                                  54, &mut temp);
    sha1compress_full_round3_step(a, &mut b, c, d, &mut e,
                                  W,
                                  55, &mut temp);
    sha1compress_full_round3_step(e, &mut a, b, c, &mut d,
                                  W,
                                  56, &mut temp);
    sha1compress_full_round3_step(d, &mut e, a, b, &mut c,
                                  W,
                                  57, &mut temp);

    states[58][0] = a;
    states[58][1] = b;
    states[58][2] = c;
    states[58][3] = d;
    states[58][4] = e;

    sha1compress_full_round3_step(c, &mut d, e, a, &mut b,
                                  W,
                                  58, &mut temp);
    sha1compress_full_round3_step(b, &mut c, d, e, &mut a,
                                  W,
                                  59, &mut temp);
    sha1compress_full_round4_step(a, &mut b, c, d, &mut e,
                                  W,
                                  60, &mut temp);
    sha1compress_full_round4_step(e, &mut a, b, c, &mut d,
                                  W,
                                  61, &mut temp);
    sha1compress_full_round4_step(d, &mut e, a, b, &mut c,
                                  W,
                                  62, &mut temp);
    sha1compress_full_round4_step(c, &mut d, e, a, &mut b,
                                  W,
                                  63, &mut temp);
    sha1compress_full_round4_step(b, &mut c, d, e, &mut a,
                                  W,
                                  64, &mut temp);

    states[65][0] = a;
    states[65][1] = b;
    states[65][2] = c;
    states[65][3] = d;
    states[65][4] = e;

    sha1compress_full_round4_step(a, &mut b, c, d, &mut e,
                                  W,
                                  65, &mut temp);
    sha1compress_full_round4_step(e, &mut a, b, c, &mut d,
                                  W,
                                  66, &mut temp);
    sha1compress_full_round4_step(d, &mut e, a, b, &mut c,
                                  W,
                                  67, &mut temp);
    sha1compress_full_round4_step(c, &mut d, e, a, &mut b,
                                  W,
                                  68, &mut temp);
    sha1compress_full_round4_step(b, &mut c, d, e, &mut a,
                                  W,
                                  69, &mut temp);
    sha1compress_full_round4_step(a, &mut b, c, d, &mut e,
                                  W,
                                  70, &mut temp);
    sha1compress_full_round4_step(e, &mut a, b, c, &mut d,
                                  W,
                                  71, &mut temp);
    sha1compress_full_round4_step(d, &mut e, a, b, &mut c,
                                  W,
                                  72, &mut temp);
    sha1compress_full_round4_step(c, &mut d, e, a, &mut b,
                                  W,
                                  73, &mut temp);
    sha1compress_full_round4_step(b, &mut c, d, e, &mut a,
                                  W,
                                  74, &mut temp);
    sha1compress_full_round4_step(a, &mut b, c, d, &mut e,
                                  W,
                                  75, &mut temp);
    sha1compress_full_round4_step(e, &mut a, b, c, &mut d,
                                  W,
                                  76, &mut temp);
    sha1compress_full_round4_step(d, &mut e, a, b, &mut c,
                                  W,
                                  77, &mut temp);
    sha1compress_full_round4_step(c, &mut d, e, a, &mut b,
                                  W,
                                  78, &mut temp);
    sha1compress_full_round4_step(b, &mut c, d, e, &mut a,
                                  W,
                                  79, &mut temp);

    let fresh5 = &mut ihv[0];
    *fresh5 = fresh5.wrapping_add(a);
    let fresh6 = &mut ihv[1];
    *fresh6 = fresh6.wrapping_add(b);
    let fresh7 = &mut ihv[2];
    *fresh7 = fresh7.wrapping_add(c);
    let fresh8 = &mut ihv[3];
    *fresh8 = fresh8.wrapping_add(d);
    let fresh9 = &mut ihv[4];
    *fresh9 = fresh9.wrapping_add(e);
}

fn sha1recompress_fast_58(ihvin: &mut [uint32_t; 5],
                                            ihvout: &mut [uint32_t; 5],
                                            me2: &[uint32_t; 80],
                                            state: &[uint32_t; 5]) {
    let mut a: uint32_t = state[0];
    let mut b: uint32_t = state[1];
    let mut c: uint32_t = state[2];
    let mut d: uint32_t = state[3];
    let mut e: uint32_t = state[4];

        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              57);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              56);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              55);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              54);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              53);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              52);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              51);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              50);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              49);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              48);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              47);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              46);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              45);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              44);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              43);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              42);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              41);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              40);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              39);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              38);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              37);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              36);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              35);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              34);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              33);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              32);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              31);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              30);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              29);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              28);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              27);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              26);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              25);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              24);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              23);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              22);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              21);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              20);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              19);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              18);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              17);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              16);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              15);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              14);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              13);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              12);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              11);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              10);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              9);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              8);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              7);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              6);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              5);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              4);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              3);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              2);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              1);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              0);

    ihvin[0] = a;
    ihvin[1] = b;
    ihvin[2] = c;
    ihvin[3] = d;
    ihvin[4] = e;

    a = state[0];
    b = state[1];
    c = state[2];
    d = state[3];
    e = state[4];

        hashclash_sha1compress_round3_step(c, &mut d, e, a, &mut b, me2,
                                           58);
        hashclash_sha1compress_round3_step(b, &mut c, d, e, &mut a, me2,
                                           59);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           60);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           61);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           62);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           63);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           64);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           65);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           66);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           67);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           68);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           69);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           70);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           71);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           72);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           73);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           74);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           75);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           76);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           77);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           78);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           79);

    ihvout[0] = ihvin[0].wrapping_add(a);
    ihvout[1] = ihvin[1].wrapping_add(b);
    ihvout[2] = ihvin[2].wrapping_add(c);
    ihvout[3] = ihvin[3].wrapping_add(d);
    ihvout[4] = ihvin[4].wrapping_add(e);
}
fn sha1recompress_fast_65(ihvin: &mut [uint32_t; 5],
                                            ihvout: &mut [uint32_t; 5],
                                            me2: &[uint32_t; 80],
                                            state: &[uint32_t; 5]) {
    let mut a: uint32_t = state[0];
    let mut b: uint32_t = state[1];
    let mut c: uint32_t = state[2];
    let mut d: uint32_t = state[3];
    let mut e: uint32_t = state[4];

        hashclash_sha1compress_round4_step_bw(b, &mut c, d, e, &mut a, me2,
                                              64);
        hashclash_sha1compress_round4_step_bw(c, &mut d, e, a, &mut b, me2,
                                              63);
        hashclash_sha1compress_round4_step_bw(d, &mut e, a, b, &mut c, me2,
                                              62);
        hashclash_sha1compress_round4_step_bw(e, &mut a, b, c, &mut d, me2,
                                              61);
        hashclash_sha1compress_round4_step_bw(a, &mut b, c, d, &mut e, me2,
                                              60);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              59);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              58);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              57);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              56);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              55);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              54);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              53);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              52);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              51);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              50);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              49);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              48);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              47);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              46);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              45);
        hashclash_sha1compress_round3_step_bw(b, &mut c, d, e, &mut a, me2,
                                              44);
        hashclash_sha1compress_round3_step_bw(c, &mut d, e, a, &mut b, me2,
                                              43);
        hashclash_sha1compress_round3_step_bw(d, &mut e, a, b, &mut c, me2,
                                              42);
        hashclash_sha1compress_round3_step_bw(e, &mut a, b, c, &mut d, me2,
                                              41);
        hashclash_sha1compress_round3_step_bw(a, &mut b, c, d, &mut e, me2,
                                              40);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              39);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              38);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              37);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              36);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              35);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              34);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              33);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              32);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              31);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              30);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              29);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              28);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              27);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              26);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              25);
        hashclash_sha1compress_round2_step_bw(b, &mut c, d, e, &mut a, me2,
                                              24);
        hashclash_sha1compress_round2_step_bw(c, &mut d, e, a, &mut b, me2,
                                              23);
        hashclash_sha1compress_round2_step_bw(d, &mut e, a, b, &mut c, me2,
                                              22);
        hashclash_sha1compress_round2_step_bw(e, &mut a, b, c, &mut d, me2,
                                              21);
        hashclash_sha1compress_round2_step_bw(a, &mut b, c, d, &mut e, me2,
                                              20);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              19);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              18);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              17);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              16);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              15);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              14);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              13);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              12);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              11);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              10);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              9);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              8);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              7);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              6);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              5);
        hashclash_sha1compress_round1_step_bw(b, &mut c, d, e, &mut a, me2,
                                              4);
        hashclash_sha1compress_round1_step_bw(c, &mut d, e, a, &mut b, me2,
                                              3);
        hashclash_sha1compress_round1_step_bw(d, &mut e, a, b, &mut c, me2,
                                              2);
        hashclash_sha1compress_round1_step_bw(e, &mut a, b, c, &mut d, me2,
                                              1);
        hashclash_sha1compress_round1_step_bw(a, &mut b, c, d, &mut e, me2,
                                              0);

    ihvin[0] = a;
    ihvin[1] = b;
    ihvin[2] = c;
    ihvin[3] = d;
    ihvin[4] = e;

    a = state[0];
    b = state[1];
    c = state[2];
    d = state[3];
    e = state[4];

        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           65);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           66);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           67);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           68);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           69);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           70);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           71);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           72);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           73);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           74);
        hashclash_sha1compress_round4_step(a, &mut b, c, d, &mut e, me2,
                                           75);
        hashclash_sha1compress_round4_step(e, &mut a, b, c, &mut d, me2,
                                           76);
        hashclash_sha1compress_round4_step(d, &mut e, a, b, &mut c, me2,
                                           77);
        hashclash_sha1compress_round4_step(c, &mut d, e, a, &mut b, me2,
                                           78);
        hashclash_sha1compress_round4_step(b, &mut c, d, e, &mut a, me2,
                                           79);

    ihvout[0] = ihvin[0].wrapping_add(a);
    ihvout[1] = ihvin[1].wrapping_add(b);
    ihvout[2] = ihvin[2].wrapping_add(c);
    ihvout[3] = ihvin[3].wrapping_add(d);
    ihvout[4] = ihvin[4].wrapping_add(e);
}
fn sha1_recompression_step(step: TestT,
                                             ihvin: &mut [uint32_t; 5],
                                             ihvout: &mut [uint32_t; 5],
                                             me2: &[uint32_t; 80],
                                             state: &[uint32_t; 5]) {
    match step {
        TestT::Step58 => sha1recompress_fast_58(ihvin, ihvout, me2, state),
        TestT::Step65 => sha1recompress_fast_65(ihvin, ihvout, me2, state),
    };
}
/*
   Because Little-Endian architectures are most common,
   we only set SHA1DC_BIGENDIAN if one of these conditions is met.
   Note that all MSFT platforms are little endian,
   so none of these will be defined under the MSC compiler.
   If you are compiling on a big endian platform and your compiler does not define one of these,
   you will have to add whatever macros your tool chain defines to indicate Big-Endianness.
 */
/*
 * Should detect Big Endian under GCC since at least 4.6.0 (gcc svn
 * rev #165881). See
 * https://gcc.gnu.org/onlinedocs/cpp/Common-Predefined-Macros.html
 *
 * This also works under clang since 3.2, it copied the GCC-ism. See
 * clang.git's 3b198a97d2 ("Preprocessor: add __BYTE_ORDER__
 * predefined macro", 2012-07-27)
 */
/* Not under GCC-alike */
/* Big Endian detection */
/*ENDIANNESS SELECTION*/
/*UNALIGNED ACCESS DETECTION*/
/*FORCE ALIGNED ACCESS*/

/// Processes a block of input data.
///
/// The provided `buf` must be the size of a block, but must not be
/// aligned in any way.
#[inline]
fn sha1_process_unaligned(ctx: &mut SHA1_CTX, buf: &[u8]) {
    debug_assert_eq!(buf.len(), 64);
    ctx.buffer.mut_bytes().copy_from_slice(buf);
    sha1_process(ctx, None);
}

/// Processes a block of input data.
///
/// If `block` is `None`, the block will be read from `ctx.buffer`.
fn sha1_process(ctx: &mut SHA1_CTX, block: Option<&[uint32_t; 16]>) {
    let mut ubc_dv_mask: uint32_t = 0xffffffff;
    let mut ihvtmp: [uint32_t; 5] = [0; 5];
    (*ctx).ihv1[0] =
        (*ctx).ihv[0];
    (*ctx).ihv1[1] =
        (*ctx).ihv[1];
    (*ctx).ihv1[2] =
        (*ctx).ihv[2];
    (*ctx).ihv1[3] =
        (*ctx).ihv[3];
    (*ctx).ihv1[4] =
        (*ctx).ihv[4];
    sha1_compression_states(&mut (*ctx).ihv,
                            block.unwrap_or((*ctx).buffer.as_u32s()),
                            &mut (*ctx).m1,
                            &mut (*ctx).states);
    if (*ctx).detect_coll {
        if (*ctx).ubc_check {
            ubc_check(&(*ctx).m1, &mut ubc_dv_mask);
        }
        if ubc_dv_mask != 0 {
            for dv in sha1_dvs.iter() {
                if ubc_dv_mask & 1 << dv.maskb != 0 {
                    for (m2, (m1, dm)) in ctx.m2.iter_mut()
                        .zip(ctx.m1.iter().zip(dv.dm.iter()))
                    {
                        *m2 = *m1 ^ *dm;
                    }
                    sha1_recompression_step(dv.testt,
                                            &mut (*ctx).ihv2,
                                            &mut ihvtmp,
                                            &(*ctx).m2,
                                            &(*ctx).states[dv.testt.step()]);
                    /* to verify SHA-1 collision detection code with collisions for reduced-step SHA-1 */
                    if 0 == ihvtmp[0] ^
                               (*ctx).ihv[0] |
                               ihvtmp[1] ^
                                   (*ctx).ihv[1] |
                               ihvtmp[2] ^
                                   (*ctx).ihv[2] |
                               ihvtmp[3] ^
                                   (*ctx).ihv[3] |
                               ihvtmp[4] ^
                                   (*ctx).ihv[4] ||
                           (*ctx).reduced_round_coll &&
                               0 == (*ctx).ihv1[0] ^
                                       (*ctx).ihv2[0]
                                       |
                                       (*ctx).ihv1[1]
                                           ^
                                           (*ctx).ihv2[1] |
                                       (*ctx).ihv1[2]
                                           ^
                                           (*ctx).ihv2[2] |
                                       (*ctx).ihv1[3]
                                           ^
                                           (*ctx).ihv2[3] |
                                       (*ctx).ihv1[4]
                                           ^
                                           (*ctx).ihv2[4] {
                        (*ctx).found_collision = true;
                        if (*ctx).safe_hash {
                            sha1_compression_W(&mut (*ctx).ihv,
                                               &(*ctx).m1);
                            sha1_compression_W(&mut (*ctx).ihv,
                                               &(*ctx).m1);
                        }
                        break ;
                    }
                }
            }
        }
    };
}

impl Default for SHA1_CTX {
    /// Initializes the context like SHA1DCInit, but also initializes
    /// the buffers.
    fn default() -> Self {
        SHA1_CTX {
            total: 0,
            ihv: [
                0x67452301,
                0xefcdab89,
                0x98badcfe,
                0x10325476,
                0xc3d2e1f0,
            ],
            buffer: Buffer { bytes: [0; 64] },
            found_collision: false,
            safe_hash: true,
            ubc_check: true,
            detect_coll: true,
            reduced_round_coll: false,
            callback: None,
            ihv1: [0; 5],
            ihv2: [0; 5],
            m1: [0; 80],
            m2: [0; 80],
            states: [[0; 5]; 80],
        }
    }
}

impl SHA1_CTX {
    /// Re-initializes the context like SHA1DCInit.
    pub(crate) fn reset(&mut self) {
        self.total = 0;
        self.ihv = [
            0x67452301,
            0xefcdab89,
            0x98badcfe,
            0x10325476,
            0xc3d2e1f0,
        ];
        self.buffer = Buffer { bytes: [0; 64] };
        self.found_collision = false;
        self.safe_hash = true;
        self.ubc_check = true;
        self.detect_coll = true;
        self.reduced_round_coll = false;
        self.callback = None;
    }

/* **
* Copyright 2017 Marc Stevens <marc@marc-stevens.nl>, Dan Shumow <danshu@microsoft.com>
* Distributed under the MIT Software License.
* See accompanying file LICENSE.txt or copy at
* https://opensource.org/licenses/MIT
***/
/* sha-1 compression function that takes an already expanded message, and additionally store intermediate states */
/* only stores states ii (the state between step ii-1 and step ii) when DOSTORESTATEii is defined in ubc_check.h */
/*
// Function type for sha1_recompression_step_T (uint32_t ihvin[5], uint32_t ihvout[5], const uint32_t me2[80], const uint32_t state[5]).
// Where 0 <= T < 80
//       me2 is an expanded message (the expansion of an original message block XOR'ed with a disturbance vector's message block difference.)
//       state is the internal state (a,b,c,d,e) before step T of the SHA-1 compression function while processing the original message block.
// The function will return:
//       ihvin: The reconstructed input chaining value.
//       ihvout: The reconstructed output chaining value.
*/
/* A callback function type that can be set to be called when a collision block has been found: */
/* void collision_block_callback(uint64_t byteoffset, const uint32_t ihvin1[5], const uint32_t ihvin2[5], const uint32_t m1[80], const uint32_t m2[80]) */
/* The SHA-1 context. */
/* Initialize SHA-1 context. */

    /// Function to enable safe SHA-1 hashing:
    ///
    /// Collision attacks are thwarted by hashing a detected near-collision block 3 times.
    /// Think of it as extending SHA-1 from 80-steps to 240-steps for such blocks:
    ///      The best collision attacks against SHA-1 have complexity about 2^60,
    ///      thus for 240-steps an immediate lower-bound for the best cryptanalytic attacks would be 2^180.
    ///      An attacker would be better off using a generic birthday search of complexity 2^80.
    ///
    /// Enabling safe SHA-1 hashing will result in the correct SHA-1 hash for messages where no collision attack was detected,
    /// but it will result in a different SHA-1 hash for messages where a collision attack was detected.
    /// This will automatically invalidate SHA-1 based digital signature forgeries.
    /// Enabled by default.
    pub fn set_safe_hash(&mut self, v: bool) {
        self.safe_hash = v;
    }

    /// Function to disable or enable the use of Unavoidable Bitconditions (provides a significant speed up).
    ///
    /// Enabled by default
    pub fn set_use_UBC(&mut self, v: bool) {
        self.ubc_check = v;
    }

    /// Function to disable or enable the use of Collision Detection.
    ///
    /// Enabled by default.
    ///
    pub fn set_use_detect_coll(&mut self, v: bool) {
        self.detect_coll = v;
    }

    /// Function to disable or enable the detection of reduced-round
    /// SHA-1 collisions.
    ///
    /// Disabled by default.
    #[allow(dead_code)]
    pub fn set_detect_reduced_round_collision(&mut self, v: bool) {
        self.reduced_round_coll = v;
    }

    /// Function to set a callback function, pass NULL to disable.
    ///
    /// Default no callback set.
    #[allow(dead_code)]
    pub fn set_callback(&mut self, callback: collision_block_callback) {
        self.callback = callback;
    }

    /// Update SHA-1 context with buffer contents.
    pub fn update(&mut self, mut buf: &[u8]) {
        if buf.is_empty() {
            return;
        }

        let mut left = (self.total & 63) as usize;
        let fill = 64usize.wrapping_sub(left);

        // Fill up the buffer and process.
        if left != 0 && buf.len() >= fill {
            self.total = self.total.wrapping_add(fill as u64);
            self.buffer.mut_bytes()[left..left + fill]
                .copy_from_slice(&buf[..fill]);
            sha1_process(self, None);
            buf = &buf[fill..];
            left = 0;
        }

        // If the buffer is aligned to u32 by chance, we don't need to
        // copy the block into our buffer.
        let aligned =
            buf.as_ptr().align_offset(core::mem::align_of::<u32>()) == 0;

        // Process all whole blocks.
        while buf.len() >= 64 {
            self.total = self.total.wrapping_add(64);
            if aligned {
                // Safety: buf is aligned, there are at least 64 bytes
                // available (the size of 16 u32s), the array layout
                // is compatible with the slice layout.  The algorithm
                // takes care of endian conversion when loading the
                // u32s.
                let block = unsafe {
                    core::mem::transmute(buf.as_ptr())
                };
                sha1_process(self, Some(block));
            } else {
                sha1_process_unaligned(self, &buf[..64]);
            }
            buf = &buf[64..];
        }

        // Buffer the rest.
        if buf.len() > 0 {
            assert!(buf.len() < 64 - left);
            self.total = self.total.wrapping_add(buf.len() as u64);
            self.buffer.mut_bytes()[left..left + buf.len()]
                .copy_from_slice(buf);
        };
    }

    const SHA1_PADDING: [u8; 64] =
        [0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// Obtain SHA-1 hash from SHA-1 context.
    ///
    /// Returns: 0 = no collision detected, otherwise = collision
    /// found => warn user for active attack.
    pub fn finalize(&mut self, output: &mut [u8; 20]) -> bool {
        let last = (self.total & 63) as usize;
        let padn = if last < 56 {
            56usize.wrapping_sub(last)
        } else {
            120usize.wrapping_sub(last)
        };

        let mut total: u64;
        self.update(&Self::SHA1_PADDING[..padn]);
        total = self.total.wrapping_sub(padn as u64);
        total <<= 3;
        self.buffer.mut_bytes()[56] = (total >> 56) as u8;
        self.buffer.mut_bytes()[57] = (total >> 48) as u8;
        self.buffer.mut_bytes()[58] = (total >> 40) as u8;
        self.buffer.mut_bytes()[59] = (total >> 32) as u8;
        self.buffer.mut_bytes()[60] = (total >> 24) as u8;
        self.buffer.mut_bytes()[61] = (total >> 16) as u8;
        self.buffer.mut_bytes()[62] = (total >> 8) as u8;
        self.buffer.mut_bytes()[63] = total as u8;
        sha1_process(self, None);

        // Now write out the digest.
        output[0] = (self.ihv[0] >> 24) as u8;
        output[1] = (self.ihv[0] >> 16) as u8;
        output[2] = (self.ihv[0] >> 8) as u8;
        output[3] = self.ihv[0] as u8;
        output[4] = (self.ihv[1] >> 24) as u8;
        output[5] = (self.ihv[1] >> 16) as u8;
        output[6] = (self.ihv[1] >> 8) as u8;
        output[7] = self.ihv[1] as u8;
        output[8] = (self.ihv[2] >> 24) as u8;
        output[9] = (self.ihv[2] >> 16) as u8;
        output[10] = (self.ihv[2] >> 8) as u8;
        output[11] = self.ihv[2] as u8;
        output[12] = (self.ihv[3] >> 24) as u8;
        output[13] = (self.ihv[3] >> 16) as u8;
        output[14] = (self.ihv[3] >> 8) as u8;
        output[15] = self.ihv[3] as u8;
        output[16] = (self.ihv[4] >> 24) as u8;
        output[17] = (self.ihv[4] >> 16) as u8;
        output[18] = (self.ihv[4] >> 8) as u8;
        output[19] = self.ihv[4] as u8;

        self.found_collision
    }
}
