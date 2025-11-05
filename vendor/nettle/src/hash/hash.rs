use nettle_sys::nettle_hash;

/// Hash function.
///
/// Cryptographic hash functions compute a fixed length checksum (also called digest) from variable length data.
pub trait Hash {
    /// Size of the digest in bytes
    fn digest_size(&self) -> usize;

    /// Writes data into the hash function.
    fn update(&mut self, data: &[u8]);

    /// Finalizes the hash function and writes the digest into the provided slice.
    ///
    /// `digest` must be at least DIGEST_SIZE bytes large, otherwise
    /// the digest will be truncated. Resets the hash function
    /// contexts.
    fn digest(&mut self, digest: &mut [u8]);

    /// Clones the hash context into a Box.
    fn box_clone(&self) -> Box<dyn Hash>;
}

impl_write_for_hash!(dyn Hash);

/// Nettle context a.k.a. `<hash>_ctx` of this hash
pub trait NettleHash: Hash + Default {
    #[doc(hidden)]
    type Context: Sized;

    /// Pointer to the `nettle_hash` structure for this hash.
    unsafe fn nettle_hash() -> &'static nettle_hash;
}

impl Hash for Box<dyn Hash> {
    fn digest_size(&self) -> usize {
        self.as_ref().digest_size()
    }

    fn update(&mut self, data: &[u8]) {
        self.as_mut().update(data)
    }

    fn digest(&mut self, digest: &mut [u8]) {
        self.as_mut().digest(digest)
    }

    fn box_clone(&self) -> Box<dyn Hash> {
        self.as_ref().box_clone()
    }
}

impl Clone for Box<dyn Hash> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::{Hash, Sha224, Sha256, Sha3_512};

    #[test]
    fn polymorphic_one_shot() {
        fn hash_with<H: Hash + Default>(data: &[u8]) -> Vec<u8> {
            let mut h = H::default();
            let mut ret = vec![0u8; h.digest_size()];

            h.update(data);
            h.digest(&mut ret);

            ret
        }

        let test = &b"test123"[..];

        hash_with::<Sha224>(test);
        hash_with::<Sha3_512>(test);
    }

    #[test]
    fn polymorphic_multiple() {
        fn hash_with<H: Hash>(hash: &mut H, data: &[u8]) {
            hash.update(data);
        }

        let test = &b"test123"[..];

        {
            let mut h = Sha224::default();
            let mut ret = vec![0u8; h.digest_size()];

            hash_with(&mut h, test);
            h.digest(&mut ret);
        }

        {
            let mut h = Sha3_512::default();
            let mut ret = vec![0u8; h.digest_size()];

            hash_with(&mut h, test);
            h.digest(&mut ret);
        }
    }

    #[test]
    fn write_trait() {
        use std::io::Write;

        let mut h0 = Sha256::default();
        let mut d0 = [0u8; 256 / 8];
        let mut h1: Box<dyn Hash> = Box::new(Sha256::default());
        let mut d1 = [0u8; 256 / 8];
        let mut h2 = Sha256::default();
        let mut d2 = [0u8; 256 / 8];

        fn hash<H: Write + Hash>(hash: &mut H) {
            hash.write_all(&b"test123"[..]).unwrap();
        }
        hash(&mut h0);
        hash(&mut h1);
        h2.update(&b"test123"[..]);

        h0.digest(&mut d0);
        h1.digest(&mut d1);
        h2.digest(&mut d2);

        assert_eq!(d0, d2);
        assert_eq!(d1, d2);
    }
}
