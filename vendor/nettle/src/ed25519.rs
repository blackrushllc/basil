//! D.J. Bernstein's "Twisted" Edwards curve Ed25519.

use nettle_sys::{
    nettle_ed25519_sha512_public_key, nettle_ed25519_sha512_sign,
    nettle_ed25519_sha512_verify,
};

use crate::{Error, random::Random, Result};

/// Size of a public or secret Ed25519 key in bytes.
pub const ED25519_KEY_SIZE: usize = ::nettle_sys::ED25519_KEY_SIZE as usize;

/// Size of a Ed25519 signature in bytes.
pub const ED25519_SIGNATURE_SIZE: usize =
    ::nettle_sys::ED25519_SIGNATURE_SIZE as usize;

/// Generates a new Ed25519 private key.
pub fn private_key<R>(rng: &mut R) -> Box<[u8]>
where
    R: Random,
{
    let mut ret = vec![0u8; ED25519_KEY_SIZE].into_boxed_slice();
    rng.random(&mut ret[..]);
    ret
}

/// Computes the `public` key for a given `private` Ed25519 key.
///
/// Fails if one of the buffers is not [`ED25519_KEY_SIZE`] bytes
/// large.
pub fn public_key(public: &mut [u8], private: &[u8]) -> Result<()> {
    if public.len() != ED25519_KEY_SIZE {
        return Err(Error::InvalidArgument { argument_name: "public" });
    }
    if private.len() != ED25519_KEY_SIZE {
        return Err(Error::InvalidArgument { argument_name: "private" });
    }

    unsafe {
        nettle_ed25519_sha512_public_key(public.as_mut_ptr(), private.as_ptr());
    }

    Ok(())
}

/// Signs the message `msg` using the given `public`/`private`,
/// producing `signature`.
///
/// Fails if `public` or `private` is not [`ED25519_KEY_SIZE`] bytes
/// large, or if `signature` is not [`ED25519_SIGNATURE_SIZE`] bytes.
pub fn sign(
    public: &[u8],
    private: &[u8],
    msg: &[u8],
    signature: &mut [u8],
) -> Result<()> {
    if public.len() != ED25519_KEY_SIZE {
        return Err(Error::InvalidArgument { argument_name: "public" });
    }
    if private.len() != ED25519_KEY_SIZE {
        return Err(Error::InvalidArgument { argument_name: "private" });
    }
    if signature.len() != ED25519_SIGNATURE_SIZE {
        return Err(Error::InvalidArgument { argument_name: "signature" });
    }

    unsafe {
        nettle_ed25519_sha512_sign(
            public.as_ptr(),
            private.as_ptr(),
            msg.len(),
            msg.as_ptr(),
            signature.as_mut_ptr(),
        );
    }

    Ok(())
}

/// Verifies `signature` of message `msg` using `public`.
///
/// Returns `true` if the signature is valid.  Fails if `public` is
/// not [`ED25519_KEY_SIZE`] bytes large, or `signature` is not
/// [`ED25519_SIGNATURE_SIZE`] bytes.
pub fn verify(public: &[u8], msg: &[u8], signature: &[u8]) -> Result<bool> {
    if public.len() != ED25519_KEY_SIZE {
        return Err(Error::InvalidArgument { argument_name: "public" });
    }
    if signature.len() != ED25519_SIGNATURE_SIZE {
        return Err(Error::InvalidArgument { argument_name: "signature" });
    }

    unsafe {
        Ok(nettle_ed25519_sha512_verify(
            public.as_ptr(),
            msg.len(),
            msg.as_ptr(),
            signature.as_ptr(),
        ) == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Yarrow;

    #[test]
    fn malformed_public_key() {
        let mut my_public = vec![0u8; ED25519_KEY_SIZE];

        assert!(public_key(&mut my_public, &[0u8; ED25519_KEY_SIZE - 1][..])
            .is_err());
        assert!(public_key(&mut my_public, &[0u8; ED25519_KEY_SIZE + 1][..])
            .is_err());
        assert!(public_key(&mut my_public, &[]).is_err());

        let mut my_public = vec![0u8; ED25519_KEY_SIZE + 1];
        assert!(
            public_key(&mut my_public, &[0u8; ED25519_KEY_SIZE][..]).is_err()
        );
        let mut my_public = vec![0u8; ED25519_KEY_SIZE - 1];
        assert!(
            public_key(&mut my_public, &[0u8; ED25519_KEY_SIZE][..]).is_err()
        );
        let mut my_public = vec![];
        assert!(
            public_key(&mut my_public, &[0u8; ED25519_KEY_SIZE][..]).is_err()
        );
    }

    #[test]
    fn malformed_sign_input() {
        let mut signature = vec![0u8; ED25519_SIGNATURE_SIZE];

        assert!(sign(
            &[0u8; ED25519_KEY_SIZE][..],
            &[0u8; ED25519_KEY_SIZE - 1][..],
            &[0u8; 1][..],
            &mut signature
        )
        .is_err());
        assert!(sign(
            &[0u8; ED25519_KEY_SIZE - 1][..],
            &[0u8; ED25519_KEY_SIZE][..],
            &[0u8; 1][..],
            &mut signature
        )
        .is_err());
        let mut signature = vec![0u8; ED25519_SIGNATURE_SIZE - 1];
        assert!(sign(
            &[0u8; ED25519_KEY_SIZE][..],
            &[0u8; ED25519_KEY_SIZE][..],
            &[0u8; 1][..],
            &mut signature
        )
        .is_err());
    }

    #[test]
    fn malformed_verify_input() {
        assert!(verify(
            &[0u8; ED25519_KEY_SIZE][..],
            &[0u8; 1][..],
            &[0u8; ED25519_SIGNATURE_SIZE - 1][..]
        )
        .is_err());
        assert!(verify(
            &[0u8; ED25519_KEY_SIZE - 1][..],
            &[0u8; 1][..],
            &[0u8; ED25519_SIGNATURE_SIZE][..]
        )
        .is_err());
    }

    #[test]
    fn djb_test_vectors() {
        let mut my_public = vec![0u8; ED25519_KEY_SIZE];
        let mut my_signature = vec![0u8; ED25519_SIGNATURE_SIZE];

        {
            let private = &b"\x9d\x61\xb1\x9d\xef\xfd\x5a\x60\xba\x84\x4a\xf4\x92\xec\x2c\xc4\x44\x49\xc5\x69\x7b\x32\x69\x19\x70\x3b\xac\x03\x1c\xae\x7f\x60"[..];
            let public = &b"\xd7\x5a\x98\x01\x82\xb1\x0a\xb7\xd5\x4b\xfe\xd3\xc9\x64\x07\x3a\x0e\xe1\x72\xf3\xda\xa6\x23\x25\xaf\x02\x1a\x68\xf7\x07\x51\x1a"[..];
            let msg = &b""[..];
            let signature = &b"\xe5\x56\x43\x00\xc3\x60\xac\x72\x90\x86\xe2\xcc\x80\x6e\x82\x8a\x84\x87\x7f\x1e\xb8\xe5\xd9\x74\xd8\x73\xe0\x65\x22\x49\x01\x55\x5f\xb8\x82\x15\x90\xa3\x3b\xac\xc6\x1e\x39\x70\x1c\xf9\xb4\x6b\xd2\x5b\xf5\xf0\x59\x5b\xbe\x24\x65\x51\x41\x43\x8e\x7a\x10\x0b"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x4c\xcd\x08\x9b\x28\xff\x96\xda\x9d\xb6\xc3\x46\xec\x11\x4e\x0f\x5b\x8a\x31\x9f\x35\xab\xa6\x24\xda\x8c\xf6\xed\x4f\xb8\xa6\xfb"[..];
            let public = &b"\x3d\x40\x17\xc3\xe8\x43\x89\x5a\x92\xb7\x0a\xa7\x4d\x1b\x7e\xbc\x9c\x98\x2c\xcf\x2e\xc4\x96\x8c\xc0\xcd\x55\xf1\x2a\xf4\x66\x0c"[..];
            let msg = &b"\x72"[..];
            let signature = &b"\x92\xa0\x09\xa9\xf0\xd4\xca\xb8\x72\x0e\x82\x0b\x5f\x64\x25\x40\xa2\xb2\x7b\x54\x16\x50\x3f\x8f\xb3\x76\x22\x23\xeb\xdb\x69\xda\x08\x5a\xc1\xe4\x3e\x15\x99\x6e\x45\x8f\x36\x13\xd0\xf1\x1d\x8c\x38\x7b\x2e\xae\xb4\x30\x2a\xee\xb0\x0d\x29\x16\x12\xbb\x0c\x00"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\xc5\xaa\x8d\xf4\x3f\x9f\x83\x7b\xed\xb7\x44\x2f\x31\xdc\xb7\xb1\x66\xd3\x85\x35\x07\x6f\x09\x4b\x85\xce\x3a\x2e\x0b\x44\x58\xf7"[..];
            let public = &b"\xfc\x51\xcd\x8e\x62\x18\xa1\xa3\x8d\xa4\x7e\xd0\x02\x30\xf0\x58\x08\x16\xed\x13\xba\x33\x03\xac\x5d\xeb\x91\x15\x48\x90\x80\x25"[..];
            let msg = &b"\xaf\x82"[..];
            let signature = &b"\x62\x91\xd6\x57\xde\xec\x24\x02\x48\x27\xe6\x9c\x3a\xbe\x01\xa3\x0c\xe5\x48\xa2\x84\x74\x3a\x44\x5e\x36\x80\xd7\xdb\x5a\xc3\xac\x18\xff\x9b\x53\x8d\x16\xf2\x90\xae\x67\xf7\x60\x98\x4d\xc6\x59\x4a\x7c\x15\xe9\x71\x6e\xd2\x8d\xc0\x27\xbe\xce\xea\x1e\xc4\x0a"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x0d\x4a\x05\xb0\x73\x52\xa5\x43\x6e\x18\x03\x56\xda\x0a\xe6\xef\xa0\x34\x5f\xf7\xfb\x15\x72\x57\x57\x72\xe8\x00\x5e\xd9\x78\xe9"[..];
            let public = &b"\xe6\x1a\x18\x5b\xce\xf2\x61\x3a\x6c\x7c\xb7\x97\x63\xce\x94\x5d\x3b\x24\x5d\x76\x11\x4d\xd4\x40\xbc\xf5\xf2\xdc\x1a\xa5\x70\x57"[..];
            let msg = &b"\xcb\xc7\x7b"[..];
            let signature = &b"\xd9\x86\x8d\x52\xc2\xbe\xbc\xe5\xf3\xfa\x5a\x79\x89\x19\x70\xf3\x09\xcb\x65\x91\xe3\xe1\x70\x2a\x70\x27\x6f\xa9\x7c\x24\xb3\xa8\xe5\x86\x06\xc3\x8c\x97\x58\x52\x9d\xa5\x0e\xe3\x1b\x82\x19\xcb\xa4\x52\x71\xc6\x89\xaf\xa6\x0b\x0e\xa2\x6c\x99\xdb\x19\xb0\x0c"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x6d\xf9\x34\x0c\x13\x8c\xc1\x88\xb5\xfe\x44\x64\xeb\xaa\x3f\x7f\xc2\x06\xa2\xd5\x5c\x34\x34\x70\x7e\x74\xc9\xfc\x04\xe2\x0e\xbb"[..];
            let public = &b"\xc0\xda\xc1\x02\xc4\x53\x31\x86\xe2\x5d\xc4\x31\x28\x47\x23\x53\xea\xab\xdb\x87\x8b\x15\x2a\xeb\x8e\x00\x1f\x92\xd9\x02\x33\xa7"[..];
            let msg = &b"\x5f\x4c\x89\x89"[..];
            let signature = &b"\x12\x4f\x6f\xc6\xb0\xd1\x00\x84\x27\x69\xe7\x1b\xd5\x30\x66\x4d\x88\x8d\xf8\x50\x7d\xf6\xc5\x6d\xed\xfd\xb5\x09\xae\xb9\x34\x16\xe2\x6b\x91\x8d\x38\xaa\x06\x30\x5d\xf3\x09\x56\x97\xc1\x8b\x2a\xa8\x32\xea\xa5\x2e\xdc\x0a\xe4\x9f\xba\xe5\xa8\x5e\x15\x0c\x07"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\xb7\x80\x38\x1a\x65\xed\xf8\xb7\x8f\x69\x45\xe8\xdb\xec\x79\x41\xac\x04\x9f\xd4\xc6\x10\x40\xcf\x0c\x32\x43\x57\x97\x5a\x29\x3c"[..];
            let public = &b"\xe2\x53\xaf\x07\x66\x80\x4b\x86\x9b\xb1\x59\x5b\xe9\x76\x5b\x53\x48\x86\xbb\xaa\xb8\x30\x5b\xf5\x0d\xbc\x7f\x89\x9b\xfb\x5f\x01"[..];
            let msg = &b"\x18\xb6\xbe\xc0\x97"[..];
            let signature = &b"\xb2\xfc\x46\xad\x47\xaf\x46\x44\x78\xc1\x99\xe1\xf8\xbe\x16\x9f\x1b\xe6\x32\x7c\x7f\x9a\x0a\x66\x89\x37\x1c\xa9\x4c\xaf\x04\x06\x4a\x01\xb2\x2a\xff\x15\x20\xab\xd5\x89\x51\x34\x16\x03\xfa\xed\x76\x8c\xf7\x8c\xe9\x7a\xe7\xb0\x38\xab\xfe\x45\x6a\xa1\x7c\x09"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x78\xae\x9e\xff\xe6\xf2\x45\xe9\x24\xa7\xbe\x63\x04\x11\x46\xeb\xc6\x70\xdb\xd3\x06\x0c\xba\x67\xfb\xc6\x21\x6f\xeb\xc4\x45\x46"[..];
            let public = &b"\xfb\xcf\xbf\xa4\x05\x05\xd7\xf2\xbe\x44\x4a\x33\xd1\x85\xcc\x54\xe1\x6d\x61\x52\x60\xe1\x64\x0b\x2b\x50\x87\xb8\x3e\xe3\x64\x3d"[..];
            let msg = &b"\x89\x01\x0d\x85\x59\x72"[..];
            let signature = &b"\x6e\xd6\x29\xfc\x1d\x9c\xe9\xe1\x46\x87\x55\xff\x63\x6d\x5a\x3f\x40\xa5\xd9\xc9\x1a\xfd\x93\xb7\x9d\x24\x18\x30\xf7\xe5\xfa\x29\x85\x4b\x8f\x20\xcc\x6e\xec\xbb\x24\x8d\xbd\x8d\x16\xd1\x4e\x99\x75\x21\x94\xe4\x90\x4d\x09\xc7\x4d\x63\x95\x18\x83\x9d\x23\x00"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x69\x18\x65\xbf\xc8\x2a\x1e\x4b\x57\x4e\xec\xde\x4c\x75\x19\x09\x3f\xaf\x0c\xf8\x67\x38\x02\x34\xe3\x66\x46\x45\xc6\x1c\x5f\x79"[..];
            let public = &b"\x98\xa5\xe3\xa3\x6e\x67\xaa\xba\x89\x88\x8b\xf0\x93\xde\x1a\xd9\x63\xe7\x74\x01\x3b\x39\x02\xbf\xab\x35\x6d\x8b\x90\x17\x8a\x63"[..];
            let msg = &b"\xb4\xa8\xf3\x81\xe7\x0e\x7a"[..];
            let signature = &b"\x6e\x0a\xf2\xfe\x55\xae\x37\x7a\x6b\x7a\x72\x78\xed\xfb\x41\x9b\xd3\x21\xe0\x6d\x0d\xf5\xe2\x70\x37\xdb\x88\x12\xe7\xe3\x52\x98\x10\xfa\x55\x52\xf6\xc0\x02\x09\x85\xca\x17\xa0\xe0\x2e\x03\x6d\x7b\x22\x2a\x24\xf9\x9b\x77\xb7\x5f\xdd\x16\xcb\x05\x56\x81\x07"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\x3b\x26\x51\x6f\xb3\xdc\x88\xeb\x18\x1b\x9e\xd7\x3f\x0b\xcd\x52\xbc\xd6\xb4\xc7\x88\xe4\xbc\xaf\x46\x05\x7f\xd0\x78\xbe\xe0\x73"[..];
            let public = &b"\xf8\x1f\xb5\x4a\x82\x5f\xce\xd9\x5e\xb0\x33\xaf\xcd\x64\x31\x40\x75\xab\xfb\x0a\xbd\x20\xa9\x70\x89\x25\x03\x43\x6f\x34\xb8\x63"[..];
            let msg = &b"\x42\x84\xab\xc5\x1b\xb6\x72\x35"[..];
            let signature = &b"\xd6\xad\xde\xc5\xaf\xb0\x52\x8a\xc1\x7b\xb1\x78\xd3\xe7\xf2\x88\x7f\x9a\xdb\xb1\xad\x16\xe1\x10\x54\x5e\xf3\xbc\x57\xf9\xde\x23\x14\xa5\xc8\x38\x8f\x72\x3b\x89\x07\xbe\x0f\x3a\xc9\x0c\x62\x59\xbb\xe8\x85\xec\xc1\x76\x45\xdf\x3d\xb7\xd4\x88\xf8\x05\xfa\x08"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }

        {
            let private = &b"\xed\xc6\xf5\xfb\xdd\x1c\xee\x4d\x10\x1c\x06\x35\x30\xa3\x04\x90\xb2\x21\xbe\x68\xc0\x36\xf5\xb0\x7d\x0f\x95\x3b\x74\x5d\xf1\x92"[..];
            let public = &b"\xc1\xa4\x9c\x66\xe6\x17\xf9\xef\x5e\xc6\x6b\xc4\xc6\x56\x4c\xa3\x3d\xe2\xa5\xfb\x5e\x14\x64\x06\x2e\x6d\x6c\x62\x19\x15\x5e\xfd"[..];
            let msg = &b"\x67\x2b\xf8\x96\x5d\x04\xbc\x51\x46"[..];
            let signature = &b"\x2c\x76\xa0\x4a\xf2\x39\x1c\x14\x70\x82\xe3\x3f\xaa\xcd\xbe\x56\x64\x2a\x1e\x13\x4b\xd3\x88\x62\x0b\x85\x2b\x90\x1a\x6b\xc1\x6f\xf6\xc9\xcc\x94\x04\xc4\x1d\xea\x12\xed\x28\x1d\xa0\x67\xa1\x51\x38\x66\xf9\xd9\x64\xf8\xbd\xd2\x49\x53\x85\x6c\x50\x04\x29\x01"[..];

            assert!(public_key(&mut my_public, private).is_ok());
            assert_eq!(public, &my_public[..]);
            assert!(sign(public, private, msg, &mut my_signature).is_ok());
            assert_eq!(signature, &my_signature[..]);
            assert_eq!(verify(public, msg, signature).ok(), Some(true));
        }
    }

    #[test]
    fn smoke_test() {
        let mut rng = Yarrow::default();
        let msg = &b"Hello, World"[..];
        let sk = private_key(&mut rng);

        let mut pk = vec![0u8; ED25519_KEY_SIZE];
        assert!(public_key(&mut pk[..], &sk[..]).is_ok());

        let mut sig = vec![0u8; ED25519_SIGNATURE_SIZE];
        assert!(sign(&pk, &sk, msg, &mut sig).is_ok());
        assert_eq!(verify(&pk, msg, &sig).ok(), Some(true));
    }
}
