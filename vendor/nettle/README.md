# nettle

Rust bindings for the [Nettle cryptographic
library](https://www.lysator.liu.se/~nisse/nettle/).

The documentation for the latest version can also be found
[here](https://sequoia-pgp.gitlab.io/nettle-rs).

# Windows

The bindings can be compiled using Mingw-w64 or the MSVC toolchain.

In the MSYS2/mingw64 environment, you can install the toolchain and
Nettle by running:

```
$ pacman -S mingw-w64-x86_64-clang mingw-w64-x86_64-pkg-config mingw-w64-x86_64-nettle
```

For MSVC, first install the "Build Tools for Visual Studio".  Then,
install Nettle, for example from MSYS2:

```
$ pacman -S libnettle-devel
```

# License

This project is licensed under either of

 * GNU General Public License, Version 2.0, ([LICENSE-GPL2](LICENSE-GPL2) or
   https://www.gnu.org/licenses/old-licenses/gpl-2.0.html)
 * GNU General Public License, Version 3.0, ([LICENSE-GPL3](LICENSE-GPL3) or
   https://www.gnu.org/licenses/gpl.html)
 * GNU Lesser General Public License, Version 3.0, ([LICENSE-LGPL3](LICENSE-LGPL3) or
   https://www.gnu.org/licenses/lgpl.html)

at your option.
