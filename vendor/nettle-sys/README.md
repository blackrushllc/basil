# nettle-sys

Low-level Rust bindings for the [Nettle cryptographic library](https://git.lysator.liu.se/nettle/nettle).

The documentation can be found [here](https://sequoia-pgp.gitlab.io/nettle-sys).

## Building

To build the Nettle bindings, you need a few libraries and support
packages.  Notably, you need the Nettle cryptographic library version
3.4.1 or up.  Some cryptographic algorithms are only available if
build against newer versions of Nettle, see [optional
features](#optional-features).

Please see below for OS-specific commands to install the needed
libraries.

### Debian

```shell
$ sudo apt install clang llvm pkg-config nettle-dev
```

### Arch Linux

```shell
$ sudo pacman -S clang pkg-config nettle --needed
```

### Fedora

```shell
$ sudo dnf install clang pkg-config nettle-devel
```

### macOS (Mojave), using MacPorts

```shell
$ sudo port install nettle pkgconfig
```

### Windows

#### MSYS2

You can install the needed libraries with the following command:

```shell
$ pacman -S mingw-w64-x86_64-{clang,pkg-config,nettle} libnettle-devel
```

## Optional features

Some cryptographic algorithms are only available if build against
newer versions of Nettle.  Support for each feature is detected at
build time.

In case the automatic detection does not work for some reason, the
optional algorithms can be enabled or disabled by setting the given
environment variable.

Setting such a variable to `1`, `y`, `yes`, `enable`, or `true`
enables a feature.  Any other value disables it.

| Algorithm    | Required Nettle version | Environment variable |
|--------------|-------------------------|----------------------|
| cv448, ed448 | >= 3.6                  | NETTLE_HAVE_CV448    |
| OCB          | >= 3.9                  | NETTLE_HAVE_OCB      |

## Cross compilation

`nettle-sys` can be cross compiled using [`cross`](https://github.com/rust-embedded/cross) and a custom Docker container. First, build the container and install `cross`:

```bash
cargo install cross
docker -t nettle-sys/<toolchain>:1 docker/<toolchain>
```

Then, you can cross compile the project:

```bash
cross --target <toolchain> -v
```

The build artifacts will be placed in `target/debug/<toolchain>`.

## Static linking

By default, `nettle-sys` is dynamically linked to its dependencies.

By defining the `NETTLE_STATIC` environment variable during the build, it can
also be statically link to its dependencies:

```bash
env NETTLE_STATIC=yes cargo build
```

This is particularly useful to produce standalone binaries that can be easily
distributed.

## Pregenerate `bindings.rs`

By default, `nettle-sys` invokes `bindgen` to generate bindings for
Nettle.  In some build environments, this might not work due to
`bindgen` depending on `libllvm`.  In this case, the `bindings.rs` may
be pregenerated, and used by setting:

```bash
env NETTLE_PREGENERATED_BINDINGS=/path/to/bindings.rs cargo build
```

Note: `bindings.rs` is specific to target architecture, operating
system, and Nettle version.

# License

This project is licensed under either of

 * GNU General Public License, Version 2.0, ([LICENSE-GPL2](LICENSE-GPL2) or
   https://www.gnu.org/licenses/old-licenses/gpl-2.0.html)
 * GNU General Public License, Version 3.0, ([LICENSE-GPL3](LICENSE-GPL3) or
   https://www.gnu.org/licenses/gpl.html)
 * GNU Lesser General Public License, Version 3.0, ([LICENSE-LGPL3](LICENSE-LGPL3) or
   https://www.gnu.org/licenses/lgpl.html)

at your option.
