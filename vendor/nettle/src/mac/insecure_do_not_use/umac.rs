use nettle_sys::{
    nettle_umac128_digest, nettle_umac128_set_key, nettle_umac128_set_nonce,
    nettle_umac128_update, nettle_umac32_digest, nettle_umac32_set_key,
    nettle_umac32_set_nonce, nettle_umac32_update, nettle_umac64_digest,
    nettle_umac64_set_key, nettle_umac64_set_nonce, nettle_umac64_update,
    nettle_umac96_digest, nettle_umac96_set_key, nettle_umac96_set_nonce,
    nettle_umac96_update, umac128_ctx, umac32_ctx, umac64_ctx, umac96_ctx,
    UMAC128_DIGEST_SIZE, UMAC32_DIGEST_SIZE, UMAC64_DIGEST_SIZE,
    UMAC96_DIGEST_SIZE, UMAC_KEY_SIZE,
};
use std::mem::zeroed;

use crate::{Error, mac::Mac, Result};

/// 32 bit variant of the universal MAC (RFC 4418).
pub struct Umac32 {
    context: umac32_ctx,
}

impl Umac32 {
    /// Creates a new MAC instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != UMAC_KEY_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        unsafe {
            let mut ret: Umac32 = zeroed();

            nettle_umac32_set_key(&mut ret.context as *mut _, key.as_ptr());

            nettle_umac32_set_nonce(
                &mut ret.context as *mut _,
                nonce.len(),
                nonce.as_ptr(),
            );
            Ok(ret)
        }
    }
}

impl Mac for Umac32 {
    fn mac_size(&self) -> usize {
        UMAC32_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_umac32_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) -> Result<()> {
        if digest.len() != self.mac_size() {
            return Err(
                Error::InvalidArgument { argument_name: "digest" }
            );
        }

        unsafe {
            nettle_umac32_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }

        Ok(())
    }
}

/// 64 bit variant of the universal MAC (RFC 4418).
pub struct Umac64 {
    context: umac64_ctx,
}

impl Umac64 {
    /// Creates a new MAC instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != UMAC_KEY_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        unsafe {
            let mut ret: Umac64 = zeroed();

            nettle_umac64_set_key(&mut ret.context as *mut _, key.as_ptr());

            nettle_umac64_set_nonce(
                &mut ret.context as *mut _,
                nonce.len(),
                nonce.as_ptr(),
            );
            Ok(ret)
        }
    }
}

impl Mac for Umac64 {
    fn mac_size(&self) -> usize {
        UMAC64_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_umac64_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) -> Result<()> {
        if digest.len() != self.mac_size() {
            return Err(
                Error::InvalidArgument { argument_name: "digest" }
            );
        }

        unsafe {
            nettle_umac64_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }

        Ok(())
    }
}

/// 96 bit variant of the universal MAC (RFC 4418).
pub struct Umac96 {
    context: umac96_ctx,
}

impl Umac96 {
    /// Creates a new MAC instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != UMAC_KEY_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        unsafe {
            let mut ret: Umac96 = zeroed();

            nettle_umac96_set_key(&mut ret.context as *mut _, key.as_ptr());

            nettle_umac96_set_nonce(
                &mut ret.context as *mut _,
                nonce.len(),
                nonce.as_ptr(),
            );
            Ok(ret)
        }
    }
}

impl Mac for Umac96 {
    fn mac_size(&self) -> usize {
        UMAC96_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_umac96_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) -> Result<()> {
        if digest.len() != self.mac_size() {
            return Err(
                Error::InvalidArgument { argument_name: "digest" }
            );
        }

        unsafe {
            nettle_umac96_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }

        Ok(())
    }
}

/// 128 bit variant of the universal MAC (RFC 4418).
pub struct Umac128 {
    context: umac128_ctx,
}

impl Umac128 {
    /// Creates a new MAC instance with secret `key` and public `nonce`.
    pub fn with_key_and_nonce(key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != UMAC_KEY_SIZE as usize {
            return Err(Error::InvalidArgument { argument_name: "key" });
        }

        unsafe {
            let mut ret: Umac128 = zeroed();

            nettle_umac128_set_key(&mut ret.context as *mut _, key.as_ptr());

            nettle_umac128_set_nonce(
                &mut ret.context as *mut _,
                nonce.len(),
                nonce.as_ptr(),
            );
            Ok(ret)
        }
    }
}

impl Mac for Umac128 {
    fn mac_size(&self) -> usize {
        UMAC128_DIGEST_SIZE as usize
    }

    fn update(&mut self, data: &[u8]) {
        unsafe {
            nettle_umac128_update(
                &mut self.context as *mut _,
                data.len(),
                data.as_ptr(),
            );
        }
    }

    fn digest(&mut self, digest: &mut [u8]) -> Result<()> {
        if digest.len() != self.mac_size() {
            return Err(
                Error::InvalidArgument { argument_name: "digest" }
            );
        }

        unsafe {
            nettle_umac128_digest(
                &mut self.context as *mut _,
                digest.len(),
                digest.as_mut_ptr(),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
    K  = "abcdefghijklmnop"                  // A 16-byte UMAC key
    N  = "bcdefghi"                          // An 8-byte nonce

    The tags generated by UMAC using key K and nonce N are:

    Message      32-bit Tag    64-bit Tag            96-bit Tag
    -------      ----------    ----------            ----------
    <empty>       113145FB  6E155FAD26900BE1  32FEDB100C79AD58F07FF764
    'a' * 3       3B91D102  44B5CB542F220104  185E4FE905CBA7BD85E4C2DC
    'a' * 2^10    599B350B  26BF2F5D60118BD9  7A54ABE04AF82D60FB298C3C
    'a' * 2^15    58DCF532  27F8EF643B0D118D  7B136BD911E4B734286EF2BE
    'a' * 2^20    DB6364D1  A4477E87E9F55853  F8ACFA3AC31CFEEA047F7B11
    'a' * 2^25    5109A660  2E2DBC36860A0A5F  72C6388BACE3ACE6FBF062D9
    'abc' * 1     ABF3A3A0  D4D7B9F6BD4FBFCF  883C3D4B97A61976FFCF2323
    'abc' * 500   ABEB3C8B  D4CF26DDEFD5C01A  8824A260C53C66A36C9260A6
    */
    #[test]
    fn rfc_4418_umac32() {
        let key = &b"abcdefghijklmnop"[..];
        let nonce = &b"bcdefghi"[..];
        let msg1 = &b""[..];
        let msg2 = &b"aaa"[..];
        let msg3 = &vec![b'a'; 0x400][..];
        let msg4 = &vec![b'a'; 0x8000][..];
        let msg5 = &vec![b'a'; 0x10_0000][..];
        let msg6 = &vec![b'a'; 0x200_0000][..];
        let msg7 = &b"abc"[..];

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg1);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x11\x31\x45\xFB"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg2);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x3B\x91\xD1\x02"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg3);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x59\x9B\x35\x0B"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg4);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x58\xDC\xF5\x32"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg5);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\xDB\x63\x64\xD1"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg6);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x85\xEE\x5C\xAE"[..]);
        }

        {
            let mut umac32 = Umac32::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac32.mac_size()];

            umac32.update(msg7);
            umac32.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\xAB\xF3\xA3\xA0"[..]);
        }
    }

    #[test]
    fn rfc_4418_umac64() {
        let key = &b"abcdefghijklmnop"[..];
        let nonce = &b"bcdefghi"[..];
        let msg1 = &b""[..];
        let msg2 = &b"aaa"[..];
        let msg3 = &vec![b'a'; 0x400][..];
        let msg4 = &vec![b'a'; 0x8000][..];
        let msg5 = &vec![b'a'; 0x10_0000][..];
        let msg6 = &vec![b'a'; 0x200_0000][..];
        let msg7 = &b"abc"[..];

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg1);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x6E\x15\x5F\xAD\x26\x90\x0B\xE1"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg2);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x44\xB5\xCB\x54\x2F\x22\x01\x04"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg3);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x26\xBF\x2F\x5D\x60\x11\x8B\xD9"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg4);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\x27\xF8\xEF\x64\x3B\x0D\x11\x8D"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg5);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\xA4\x47\x7E\x87\xE9\xF5\x58\x53"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg6);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\xFA\xCA\x46\xF8\x56\xE9\xB4\x5F"[..]);
        }

        {
            let mut umac64 = Umac64::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac64.mac_size()];

            umac64.update(msg7);
            umac64.digest(&mut mac).unwrap();

            assert_eq!(&mac[..], &b"\xD4\xD7\xB9\xF6\xBD\x4F\xBF\xCF"[..]);
        }
    }

    #[test]
    fn rfc_4418_umac96() {
        let key = &b"abcdefghijklmnop"[..];
        let nonce = &b"bcdefghi"[..];
        let msg1 = &b""[..];
        let msg2 = &b"aaa"[..];
        let msg3 = &vec![b'a'; 0x400][..];
        let msg4 = &vec![b'a'; 0x8000][..];
        let msg5 = &vec![b'a'; 0x10_0000][..];
        let msg6 = &vec![b'a'; 0x200_0000][..];
        let msg7 = &b"abc"[..];

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg1);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\x32\xFE\xDB\x10\x0C\x79\xAD\x58\xF0\x7F\xF7\x64"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg2);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\x18\x5E\x4F\xE9\x05\xCB\xA7\xBD\x85\xE4\xC2\xDC"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg3);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\x7A\x54\xAB\xE0\x4A\xF8\x2D\x60\xFB\x29\x8C\x3C"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg4);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\x7B\x13\x6B\xD9\x11\xE4\xB7\x34\x28\x6E\xF2\xBE"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg5);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\xF8\xAC\xFA\x3A\xC3\x1C\xFE\xEA\x04\x7F\x7B\x11"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg6);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\xA6\x21\xC2\x45\x7C\x00\x12\xE6\x4F\x3F\xDA\xE9"[..]
            );
        }

        {
            let mut umac96 = Umac96::with_key_and_nonce(key, nonce).unwrap();
            let mut mac = vec![0u8; umac96.mac_size()];

            umac96.update(msg7);
            umac96.digest(&mut mac).unwrap();

            assert_eq!(
                &mac[..],
                &b"\x88\x3C\x3D\x4B\x97\xA6\x19\x76\xFF\xCF\x23\x23"[..]
            );
        }
    }
}
