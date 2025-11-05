//! Message authentication codes.

pub mod insecure_do_not_use;

mod mac;
pub use self::mac::Mac;

mod hmac;
pub use self::hmac::Hmac;

mod poly1305;
pub use self::poly1305::Poly1305;
