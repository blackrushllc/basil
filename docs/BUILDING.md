# Building Basil

This document describes how to build Basil components locally. For a portable Linux binary, see the section below.

## Static Portable Linux Builds

To produce a single-file, fully static `basilc` binary that runs on stock Linux without `libssl` or `glibc` dependencies, build against MUSL with rustls-backed networking.

1) Install prerequisites (Ubuntu):

```
sudo apt-get update && sudo apt-get install -y musl-tools
rustup target add x86_64-unknown-linux-musl
```

2) Build `basilc` for MUSL with the `portable` feature:

```
cargo build --release -p basilc --target x86_64-unknown-linux-musl --features portable
```

3) Optional: reduce size by stripping symbols:

```
strip target/x86_64-unknown-linux-musl/release/basilc
```

4) Copy/rename the artifact:

```
cp target/x86_64-unknown-linux-musl/release/basilc basilc-linux-portable
```

5) Verify the binary is static and does not need libssl:

```
ldd basilc-linux-portable || echo "Static binary (not a dynamic executable)"
file basilc-linux-portable
```

6) Run on a clean Ubuntu machine/container (no libssl installed):

```
docker run --rm -v "$PWD":/work -w /work ubuntu:24.04 ./basilc-linux-portable --help
```

Notes:
- Avoid enabling features that pull in system libraries for portable builds (e.g., `obj-net-sftp`).
- All HTTP/AWS/SQL clients are configured to use rustls; no system OpenSSL is required.
