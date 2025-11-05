This crate aims to provide a complete implementation of OpenPGP as
defined by [RFC 4880] as well as some extensions (e.g., [RFC
6637], which describes ECC cryptography for OpenPGP).  This
includes support for unbuffered message processing.

A few features that the OpenPGP community considers to be
deprecated (e.g., version 3 compatibility) have been left out.  We
have also updated some OpenPGP defaults to avoid foot guns (e.g.,
we selected modern algorithm defaults).  If some functionality is
missing, please file a bug report.

A non-goal of this crate is support for any sort of high-level,
bolted-on functionality.  For instance, [RFC 4880] does not define
trust models, such as the web of trust, direct trust, or TOFU.
Neither does this crate.  [RFC 4880] does provide some mechanisms
for creating trust models (specifically, UserID certifications),
and this crate does expose those mechanisms.

We also try hard to avoid dictating how OpenPGP should be used.
This doesn't mean that we don't have opinions about how OpenPGP
should be used in a number of common scenarios (for instance,
message validation).  But, in this crate, we refrain from
expressing those opinions; we will expose an opinionated,
high-level interface in the future.  In order to figure out the
most appropriate high-level interfaces, we look at existing users.
If you are using Sequoia, please get in contact so that we can
learn from your use cases, discuss your opinions, and develop a
high-level interface based on these experiences in the future.

Despite —or maybe because of— its unopinionated nature we found
it easy to develop opinionated OpenPGP software based on Sequoia.

[RFC 4880]: https://tools.ietf.org/html/rfc4880
[RFC 6637]: https://tools.ietf.org/html/rfc6637

# Experimental Features

This crate implements functionality from [RFC 4880bis], notably
AEAD encryption containers.  As of this writing, this RFC is still
a draft and the syntax or semantic defined in it may change or go
away.  Therefore, all related functionality may change and
artifacts created using this functionality may not be usable in
the future.  Do not use it for things other than experiments.

[RFC 4880bis]: https://tools.ietf.org/html/draft-ietf-openpgp-rfc4880bis-08

This crate aims to provide a complete implementation of OpenPGP as
defined by RFC 4880 as well as several extensions (e.g., RFC 6637,
which describes ECC cryptography for OpenPGP, and RFC 4880bis, the
draft of the next OpenPGP standard).  This includes support for
unbuffered message processing.

# Feature flags

This crate uses *features* to enable or disable optional
functionality.  You can tweak the features in your `Cargo.toml` file,
like so:

```toml
sequoia-openpgp = { version = "*", default-features = false, features = ["compression", ...] }
```

By default, Sequoia is built using Nettle as cryptographic backend
with all compression algorithms enabled.  Using the default features
is only appropriate for leaf crates, see [this section].

[this section]: #how-to-select-crypto-backends-in-crates

Note that if you use `default-features = false`, you need to
explicitly enable a crypto backend, and also enable compression
features.

## Crypto backends

Sequoia supports multiple cryptographic libraries that can be selected
at compile time.  Currently, these libraries are available:

  - The Nettle cryptographic library.  This is the default backend,
    and is selected by the default feature set.  If you use
    `default-features = false`, you need to explicitly include
    the `crypto-nettle` feature to enable it.

  - The OpenSSL backend.  To select this backend, use
    `default-features = false`, and explicitly include the
    `crypto-openssl` feature to enable it.

  - The Botan backend.  To select this backend, use
    `default-features = false`, and explicitly include the
    `crypto-botan` feature to enable it.  `crypto-botan` defaults to
    Botan v3, which was release in April 2023.  Use `crypto-botan2` to
    use v2.

  - The Windows Cryptography API: Next Generation (CNG).  To select
    this backend, use `default-features = false`, and explicitly
    include the `crypto-cng` feature to enable it.  Currently, the CNG
    backend requires at least Windows 10.

  - The RustCrypto crates.  To select this backend, use
    `default-features = false`, and explicitly include the
    `crypto-rust` feature to enable it.  As of this writing, the
    RustCrypto crates are not recommended for general use as they
    cannot offer the same security guarantees as more mature
    cryptographic libraries.

### Experimental and variable-time cryptographic backends

Some cryptographic backends are not yet considered mature enough for
general consumption.  The use of such backends requires explicit
opt-in using the feature flag `allow-experimental-crypto`.

Some cryptographic backends can not guarantee that cryptographic
operations require a constant amount of time.  This may leak secret
keys in some settings.  The use of such backends requires explicit
opt-in using the feature flag `allow-variable-time-crypto`.

### How to select crypto backends in crates

In Rust, [features are unified], and consequently features should be
additive, i.e. it should be safe to enable any combination of
features.  But, this does not hold for crypto backends, because
exactly one cryptographic backend has to be selected in order to
compile Sequoia.

[features are unified]: https://doc.rust-lang.org/cargo/reference/features.html#feature-unification

To accommodate this, we came up with the following rule: in any
project using Sequoia, exactly one crate may select the cryptographic
backend, and that crate is the leaf crate (i.e. the binary or cdylib
crate).  Any non-leaf, library crate must refrain from selecting a
crypto backend, including the default one, by disabling the default
features.

To recap, follow these rules depending on what kind of crate you are
developing:

#### Leaf crate

The leaf crate should pick a default backend (you may defer to Sequoia
to pick the default one), but should allow your downstream users to
switch backends:

```toml
# Cargo.toml
[dependencies]
sequoia-openpgp = { version = "*", default-features = false }
# If you need compression features, enable them here:
# sequoia-openpgp = { version = "*", default-features = false, features = ["compression"] }

[features]
# Pick a crypto backend enabled by default (here we defer to Sequoia
# to pick the default):
default = ["sequoia-openpgp/default"]

# .. but allow others to select a different backend, as well
crypto-nettle = ["sequoia-openpgp/crypto-nettle"]
crypto-openssl = ["sequoia-openpgp/crypto-openssl"]
crypto-botan = ["sequoia-openpgp/crypto-botan"]
crypto-botan2 = ["sequoia-openpgp/crypto-botan2"]
crypto-rust = ["sequoia-openpgp/crypto-rust"]
crypto-cng = ["sequoia-openpgp/crypto-cng"]

# Experimental and variable-time cryptographic backend opt-ins
allow-experimental-crypto = ["sequoia-openpgp/allow-experimental-crypto"]
allow-variable-time-crypto = ["sequoia-openpgp/allow-variable-time-crypto"]
```

#### Intermediate crate

Non-leaf crates must not select a cryptographic backend, and must
disable the default features.  Additionally, to make `cargo test` work
without having to select a crypto backend, and to enable `docs.rs` to
build your documentation, do selectively enable crypto backends for
those cases:

```toml
# Cargo.toml
[dependencies]
sequoia-openpgp = { version = "*", default-features = false }
# If you need compression features, enable them here:
# sequoia-openpgp = { version = "*", default-features = false, features = ["compression"] }

# Enables a crypto backend for the tests:
[target.'cfg(not(windows))'.dev-dependencies]
sequoia-openpgp = { version = "1", default-features = false, features = ["crypto-nettle", "__implicit-crypto-backend-for-tests"]  }

# Enables a crypto backend for the tests:
[target.'cfg(windows)'.dev-dependencies]
sequoia-openpgp = { version = "1", default-features = false, features = ["crypto-cng", "__implicit-crypto-backend-for-tests"] }

# Enables a crypto backend for the docs.rs generation:
[package.metadata.docs.rs]
features = ["sequoia-openpgp/default"]
```

## Compression algorithms

Use the `compression` flag to enable support for all compression
algorithms, `compression-deflate` to enable *DEFLATE* and *zlib*
compression support, and `compression-bzip2` to enable *bzip2*
support.

# Compiling to WASM

With the right feature flags, Sequoia can be compiled to WASM.  To do
that, enable the RustCrypto backend, and make sure not to enable
*bzip2* compression support:

```toml
sequoia-openpgp = { version = "*", default-features = false, features = ["crypto-rust", "allow-experimental-crypto", "allow-variable-time-crypto"] }
```

Or, with `compression-deflate` support:

```toml
sequoia-openpgp = { version = "*", default-features = false, features = ["crypto-rust", "allow-experimental-crypto", "allow-variable-time-crypto", "compression-deflate"] }
```

# Minimum Supported Rust Version (MSRV)

`sequoia-openpgp` requires Rust 1.67.
