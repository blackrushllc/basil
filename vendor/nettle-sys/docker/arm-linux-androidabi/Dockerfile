# Source docker file. Needs to match up with HOST_ARCH
FROM japaric/arm-linux-androideabi:v0.1.14

# Target architecture. See https://github.com/rust-embedded/cross#supported-targets
# for a list of all supported targets.
ENV HOST_ARCH=arm-linux-androideabi

# Version of the nettle dependencies to build.
ENV GMP_VERSION=6.1.2 \
    NETTLE_VERSION=3.4.1

# Additional cross compiler config
# Android needs the following:
ENV CFLAGS="-fPIE -fPIC" \
    LDFLAGS="-pie" \
    PATH="/tmp/android-ndk/bin:${PATH}" \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_PATH=/tmp/prefix/lib/pkgconfig

# Install deps
RUN DEBIAN_FRONTEND=noninteractive \
    apt update -yy && \
    apt install -yy m4 wget libclang1

# Cross compile GMP
RUN cd /tmp && \
    wget https://gmplib.org/download/gmp/gmp-${GMP_VERSION}.tar.bz2 && \
    tar xvf gmp-${GMP_VERSION}.tar.bz2 && \
    cd gmp-${GMP_VERSION} && \
    AR="${HOST_ARCH}-ar" \
    AS="${HOST_ARCH}-clang" \
    CC="${HOST_ARCH}-clang" \
    CXX="${HOST_ARCH}-clang++" \
    LD="${HOST_ARCH}-ld" \
    STRIP="${HOST_ARCH}-strip" ./configure --host=${HOST_ARCH} --prefix=/tmp/prefix && \
    make && make install

# Cross compile Nettle
RUN cd /tmp && \
    wget https://ftp.gnu.org/gnu/nettle/nettle-${NETTLE_VERSION}.tar.gz && \
    tar xvf nettle-${NETTLE_VERSION}.tar.gz && \
    cd nettle-${NETTLE_VERSION} && \
    AR="${HOST_ARCH}-ar" \
    AS="${HOST_ARCH}-clang" \
    CC="${HOST_ARCH}-clang" \
    CXX="${HOST_ARCH}-clang++" \
    LD="${HOST_ARCH}-ld" \
    STRIP="${HOST_ARCH}-strip" ./configure --host=${HOST_ARCH} --prefix=/tmp/prefix \
      --with-lib-path=/tmp/prefix/lib \
      --with-include-path=/tmp/prefix/include && \
    make && make install

