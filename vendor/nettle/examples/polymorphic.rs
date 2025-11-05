extern crate nettle;

use nettle::aead::{Eax, Gcm};
use nettle::cipher::{Aes128, Twofish};
use nettle::hash::{Hash, Sha224, Sha256};
use nettle::mac::Hmac;
use nettle::mode::{Cbc, Cfb};
use nettle::aead::Aead;
use nettle::mac::Mac;
use nettle::mode::Mode;
use nettle::random::{Random, Yarrow};

struct Fubar {
    foo: Foo,
    bar: Bar,
    baz: Baz,
}

impl Default for Fubar {
    fn default() -> Fubar {
        Fubar { foo: Foo::default(), bar: Bar::default(), baz: Baz::default() }
    }
}

impl Fubar {
    pub fn produce_hash(&self) -> Vec<u8> {
        let mut h = self.init_hash();

        self.foo.hash(&mut h);
        self.bar.hash(&mut h);
        self.baz.hash(&mut h);

        let mut g = h.clone();
        self.foo.hash(&mut g);
        self.bar.hash(&mut g);
        self.baz.hash(&mut g);

        let mut ret = vec![0u8; h.digest_size()];
        h.digest(&mut ret);

        ret
    }

    pub fn produce_mac(&self) -> Vec<u8> {
        let mut h = self.init_mac();

        self.foo.mac(&mut h);
        self.bar.mac(&mut h);
        self.baz.mac(&mut h);

        let mut ret = vec![0u8; h.mac_size()];
        h.digest(&mut ret).unwrap();

        ret
    }

    fn init_hash(&self) -> Box<dyn Hash> {
        if random_bool() {
            Box::new(Sha224::default())
        } else {
            Box::new(Sha256::default())
        }
    }

    fn init_mac(&self) -> Box<dyn Mac> {
        if random_bool() {
            Box::new(Hmac::<Sha224>::with_key(&b"123"[..]))
        } else {
            Box::new(Hmac::<Sha256>::with_key(&b"123"[..]))
        }
    }

    fn init_cipher(&self) -> Box<dyn Mode> {
        if random_bool() {
            Box::new(Cbc::<Aes128>::with_encrypt_key(&b"123"[..]).unwrap())
        } else {
            Box::new(Cfb::<Twofish>::with_encrypt_key(&b"123"[..]).unwrap())
        }
    }

    fn init_aead(&self) -> Box<dyn Aead> {
        if random_bool() {
            Box::new(
                Gcm::<Aes128>::with_key_and_nonce(&b"123"[..], &b"123"[..])
                    .unwrap(),
            )
        } else {
            Box::new(
                Eax::<Twofish>::with_key_and_nonce(&b"123"[..], &b"123"[..])
                    .unwrap(),
            )
        }
    }
}

struct Foo {
    num_foos: usize,
}

impl Default for Foo {
    fn default() -> Foo {
        Foo { num_foos: 42 }
    }
}

impl Foo {
    pub fn hash<H: AsMut<dyn Hash>>(&self, h: &mut H) {
        h.as_mut().update(format!("{}", self.num_foos).as_bytes());
    }

    pub fn mac<H: AsMut<dyn Mac>>(&self, h: &mut H) {
        h.as_mut().update(format!("{}", self.num_foos).as_bytes());
    }
}

struct Bar {
    bar: &'static str,
}

impl Default for Bar {
    fn default() -> Bar {
        Bar { bar: "Hallo, I bims bar vong foo her" }
    }
}

impl Bar {
    pub fn hash<H: AsMut<dyn Hash>>(&self, h: &mut H) {
        h.as_mut().update(self.bar.as_bytes());
    }

    pub fn mac<H: AsMut<dyn Mac>>(&self, h: &mut H) {
        h.as_mut().update(self.bar.as_bytes());
    }
}

enum Baz {
    One,
    Two,
    Three,
}

impl Default for Baz {
    fn default() -> Baz {
        if random_bool() {
            if random_bool() {
                Baz::One
            } else {
                Baz::Two
            }
        } else {
            Baz::Three
        }
    }
}

impl Baz {
    pub fn hash<H: AsMut<dyn Hash>>(&self, h: &mut H) {
        match self {
            &Baz::One => h.as_mut().update(&b"Hello"[..]),
            &Baz::Two => h.as_mut().update(&b"Hallo"[..]),
            &Baz::Three => h.as_mut().update("こにちわ".as_bytes()),
        }
    }

    pub fn mac<H: AsMut<dyn Mac>>(&self, h: &mut H) {
        match self {
            &Baz::One => h.as_mut().update(&b"Hello"[..]),
            &Baz::Two => h.as_mut().update(&b"Hallo"[..]),
            &Baz::Three => h.as_mut().update("こにちわ".as_bytes()),
        }
    }
}

fn main() {
    let fubar = Fubar::default();

    fubar.init_aead();
    fubar.init_cipher();

    println!("Fubar hash: {:?}", fubar.produce_hash());
    println!("Fubar mac: {:?}", fubar.produce_mac());
}

fn random_bool() -> bool {
    let mut b = [0];
    Yarrow::default().random(&mut b);
    b[0] > 127
}
