There are three good paths to “`apt install basil`”-style installs on Ubuntu. Here’s a concise, practical menu from fastest → most official:

---

# Option A — Quickest: ship a `.deb` + your own APT repo

Great if you want instant installs without Launchpad/PPAs.

## 1) Build a `.deb` from your Rust project

Use `cargo-deb` (it handles Debian packaging for Rust binaries).

```bash
cargo install cargo-deb
# in your project root (where Cargo.toml is):
cargo deb --target x86_64-unknown-linux-gnu
# (repeat for aarch64 if you want ARM64 builds)
```

Add this to `Cargo.toml` (top-level) so the .deb is nice and complete:

```toml
[package]
name = "basil"
version = "1.2.3"
description = "Basil: a modern BASIC-flavored language and toolchain."
license = "MIT"
authors = ["Your Name <you@example.com>"]
homepage = "https://yobasic.com/basil"
repository = "https://github.com/blackrushllc/basil"

[package.metadata.deb]
maintainer = "Your Name <you@example.com>"
depends = "libc6 (>= 2.31)"
assets = [
  ["target/release/basil", "/usr/bin/basil", "755"],
  ["README.md", "/usr/share/doc/basil/README.md", "644"],
  ["docs/man/basil.1", "/usr/share/man/man1/basil.1", "644"]
]
section = "devel"
priority = "optional"
conflicts = ""
replaces = ""
provides = "basil"
```

> Tip: if your binary is named `basilc` or similar, update the path in `assets`.
> Run `strip target/release/basil` before packaging to reduce size.

## 2) Create and publish an APT repo (signed)

Use **aptly** locally, then host the static repo on any HTTPS host (S3+CloudFront, GitHub Pages, your server).

```bash
# install aptly (on Ubuntu build box)
sudo apt-get update && sudo apt-get install -y aptly gnupg

# create a signing key (one-time)
gpg --quick-generate-key "Basil APT (repo) <repo@yourdomain>" rsa4096 sign 0

# create repo & add your deb(s)
aptly repo create -comment="Basil stable" -distribution="stable" -component="main" basil-stable
aptly repo add basil-stable path/to/basil_1.2.3_amd64.deb
# (add arm64 too, same command)

# publish to filesystem (signed). First export your key ID:
KEYID=$(gpg --list-keys --with-colons | awk -F: '/^pub/ {print last} {last=$5}')
aptly publish repo -architectures="amd64,arm64" -component="main" -distribution="stable" -gpg-key="$KEYID" basil-stable
```

That creates a `public/` folder with `dists/`, `pool/`, `Release`, `InRelease` files. Host `public/` at `https://apt.yourdomain/`.

Export your repo’s public key and host it too:

```bash
gpg --armor --export "$KEYID" > public/basil-archive-keyring.gpg
```

### Users install with:

```bash
# add key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://apt.yourdomain/basil-archive-keyring.gpg | sudo tee /etc/apt/keyrings/basil.gpg >/dev/null

# add source list
echo "deb [signed-by=/etc/apt/keyrings/basil.gpg] https://apt.yourdomain stable main" | sudo tee /etc/apt/sources.list.d/basil.list

sudo apt-get update
sudo apt-get install basil
```

> Release flow: each time you publish a new version, `aptly repo add …` the new `.deb` and re-run `aptly publish update stable`.

---

# Option B — Launchpad PPA (Ubuntu-native, auto-builds for users)

This gives you the classic:

```
sudo add-apt-repository ppa:yourname/basil
sudo apt-get update
sudo apt-get install basil
```

High-level steps:

1. **Launchpad account + GPG key**

    * Create a GPG key, upload the public key to Launchpad.
    * Prove email ownership with that key.

2. **Create a PPA**

    * e.g., `ppa:yourname/basil`.

3. **Add a `debian/` folder using `dh-cargo`**
   Minimal files:

    * `debian/control` (source/binary stanzas; Build-Depends: `debhelper-compat (= 13), dh-cargo, cargo, rustc`)
    * `debian/rules` (short `dh` override that calls `dh $@ --with cargo`)
    * `debian/changelog` (use `dch` to make one, version like `1.2.3-0ppa1`)
    * `debian/copyright`
    * optional: `debian/basil.install`, manpages, bash completions, etc.

   Example `debian/rules`:

   ```make
   #!/usr/bin/make -f
   %:
   	dh $@ --with cargo

   override_dh_auto_configure:
   	dh_auto_configure -- --no-default-features

   override_dh_auto_build:
   	dh_auto_build -- --release

   override_dh_auto_test:
   	# skip or run if you have offline-safe tests
   	:
   ```

   Example `debian/control` (simplified):

   ```
   Source: basil
   Section: devel
   Priority: optional
   Maintainer: Your Name <you@example.com>
   Build-Depends: debhelper-compat (= 13), dh-cargo, cargo, rustc
   Standards-Version: 4.7.0
   Homepage: https://github.com/blackrushllc/basil

   Package: basil
   Architecture: any
   Depends: ${shlibs:Depends}, ${misc:Depends}
   Description: Basil BASIC interpreter/compiler in Rust
    Modern BASIC-flavored language and toolchain.
   ```

4. **Build source package & upload to PPA**

   ```bash
   # ensure all vendored or networkless builds (Launchpad forbids network during build)
   # simplest: commit your Cargo.lock and rely on crates.io snapshotting; dh-cargo will fetch from Debian’s vendor mech.
   debuild -S -sa    # produces .dsc, .orig.tar.gz, .debian.tar.xz
   dput ppa:yourname/basil ../basil_1.2.3-0ppa1_source.changes
   ```

Launchpad will build for supported Ubuntu releases/arches and host your apt repo. Users just `add-apt-repository` and install.

> Pro: Most “Ubuntu-native” feel; multi-arch for free.
> Con: A touch more packaging overhead (the `debian/` dir) and PPA discipline.

---

# Option C — OpenSUSE Build Service (OBS)

OBS can build for Ubuntu and hosts an apt repo for you.

* Connect your Git repo, set Ubuntu targets, define a spec via `debian/` or OBS recipe.
* OBS publishes a signed apt repo with install instructions it generates for your project.

> Pro: cross-distro builds in one place.
> Con: another service to learn.

---

## Which should you pick?

* **You want it live this afternoon:** Option A (cargo-deb + aptly + host on your domain).
* **You want the most Ubuntu-native experience & auto multi-arch:** Option B (PPA).
* **You want many distros supported from one pipeline:** Option C (OBS).

---

## Nice-to-haves (whichever route)

* Provide a **man page** (`docs/man/basil.1`) and maybe **shell completions** (bash/zsh) installed into:

    * `/usr/share/man/man1/`
    * `/usr/share/bash-completion/completions/` and/or `/usr/share/zsh/vendor-completions/`
* Ship a small **`/usr/share/doc/basil/examples/`** directory.
* Run **`lintian`** on your source/binary packages to catch packaging nits.
* Set up **GitHub Actions** to build `.deb`, run tests, and (for Option A) push to your apt repo host.

---

If you tell me which path you want (A, B, or C), I’ll give you a copy-pasteable, project-ready checklist and exact file contents tailored to your repo (including a minimal `debian/` folder if you choose PPA).
