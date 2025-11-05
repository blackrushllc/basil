//! The Digital Signature Algorithm (DSA) described in FIPS 186.

mod signature;
pub use self::signature::Signature;

mod params;
pub use self::params::Params;

mod sign;
pub use self::sign::{sign, verify};

mod keys;
pub use self::keys::{generate_keypair, PrivateKey, PublicKey};
