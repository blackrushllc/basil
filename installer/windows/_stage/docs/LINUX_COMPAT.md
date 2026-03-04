# Two different classes of problems:

1. **GLIBC too new**

```
/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

This means you compiled `bcc` on a machine whose glibc is **newer** than the target machine. Linux binaries that link against glibc are *not* forward-compatible across older distros. If you build on glibc 2.39, it won’t run on machines with glibc 2.31/2.35/etc.

2. **Missing shared library dependency**

```
basilc: error while loading shared libraries: libasound.so.2: cannot open shared object file
```

That’s just a missing runtime dependency (ALSA). On Debian/Ubuntu it’s usually `libasound2`.

---

## The most appropriate way to handle this

### A) Pick a baseline and build against *older* glibc

This is the standard approach for “portable Linux binaries”:

* Build inside a container/VM that’s **as old as the oldest distro you want to support** (common baseline: Ubuntu 20.04 / Debian 11).
* Then the binaries will run on that baseline and **newer** distros.

Practical choices:

* **Ubuntu 20.04 (glibc 2.31)**: runs basically everywhere modern
* **Debian 11 (glibc 2.31)** similar
* If you want very wide coverage: build on something even older (but you’ll fight toolchain versions)

This is why projects publish “manylinux” wheels, or build in old containers.

### B) Offer a `musl` static build for CLI tools

For `bcc` (and probably `basilc-naked` if it’s “no extra libs”), you can avoid glibc entirely:

* Build `x86_64-unknown-linux-musl` so the binary is largely static and doesn’t care about the target distro’s glibc.

In Rust this is straightforward for pure CLI binaries.
For things that need system libs (ALSA, X11, etc.), musl static may be harder or not worth it.

### C) Treat “feature” builds as having OS packages

Your `basilc` variant that needs audio should either:

* be built **without** ALSA as a default (and ship `basilc-daw` as the one that needs it), or
* you include dependency installation instructions/checks in `install.sh`.

For Debian/Ubuntu, the fix for your current error is usually:

```bash
apt-get update && apt-get install -y libasound2
```

---

## How to *detect* prerequisites (and generate a list)

Yes — Junie can help, but the best source of truth is the binary itself.

### 1) List required shared libraries

```bash
ldd ./basilc
ldd ./bcc
```

### 2) See which GLIBC versions your binary requires

```bash
strings -a ./bcc | grep -o 'GLIBC_[0-9.]\+' | sort -Vu
```

### 3) Show dynamic section “NEEDED” entries (more precise than `ldd`)

```bash
readelf -d ./basilc | grep NEEDED
```

You can even automate this on your build machine and write it into `latest.json` as metadata.

---

## What I’d do for Basrun releases

### Release matrix (simple, realistic)

* `linux-x86_64-gnu` built on Ubuntu 20.04 (glibc 2.31)
* `linux-x86_64-musl` for `bcc` and `basilc-naked` (static-ish)
* `mac` as you already do (and ideally universal2 later)

### Variants / expectations

* `bcc` + `bcc-naked`: should run “everywhere” (musl build is perfect)
* `basilc`: baseline glibc build, minimal deps
* `basilc-daw`: clearly documented deps (ALSA, etc.), and installer checks + prints the apt/yum command

---

## What to change in `install.sh` (best practice)

Add a preflight section:

* Detect glibc version:

  ```bash
  getconf GNU_LIBC_VERSION  # e.g. "glibc 2.31"
  ```
* Compare it against the **required** version for that artifact (which you can embed in `latest.json`).
* For libs like ALSA:

    * check `ldconfig -p | grep libasound.so.2` (Linux)
    * or try `ldd "$PREFIX/basilc" | grep "not found"`

If missing, print a friendly message like:

> “You installed basilc-daw, but `libasound.so.2` is missing. On Ubuntu/Debian run: `sudo apt-get install libasound2`.”

I would **not** auto-install packages by default (people don’t like curl|bash doing `apt install` silently). Print commands is perfect.

---

## Immediate fix for your current problem

* Rebuild Linux artifacts inside an Ubuntu 20.04 container/VM (or Debian 11) so they don’t require GLIBC_2.39.
* Split the builds so `basilc` base doesn’t require ALSA (or document/install-check it).

If you want, paste the output of:

```bash
ldd /usr/local/bin/basilc | head -n 50
strings -a /usr/local/bin/bcc | grep -o 'GLIBC_[0-9.]\+' | sort -Vu | tail
```

…and I’ll tell you exactly which dependencies to encode into `latest.json` and what the installer should check for each artifact.



See Also: BUILDING.md INSTALLER_SCRIPT.md
