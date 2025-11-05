//! Rust bindings for the [Nettle cryptographic
//! library](https://www.lysator.liu.se/~nisse/nettle/).

#![warn(missing_docs)]

// Allow Struct { field: field }
#![allow(clippy::redundant_field_names)]
// Allow module name repetition, e.g. modname::modname
#![allow(clippy::module_inception)]

extern crate libc;
extern crate nettle_sys;
extern crate getrandom;

mod errors;
pub use crate::errors::{Error, Result};

mod helper;
#[macro_use] mod macros;

pub mod hash;
pub mod cipher;
pub mod mode;
pub mod aead;
pub mod mac;
pub mod kdf;
pub mod rsa;

pub mod random;

pub mod ecc;

pub mod curve25519;
pub mod curve448;
pub mod dsa;
pub mod ecdh;
pub mod ecdsa;
pub mod ed25519;
pub mod ed448;

/// Returns the version of the Nettle library.
///
/// Returns the major and minor version of the Nettle library that is
/// linked to at runtime.
///
/// This information should only be used for display purposes, e.g. in
/// version messages to improve bug reports.  It should not be used to
/// test for features, use the appropriate flags for that
/// (e.g. [`ed448::IS_SUPPORTED`]).
pub fn version() -> (u32, u32) {
    let (major, minor)  = unsafe {
        (nettle_sys::nettle_version_major(),
         nettle_sys::nettle_version_minor())
    };

    // Oddly, Nettle returns signed integers even though version
    // numbers should only be positive.  Further, the size of c_int is
    // platform-dependent.  Use fallible conversion and punt to zero.
    (major.try_into().unwrap_or(0), minor.try_into().unwrap_or(0))
}
