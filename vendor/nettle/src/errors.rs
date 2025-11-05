#[derive(Debug, thiserror::Error)]
/// Nettle error type.
pub enum Error {
    #[error("invalid argument name: {argument_name}")]
    /// Invalid input argument.
    InvalidArgument {
        /// Name of the invalid argument.
        argument_name: &'static str,
    },
    #[error("signing failed")]
    /// Signing failed
    SigningFailed,
    #[error("encryption failed")]
    /// Encryption failed,
    EncryptionFailed,
    #[error("decryption failed")]
    /// Decryption failed,
    DecryptionFailed,
    #[error("key generation failed")]
    /// Key generation failed,
    KeyGenerationFailed,
    #[error("invalid q_bits and/or p_bits values")]
    /// Invalid q_bits and/or p_bits values.
    InvalidBitSizes,
    #[error("arguments are not on the same EC")]
    /// Arguments are not on the same EC.
    InconsistentCurves,
}

impl Error {
    pub(crate) fn invalid_argument(name: &'static str) -> Self {
        Error::InvalidArgument {
            argument_name: name,
        }
    }
}

/// Specialized Result type.
pub type Result<T> = ::std::result::Result<T, Error>;
