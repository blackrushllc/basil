### Troubleshooting ALSA dependency errors on Linux

```

In the output, I am at a bash prompt "root@YORE-ARC:/var/www/basil#" and first seen to be switching to the "webmain" branch...

Console output:

root@YORE-ARC:/var/www/basil# git checkout webmain

Switched to branch 'webmain'
Your branch is ahead of 'origin/webmain' by 52 commits.
(use "git push" to publish your local commits)


root@YORE-ARC:/var/www/basil# cargo run -q -p basilc --features obj-all -- run examples/hello.basil

Hello World!
This is loop number     1
This is loop number     2
This is loop number     3
This is loop number     4
This is loop number     5
Goodbye!


root@YORE-ARC:/var/www/basil# git checkout development

Switched to branch 'development'
Your branch is up to date with 'origin/development'.


root@YORE-ARC:/var/www/basil# cargo run -q -p basilc --features obj-all -- run examples/hello.basil

error: failed to run custom build command for `alsa-sys v0.3.1`

Caused by:
process didn't exit successfully: `/var/www/basil/target/debug/build/alsa-sys-56c2230aacaabc16/build-script-build` (exit status: 101)
--- stdout
cargo:rerun-if-env-changed=ALSA_NO_PKG_CONFIG
cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-unknown-linux-gnu
cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_unknown_linux_gnu
cargo:rerun-if-env-changed=HOST_PKG_CONFIG
cargo:rerun-if-env-changed=PKG_CONFIG
cargo:rerun-if-env-changed=ALSA_STATIC
cargo:rerun-if-env-changed=ALSA_DYNAMIC
cargo:rerun-if-env-changed=PKG_CONFIG_ALL_STATIC
cargo:rerun-if-env-changed=PKG_CONFIG_ALL_DYNAMIC
cargo:rerun-if-env-changed=PKG_CONFIG_PATH_x86_64-unknown-linux-gnu
cargo:rerun-if-env-changed=PKG_CONFIG_PATH_x86_64_unknown_linux_gnu
cargo:rerun-if-env-changed=HOST_PKG_CONFIG_PATH
cargo:rerun-if-env-changed=PKG_CONFIG_PATH
cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR_x86_64-unknown-linux-gnu
cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR_x86_64_unknown_linux_gnu
cargo:rerun-if-env-changed=HOST_PKG_CONFIG_LIBDIR
cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR
cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-unknown-linux-gnu
cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_unknown_linux_gnu
cargo:rerun-if-env-changed=HOST_PKG_CONFIG_SYSROOT_DIR
cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR

--- stderr

thread 'main' panicked at /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/alsa-sys-0.3.1/build.rs:13:18:

pkg-config exited with status code 1
> PKG_CONFIG_ALLOW_SYSTEM_LIBS=1 PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 pkg-config --libs --cflags alsa

The system library `alsa` required by crate `alsa-sys` was not found.
The file `alsa.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
The PKG_CONFIG_PATH environment variable is not set.

HINT: if you have installed the library, try setting PKG_CONFIG_PATH to the directory containing `alsa.pc`.

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```


You’ve got a classic “native dependency” failure: your newer branch pulls in a crate that needs the system ALSA development headers, and on this box they’re not installed. That’s why `alsa-sys` (brought in via your audio stack—likely `cpal`/`rodio`/your DAW code) dies at the `pkg-config` step.

Here’s how I’d triage and fix it quickly:

## 1) Confirm what’s pulling ALSA in

```bash
# from the development branch
cargo tree -i alsa-sys
# and see all enabled features for basilc
cargo tree -e features -p basilc
```

You’ll probably see `cpal`/`rodio` (or your `obj-daw`) in that chain.

## 2) Install the missing system deps (Ubuntu/Debian)

```bash
apt-get update
apt-get install -y build-essential pkg-config libasound2-dev
```

Then verify `pkg-config` can see ALSA:

```bash
pkg-config --libs --cflags alsa
```

If that prints flags (e.g., `-lasound`), you’re good. Re-run:

```bash
cargo run -q -p basilc --features obj-all -- run examples/hello.basil
```

## 3) If this machine is headless and you don’t need audio here

You can build without the DAW/audio feature to avoid ALSA entirely:

```bash
# Replace obj-all with the specific features you actually need, excluding the audio one
cargo run -q -p basilc --no-default-features --features "obj-json,obj-zip,obj-sql,..." -- run examples/hello.basil
```

(Or redefine `obj-all` in your Cargo features to *not* include the DAW bundle on Linux servers.)

## 4) Useful sanity checks if it still fails

* Make sure you didn’t inherit a weird env:
  `env | grep -E 'PKG_CONFIG|ALSA'`
* Clear build artifacts after adding system libs:
  `cargo clean && cargo build -p basilc`
* Turn on a backtrace for more context:
  `RUST_BACKTRACE=1 cargo build -p basilc`

## 5) Make it future-proof

* Document the native deps in your README/SETUP (Linux: `libasound2-dev`, macOS: none for ALSA; Windows uses different backends).
* Feature-gate audio (`obj-daw`) so CI and server builds don’t require ALSA unless requested.
* Consider a “minimal” feature set for servers and an “all” set for dev desktops.

Short version: install `pkg-config` and `libasound2-dev`, or disable the audio feature on this host. That will get your development branch building like the older `webmain` did.
