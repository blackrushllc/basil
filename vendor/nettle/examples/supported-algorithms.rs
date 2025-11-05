//! Prints a list of optional algorithms and whether they are
//! supported.

fn main() {
    let (major, minor) = nettle::version();
    println!("Nettle {}.{}", major, minor);
    println!("Cv448: {:?}", nettle::curve448::IS_SUPPORTED);
    println!("OCB: {:?}", nettle::aead::OCB_IS_SUPPORTED);
}
