### Overview
This update switches Basil’s networking to rustls everywhere and adds a new `portable` feature so `basilc` can be built as a fully static, portable Linux binary using MUSL. It avoids any dependency on system OpenSSL/`libssl` at runtime.

- TLS backends: enforce rustls for `reqwest`, `ureq`, `lettre`, and AWS Rust SDK crates.
- New feature flag: `portable` (on the `basilc` binary crate) to explicitly opt into the MUSL build.
- MUSL target: instructions and CI snippet to produce a `basilc-linux-portable` artifact.

Below are precise changes and validation steps.

---

### Cargo.toml changes (TLS → rustls, add `portable` feature)

#### 1) `basil-objects-net/Cargo.toml` (already rustls)
`reqwest` is already configured correctly: `default-features = false` and `features = ["rustls-tls", …]`. `lettre` already uses `tokio1-rustls-tls`. No change required here.

```toml
# basil-objects-net/Cargo.toml (for reference)
reqwest = { version = "0.12", default-features = false, features = ["json", "gzip", "brotli", "deflate", "stream", "rustls-tls", "multipart"], optional = true }
lettre  = { version = "0.11", default-features = false, features = ["builder", "smtp-transport", "tokio1", "tokio1-rustls-tls", "rustls-native-certs", "ring"], optional = true }
```

#### 2) `basil-objects-aws/Cargo.toml` (force AWS SDK to rustls)
Replace the AWS SDK dependencies to disable defaults and opt into `rustls` explicitly:

```toml
# basil-objects-aws/Cargo.toml
[dependencies]
# ...unchanged...
tokio = { version = "1", features = ["rt-multi-thread", "macros"], default-features = false }

# Force rustls TLS everywhere in AWS crates
aws-config = { version = "1", default-features = false, features = ["behavior-version-latest", "rustls"] }
aws-credential-types = { version = "1" }
aws-types = { version = "1" }
aws-sdk-s3 = { version = "1", optional = true, default-features = false, features = ["rustls"] }
aws-sdk-sesv2 = { version = "1", optional = true, default-features = false, features = ["rustls"] }
aws-sdk-sqs = { version = "1", optional = true, default-features = false, features = ["rustls"] }
serde_json = { version = "1" }
once_cell = { version = "1" }
```

Notes:
- The AWS Rust SDK v1 supports `features = ["rustls"]` and `default-features = false` to avoid `native-tls`.

#### 3) `basil-objects/Cargo.toml` (switch `ureq` to rustls explicitly)
`obj-curl` and `obj-ai` use `ureq`. Ensure `ureq` uses rustls (not native-tls) and does not rely on default features:

```toml
# basil-objects/Cargo.toml
[dependencies]
# ...unchanged...
- ureq = { version = "2.9", optional = true, features = ["json", "tls"] }
+ ureq = { version = "2.9", optional = true, default-features = false, features = ["json", "tls"] }
```

This prevents any inadvertent `native-tls` backend.

#### 4) `basil-objects-sql/Cargo.toml` (already rustls)
`sqlx` is already configured with `tls-rustls` for MySQL/Postgres features. No change needed:

```toml
# basil-objects-sql/Cargo.toml (for reference)
obj-sql-mysql    = ["dep:sqlx", "sqlx/mysql", "sqlx/runtime-tokio", "sqlx/tls-rustls", ...]
obj-sql-postgres = ["dep:sqlx", "sqlx/postgres", "sqlx/runtime-tokio", "sqlx/tls-rustls", ...]
```

#### 5) `basilc/Cargo.toml` (add `portable` feature)
Add an empty `portable` feature on the binary crate so we can target it during MUSL builds:

```toml
# basilc/Cargo.toml
[features]
# ... existing feature forwards ...
obj-all = ["obj-base64", "obj-zip", "obj-curl", "obj-json", "obj-csv", "obj-sqlite", "obj-sql", "obj-bmx", "obj-daw", "obj-ai", "obj-term", "obj-aws", "obj-net", "obj-orm-all"]
+
+# Enables building a fully static, portable Linux binary when used with the MUSL target
+portable = []
```

Notes:
- The `portable` feature itself doesn’t need to toggle code; it’s a build selector you can reference in docs/CI and use to gate any future MUSL-specific tweaks.
- Make sure your release builds of `basilc` don’t enable features that pull in system libraries (e.g., `obj-net-sftp` via `ssh2` links to OpenSSL/libssh2). Keep portable builds limited to rustls-backed features.

---

### Building a fully static MUSL binary

#### One-time setup on Linux
```bash
# Install MUSL toolchain
sudo apt-get update && sudo apt-get install -y musl-tools

# Add the MUSL target
rustup target add x86_64-unknown-linux-musl
```

#### Build command
```bash
# From the workspace root, build the basilc binary only, as a static MUSL executable
cargo build --release -p basilc --target x86_64-unknown-linux-musl --features portable

# Optional: strip symbols to reduce size
strip target/x86_64-unknown-linux-musl/release/basilc

# Rename/copy the artifact as the portable deliverable
target/x86_64-unknown-linux-musl/release/basilc  ->  basilc-linux-portable
```

If you build on macOS or Windows for Linux, consider cross-compiling via Docker (see below) or set up `cross`.

---

### CI: Add a MUSL release artifact (`basilc-linux-portable`)
If you already have GitHub Actions, add a MUSL job. If you don’t, you can create `.github/workflows/build.yml` with the following minimal workflow:

```yaml
name: build
on:
  push:
    branches: [ main ]
  pull_request:

jobs:
  linux-portable:
    name: basilc (linux musl portable)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl

      - name: Install MUSL tools
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      - name: Build (MUSL)
        run: |
          cargo build --release -p basilc --target x86_64-unknown-linux-musl --features portable

      - name: Strip
        run: |
          strip target/x86_64-unknown-linux-musl/release/basilc || true

      - name: Package artifact
        run: |
          cp target/x86_64-unknown-linux-musl/release/basilc basilc-linux-portable

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: basilc-linux-portable
          path: basilc-linux-portable
```

If you already have a workflow, simply add a job like the one above and ensure the artifact is named `basilc-linux-portable`.

---

### Documentation: Add section to `/docs/BUILDING.md`
Create (or update) `docs/BUILDING.md` with a new section "Static Portable Linux Builds":

```markdown
### Static Portable Linux Builds

To produce a single-file, fully static `basilc` binary that runs on stock Linux without `libssl` or `glibc` dependencies, build against MUSL with rustls-backed networking.

1) Install prerequisites (Ubuntu):

```bash
sudo apt-get update && sudo apt-get install -y musl-tools
rustup target add x86_64-unknown-linux-musl
```

2) Build `basilc` for MUSL with the `portable` feature:

```bash
cargo build --release -p basilc --target x86_64-unknown-linux-musl --features portable
```

3) Optional: reduce size by stripping symbols:

```bash
strip target/x86_64-unknown-linux-musl/release/basilc
```

4) Copy/rename the artifact:

```bash
cp target/x86_64-unknown-linux-musl/release/basilc basilc-linux-portable
```

5) Verify the binary is static and does not need libssl:

```bash
ldd basilc-linux-portable || echo "Static binary (not a dynamic executable)"
file basilc-linux-portable
```

6) Run on a clean Ubuntu machine/container (no libssl installed):

```bash
docker run --rm -v "$PWD":/work -w /work ubuntu:24.04 ./basilc-linux-portable --help
```

Notes:
- Avoid enabling features that pull in system libraries for portable builds (e.g., `obj-net-sftp`).
- All HTTP/AWS/SQL clients are configured to use rustls; no system OpenSSL is required.
```

---

### README note (optional)
Add a short note to `README.md` mentioning the portable build:

```markdown
### Portable Linux Binary
We ship a MUSL-based Linux build (`basilc-linux-portable`) that is fully self-contained (static) and does not require system `libssl` or other external libraries. See `docs/BUILDING.md` for how to build it locally.
```

---

### Verification checklist
- Search confirms no `native-tls` crate in `Cargo.lock` for the new build.
- `ureq` uses `rustls` and `reqwest` uses `rustls-tls`.
- AWS crates have `default-features = false` and `features = ["rustls"]`.
- `sqlx` uses `tls-rustls` for Postgres/MySQL.
- `basilc` MUSL binary reports as static (`ldd` says "not a dynamic executable").
- Running on a clean Ubuntu works without `libssl` installed.

---

### Potential pitfalls and tips
- `obj-net-sftp` uses `ssh2` (libssh2) which is commonly linked with OpenSSL; avoid enabling it in portable builds.
- If you add new HTTP/AWS clients, ensure they follow the same pattern: `default-features = false` and enable `rustls` features only.
- If you later need deterministic stripping on CI, prefer `llvm-strip` from `llvm-tools-preview` or keep `strip` as shown.

---

### Summary of deliverables
- Cargo changes:
  - `basil-objects-aws/Cargo.toml`: force rustls on `aws-config` and `aws-sdk-*`.
  - `basil-objects/Cargo.toml`: force rustls on `ureq` and disable default features.
  - `basilc/Cargo.toml`: add `portable = []` feature.
  - (Already OK) `basil-objects-net/Cargo.toml`: `reqwest` uses `rustls-tls`; `lettre` uses rustls.
- Docs: add "Static Portable Linux Builds" section to `docs/BUILDING.md` with MUSL install, target add, build, strip, and verification instructions.
- CI: add/update workflow to publish a MUSL artifact named `basilc-linux-portable`.

With these changes, Basil’s Linux build is portable and avoids any dependency on system OpenSSL libraries.