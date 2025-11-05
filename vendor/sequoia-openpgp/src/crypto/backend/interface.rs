//! The crypto-backend abstraction.

use crate::{
    Error,
    Result,
    crypto::{
        mem::Protected,
        mpi::{MPI, ProtectedMPI},
    },
    types::{Curve, PublicKeyAlgorithm},
};

/// Abstracts over the cryptographic backends.
pub trait Backend: Asymmetric {
    /// Returns a short, human-readable description of the backend.
    ///
    /// This starts with the name of the backend, possibly a version,
    /// and any optional features that are available.  This is meant
    /// for inclusion in version strings to improve bug reports.
    fn backend() -> String;

    /// Fills the given buffer with random data.
    ///
    /// Fills the given buffer with random data produced by a
    /// cryptographically secure pseudorandom number generator
    /// (CSPRNG).  The output may be used as session keys or to derive
    /// long-term cryptographic keys from.
    fn random(buf: &mut [u8]) -> Result<()>;
}

/// Public-key cryptography interface.
pub trait Asymmetric {
    /// Returns whether the given public key cryptography algorithm is
    /// supported by this backend.
    ///
    /// Note: when implementing this function, match exhaustively on
    /// `algo`, do not use a catch-all.  This way, when new algorithms
    /// are introduced, we will see where we may need to add support.
    fn supports_algo(algo: PublicKeyAlgorithm) -> bool;

    /// Returns whether the given elliptic curve is supported by this
    /// backend.
    ///
    /// Note: when implementing this function, match exhaustively on
    /// `curve`, do not use a catch-all.  This way, when new algorithms
    /// are introduced, we will see where we may need to add support.
    fn supports_curve(curve: &Curve) -> bool;

    /// Generates an X25519 key pair.
    ///
    /// Returns a tuple containing the secret and public key.
    fn x25519_generate_key() -> Result<(Protected, [u8; 32])>;

    /// Clamp the X25519 secret key scalar.
    ///
    /// X25519 does the clamping implicitly, but OpenPGP's ECDH over
    /// Curve25519 requires the secret to be clamped.  To increase
    /// compatibility with OpenPGP implementations that do not
    /// implicitly clamp the secrets before use, we do that before we
    /// store the secrets in OpenPGP data structures.
    ///
    /// Note: like every function in this trait, this function expects
    /// `secret` to be in native byte order.
    fn x25519_clamp_secret(secret: &mut Protected) {
        secret[0] &= 0b1111_1000;
        secret[31] &= !0b1000_0000;
        secret[31] |= 0b0100_0000;
    }

    /// Computes the public key for a given secret key.
    fn x25519_derive_public(secret: &Protected) -> Result<[u8; 32]>;

    /// Computes the shared point.
    fn x25519_shared_point(secret: &Protected, public: &[u8; 32])
                           -> Result<Protected>;

    /// Generates an Ed25519 key pair.
    ///
    /// Returns a tuple containing the secret and public key.
    fn ed25519_generate_key() -> Result<(Protected, [u8; 32])>;

    /// Computes the public key for a given secret key.
    fn ed25519_derive_public(secret: &Protected) -> Result<[u8; 32]>;

    /// Creates an Ed25519 signature.
    fn ed25519_sign(secret: &Protected, public: &[u8; 32], digest: &[u8])
                    -> Result<[u8; 64]>;

    /// Verifies an Ed25519 signature.
    fn ed25519_verify(public: &[u8; 32], digest: &[u8], signature: &[u8; 64])
                      -> Result<bool>;

    /// Generates a DSA key pair.
    ///
    /// `p_bits` denotes the desired size of the parameter `p`.
    /// Returns a tuple containing the parameters `p`, `q`, `g`, the
    /// public key `y`, and the secret key `x`.
    fn dsa_generate_key(p_bits: usize)
                        -> Result<(MPI, MPI, MPI, MPI, ProtectedMPI)>;

    /// Generates an ElGamal key pair.
    ///
    /// `p_bits` denotes the desired size of the parameter `p`.
    /// Returns a tuple containing the parameters `p`, `g`, the public
    /// key `y`, and the secret key `x`.
    fn elgamal_generate_key(p_bits: usize)
                            -> Result<(MPI, MPI, MPI, ProtectedMPI)> {
        let _ = p_bits;
        Err(Error::UnsupportedPublicKeyAlgorithm(
            PublicKeyAlgorithm::ElGamalEncrypt).into())
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::backend::{Backend, interface::Asymmetric};

    #[test]
    pub fn ed25519_generate_key_private_and_public_not_equal() {
        let (secret, public) = Backend::ed25519_generate_key().unwrap();
        assert_ne!(secret.as_ref(), public);
    }
}
