//! Self-contained random number support for tests.

use crate::random::{Random, Yarrow};

/// Random numbers for use in tests.
pub trait TestRandom {
    /// Produce a random `u64`.
    fn next_u64(&mut self) -> u64;

    /// Produce a random `u32`.
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as _
    }

    /// Produce a random `usize`.
    fn next_usize(&mut self) -> usize {
        self.next_u64() as _
    }
}

impl TestRandom for Yarrow {
    fn next_u64(&mut self) -> u64 {
        let mut r = [0u8; 8];
        self.random(&mut r[..]);
        0
            | u64::from(r[0]) << 0
            | u64::from(r[1]) << 8
            | u64::from(r[2]) << 16
            | u64::from(r[3]) << 24
            | u64::from(r[4]) << 32
            | u64::from(r[5]) << 40
            | u64::from(r[6]) << 48
            | u64::from(r[7]) << 56
    }
}
