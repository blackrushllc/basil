# For Users - INSTALLING
```
sudo add-apt-repository ppa:blackrush/basil
sudo apt update
apt policy basil          # confirm version, pocket, priority
sudo apt install basil
basilc --version
bcc --version
basil-serve --help

# examples/docs
dpkg -L basil | grep /usr/share/doc/basil
```

# For Basil Devs - Publishing Basil to a PPA (Ubuntu/Debian)

This is the repeatable checklist to ship a new Basil release to the `ppa:blackrush/basil` Launchpad PPA.

> TL;DR: bump version, ensure clean tree (no `dist/`, no generated examples), regenerate the upstream tarball, build a **signed source** package, upload to PPA.

---

## 0) One-time setup (already done; keep for reference)

```bash
# Packaging tools
sudo apt update
sudo apt install devscripts debhelper dput dpkg-dev fakeroot lintian \
                 dh-cargo cargo rustc pkgconf libssl-dev

# GPG identity (must match Launchpad GPG key)
export DEBFULLNAME="Blackrush LLC"
export DEBEMAIL="erikolson1965@gmail.com"
gpg --list-secret-keys --keyid-format=long
export DEBSIGN_KEYID=<YOUR_LONG_KEY_ID>   # e.g., ABCDEF1234567890

# In repo root, ensure these exist (we already added them):
# - debian/ with control, rules, changelog, source/format, postinst
# - .gitattributes with export-ignore rules
```

**Files you rely on:**

* `debian/rules` — builds `basilc` with `--features obj-all`, installs binaries/docs/examples.
* `debian/control` — `debhelper-compat (= 13)`, `dh-cargo`, `pkgconf`, etc.
* `.gitattributes` — excludes `dist/` and generated examples (`*.basilx`, caches) from the upstream tarball.
* (Optional) `debian/source/options` — ignores stray generated files if left around.

---

## 1) Version bump

Set the new version (example: `1.1.1`), and add a changelog entry targeting your Ubuntu series (e.g., `noble`):

```bash
dch -v 1.1.1-0ubuntu1 -D noble "New upstream release."
```

> Keep the `-0ubuntu1` Debian revision; increase it if you respin without changing upstream version.

---

## 2) Ensure tree is clean (no prebuilt artifacts)

You must **not** have prebuilt binaries or generated examples in the working tree when building the source package.

```bash
# Remove prebuilt artifacts
rm -rf dist

# If you’re comfortable removing ALL untracked files:
# git clean -xfd    # ⚠️ irreversible

# Sanity (should print nothing):
find dist -type f || echo "OK (no dist files)"
```

> `.gitattributes` already prevents these from going into the upstream tarball; this step ensures the **working tree** also doesn’t contain them.

---

## 3) Rebuild the upstream “orig” tarball

This creates `../basil_<VERSION>.orig.tar.gz` from the clean Git tree.

```bash
VERSION=1.1.1
rm -f ../basil_${VERSION}.orig.tar.gz
git archive --format=tar --prefix="basil-$VERSION/" HEAD \
  | gzip -n > "../basil_${VERSION}.orig.tar.gz"

# sanity: must NOT list anything under dist/
tar tzf ../basil_${VERSION}.orig.tar.gz | grep '^basil-'"$VERSION"'/dist/' || echo "OK (dist/ not in orig)"
```

---

## 4) Build the signed **source** package

```bash
# Make sure the build-deps are installed (see section 0)
debuild -S -sa -k"$DEBSIGN_KEYID"
```

Expected outputs in parent dir (`..`):

* `basil_<VER>.orig.tar.gz`
* `basil_<VER>-0ubuntu1.dsc`
* `basil_<VER>-0ubuntu1.debian.tar.xz`
* `basil_<VER>-0ubuntu1_source.changes`

Optional lint pass:

```bash
lintian -i ../basil_<VER>-0ubuntu1_source.changes
```

---

## 5) Upload to the PPA

```bash
PPA=ppa:blackrush/basil
dput "$PPA" ../basil_${VERSION}-0ubuntu1_source.changes
```

Then watch the build on Launchpad. When it’s published:

```bash
sudo add-apt-repository ppa:blackrush/basil
sudo apt update
sudo apt install basil

basilc --version
bcc --version
basil-serve --help
dpkg -L basil | grep /usr/share/doc/basil
```

---

## Notes & reminders

* **Series**: target `noble` (24.04 LTS) unless you need others. Repeat for `jammy` if desired by creating a changelog stanza with `-D jammy`.
* **No PATH hacks**: binaries install to `/usr/bin` (already in PATH).
* **Examples location**: `/usr/share/doc/basil/examples` per Debian policy (you can copy to `$HOME` later).
* **Post-install message**: `debian/postinst` prints the “What’s New” URL (doesn’t auto-open browser).
* **Features**: `basilc` is built with `--features obj-all` in `debian/rules`:

  ```make
  override_dh_auto_build:
      dh_auto_build -- -p basilc --release --features obj-all
      cargo build -p bcc --release
      cargo build -p basil-serve --release
  ```

---

## Common errors (fast fixes)

* **“cannot represent change to dist/… binary file contents changed”**
  → Remove `dist/` from working tree **and** rebuild the orig tarball (Steps 2–3).
  Optional safety: `debian/source/options` with `extend-diff-ignore = ^dist/`.

* **“no appropriate original tar file … expected basil_<VER>.orig.tar.gz”**
  → Step 3: (re)generate the orig tarball.

* **“UNRELEASED” in dpkg-buildpackage**
  → Step 1: `dch -r -D noble "PPA build."`

* **Signing error: “No secret key”**
  → Import or create a GPG key, set `DEBSIGN_KEYID`, upload the public key to Launchpad.

* **Lint: pkg-config obsolete**
  → `debian/control`: use `pkgconf` instead of `pkg-config`.

---

## Quick full run (copy/paste)

```bash
export DEBFULLNAME="Blackrush LLC"
export DEBEMAIL="erikolson1965@gmail.com"
export DEBSIGN_KEYID=<YOUR_LONG_KEY_ID>
VERSION=1.1.1
SERIES=noble
PPA=ppa:blackrush/basil

dch -v ${VERSION}-0ubuntu1 -D ${SERIES} "New upstream release."

rm -rf dist
git clean -xfd   # optional but thorough; beware

rm -f ../basil_${VERSION}.orig.tar.gz
git archive --format=tar --prefix="basil-$VERSION/" HEAD | gzip -n > ../basil_${VERSION}.orig.tar.gz

debuild -S -sa -k"$DEBSIGN_KEYID"

dput "$PPA" ../basil_${VERSION}-0ubuntu1_source.changes
```

---

Next time you’ll update the version, clean, rebuild the orig tarball, build the signed source, and `dput`.
