use std::time::SystemTime;
use nettle::hash::*;
use nettle::hash::insecure_do_not_use::*;

fn benchmark(buf: &[u8], n: usize, h: &mut dyn Hash, name: &str) {
    let mut digest = vec![0; h.digest_size()];
    let start = SystemTime::now();
    for _ in 0..n / buf.len() {
        h.update(buf);
    }
    h.digest(&mut digest);
    let elapsed = start.elapsed().unwrap();
    println!("Hashing {} bytes using {:10.}   {:2.}.{:06.}s   {:9.} byte/s",
             n, name, elapsed.as_secs(), elapsed.subsec_micros(),
             (n as f64 / (elapsed.as_secs() as f64
                          + elapsed.as_micros() as f64 / 1_000_000.)) as usize);
}

fn main() {
    const N: usize = 1024 * 1024 * 1024;
    let buf = vec![0; 1024 * 1024];

    macro_rules! bench {
        ($hash: ident) => {
            benchmark(&buf, N, &mut $hash::default(), stringify!($hash));
        }
    }

    //bench!(Md2); // MD2 is way too slow.
    bench!(Md4);
    bench!(Md5);
    bench!(Ripemd160);
    bench!(GostHash94);
    bench!(Sha1);
    bench!(Sha224);
    bench!(Sha256);
    bench!(Sha384);
    bench!(Sha512);
    bench!(Sha512_224);
    bench!(Sha512_256);
    bench!(Sha3_224);
    bench!(Sha3_256);
    bench!(Sha3_384);
    bench!(Sha3_512);
}
