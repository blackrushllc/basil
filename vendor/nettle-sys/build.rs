use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Like std::env::var, but informs cargo.
///
/// Always use this one instead of the function in the standard
/// library!
///
/// If we look at an environment variable, we need to tell cargo that
/// we're doing so.  This makes sure that cargo will consider the
/// build stale if the value of the environment variable changes.
fn env_var<K>(key: K)-> std::result::Result<String, std::env::VarError>
where
    K: AsRef<std::ffi::OsStr>,
{
    let key = key.as_ref();
    println!("cargo:rerun-if-env-changed={}", key.to_string_lossy());
    std::env::var(key)
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Build-time configuration.
struct Config {
    /// Paths containing header files needed for compilation.
    include_paths: Vec<PathBuf>,

    /// Cv448/Ed448 support in Nettle.
    have_cv448: bool,

    /// OCB support in Nettle.
    have_ocb: bool,
}

/// Returns true if the given Nettle version matches or exceeds our
/// required version.
///
/// In general, it is better to use check_cc than to rely on the
/// Nettle version number to gate features.
#[allow(dead_code)]
fn is_version_at_least<V: AsRef<str>>(nettle_version: V, need: &[u8]) -> bool {
    for (i, (n, component)) in need.iter()
        .zip(nettle_version.as_ref().split('.'))
        .enumerate()
    {
        if let Ok(c) = component.parse::<u8>() {
            if c < *n {
                return false;
            } else if c > *n {
                return true;
            } else {
                // Compare the next component.
            }
        } else {
            panic!("Failed to parse the {}th component of the Nettle version \
                    {:?}: {:?}", i + 1, nettle_version.as_ref(), component);
        }
    }

    true
}

/// Returns whether or not the feature detection should be overridden,
/// and if so, whether the feature should be enabled.
fn check_override(feature: &str) -> Option<bool> {
    env_var(&format!("NETTLE_HAVE_{}", feature))
        .ok()
        .map(|v| match v.to_lowercase().as_str() {
            "1" | "y" | "yes" | "enable" | "true" => true,
            _ => false,
        })
}

/// Tries to compile a test program to probe for Nettle features.
fn check_cc(includes: &[PathBuf], header: &str, symbol: &str) -> bool {
    let t = || -> Result<bool> {
        let wd = tempfile::tempdir()?;
        let testpath = wd.path().join("test.c");
        fs::write(&testpath, format!("#include <nettle/{}>
int
main()
{{
  (void) {};
}}
", header, symbol))?;

        let tool = cc::Build::new()
            .warnings(false)
            .includes(includes)
            .try_get_compiler()?;

        let output = tool.to_command()
            .current_dir(&wd)
            .arg(&testpath)
            .output()?;
        Ok(output.status.success())
    };
    t().unwrap_or(false)
}

/// Checks whether Nettle supports Curve 448.
fn check_cv448(includes: &[PathBuf]) -> bool {
    check_override("CV448")
        .unwrap_or_else(|| check_cc(includes, "eddsa.h",
                                    "nettle_ed448_shake256_sign"))
}

/// Checks whether Nettle supports OCB mode.
fn check_ocb(includes: &[PathBuf]) -> bool {
    check_override("OCB")
        .unwrap_or_else(|| check_cc(includes, "ocb.h",
                                    "nettle_ocb_encrypt"))
}

#[cfg(target_env = "msvc")]
fn try_vcpkg() -> Result<Config> {
    let lib = vcpkg::Config::new()
        .emit_includes(true)
        .find_package("nettle")?;

    Ok(Config {
        have_cv448: check_cv448(&lib.include_paths),
        have_ocb: check_ocb(&lib.include_paths),
        include_paths: lib.include_paths,
    })
}

#[cfg(not(target_env = "msvc"))]
fn try_vcpkg() -> Result<Config> { Err("not applicable")?; unreachable!() }

fn try_pkg_config() -> Result<Config> {
    let mut nettle= Library::from(pkg_config::probe_library("nettle")?)
        .merge(pkg_config::probe_library("hogweed")?.into());

    // GMP only got pkg-config support in release 6.2 (2020-01-18). So we'll try to use it if
    // possible, but fall back to just looking for it in the default include and lib paths.
    if let Ok(gmp) = pkg_config::probe_library("gmp").map(Into::into) {
        nettle = nettle.merge(gmp);
    } else {
        let mode = match env_var("GMP_STATIC").ok() {
            Some(_) => "static",
            None => "dylib",
        };

        println!("cargo:rustc-link-lib={}=gmp", mode);
    }

    Ok(Config {
        have_cv448: check_cv448(&nettle.include_paths),
        have_ocb: check_ocb(&nettle.include_paths),
        include_paths: nettle.include_paths,
    })
}

const NETTLE_PREGENERATED_BINDINGS: &str = "NETTLE_PREGENERATED_BINDINGS";

fn real_main() -> Result<()> {
    let config = try_vcpkg().or_else(|_| try_pkg_config())?;

    // There is a bug (or obscure feature) in pkg-config-rs that
    // doesn't let you statically link against libraries installed in
    // /usr: https://github.com/rust-lang/pkg-config-rs/issues/102
    //
    // Work around that by emitting the directives necessary for cargo
    // to statically link against Nettle, Hogweed, and GMP.
    if env_var("NETTLE_STATIC").is_ok() {
        println!("cargo:rustc-link-lib=static=nettle");
        println!("cargo:rustc-link-lib=static=hogweed");
        println!("cargo:rustc-link-lib=static=gmp");
    }

    let out_path =
        Path::new(&std::env::var("OUT_DIR").unwrap()).join("bindings.rs");

    // Check if we have a bundled bindings.rs.
    if let Ok(bundled) = env_var(NETTLE_PREGENERATED_BINDINGS)
    {
        let p = Path::new(&bundled);
        if p.exists() {
            fs::copy(&p, &out_path)?;
            println!("cargo:rerun-if-changed={:?}", p);

            // We're done.
            return Ok(());
        }
    }

    let mut builder = bindgen::Builder::default()
        // Includes all nettle headers except mini-gmp.h
        .header("bindgen-wrapper.h")
        // Workaround for https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .blocklist_type("max_align_t")
        .blocklist_function("strtold")
        .blocklist_function("qecvt")
        .blocklist_function("qfcvt")
        .blocklist_function("qgcvt")
        .blocklist_function("qecvt_r")
        .blocklist_function("qfcvt_r")
        .size_t_is_usize(true);

    if config.have_cv448 {
        builder = builder.header_contents("cv448-wrapper.h", "#include <nettle/curve448.h>");
    }

    if config.have_ocb {
        builder = builder.header_contents("ocb-wrapper.h", "#include <nettle/ocb.h>");
    }

    for p in config.include_paths {
        builder = builder.clang_arg(format!("-I{}", p.display()));
    }

    let bindings = builder.generate().unwrap();
    bindings.write_to_file(&out_path)?;

    let mut s = fs::OpenOptions::new().append(true).open(out_path)?;
    writeln!(s, "\
/// Compile-time configuration.
pub mod config {{
    /// Cv448/Ed448 support in Nettle.
    pub const HAVE_CV448: bool = {};

    /// OCB support in Nettle.
    pub const HAVE_OCB: bool = {};
}}
",
             config.have_cv448,
             config.have_ocb,
    )?;

    if ! config.have_cv448 {
        let mut stubs = fs::File::open("cv448-stubs.rs")?;
        io::copy(&mut stubs, &mut s)?;
    }

    if ! config.have_ocb {
        let mut stubs = fs::File::open("ocb-stubs.rs")?;
        io::copy(&mut stubs, &mut s)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    match real_main() {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Building nettle-sys failed.");
            eprintln!("Because: {}", e);
            eprintln!();
            print_hints();
            Err("Building nettle-sys failed.")?;
            unreachable!()
        }
    }
}

fn print_hints() {
    eprintln!("Please make sure the necessary build dependencies are \
               installed.\n");

    if cfg!(target_os = "windows") {
        eprintln!("If you are using MSYS2, try:

    $ pacman -S mingw-w64-x86_64-{{clang,pkg-config,nettle}} libnettle-devel
");
    } else if cfg!(target_os = "macos") {
        eprintln!("If you are using MacPorts, try:

    $ sudo port install nettle pkgconfig
");
    } else if cfg!(target_os = "linux") {
        eprintln!("If you are using Debian (or a derivative), try:

    $ sudo apt install clang llvm pkg-config nettle-dev
");

        eprintln!("If you are using Arch (or a derivative), try:

    $ sudo pacman -S clang pkg-config nettle --needed
");

        eprintln!("If you are using Fedora (or a derivative), try:

    $ sudo dnf install clang pkg-config nettle-devel
");
    }

    eprintln!("See https://gitlab.com/sequoia-pgp/nettle-sys#building \
               for more information.\n");
}

/// Writes a log message to /tmp/l.
///
/// I found it hard to log messages from the build script.  Hence this
/// function.
#[allow(dead_code)]
fn log<S: AsRef<str>>(m: S) {
    writeln!(&mut fs::OpenOptions::new().append(true).open("/tmp/l").unwrap(),
             "{}", m.as_ref()).unwrap();
}

/// Somewhat like pkg_config::Library, but only with the parts we use.
#[derive(Default)]
pub struct Library {
    pub libs: Vec<String>,
    pub link_paths: Vec<PathBuf>,
    pub frameworks: Vec<String>,
    pub framework_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
}

impl Library {
    /// Merges two libraries into one.
    fn merge(mut self, mut other: Self) -> Self {
        self.libs.append(&mut other.libs);
        self.libs.sort();
        self.libs.dedup();

        self.link_paths.append(&mut other.link_paths);
        self.link_paths.sort();
        self.link_paths.dedup();

        self.frameworks.append(&mut other.frameworks);
        self.frameworks.sort();
        self.frameworks.dedup();

        self.framework_paths.append(&mut other.framework_paths);
        self.framework_paths.sort();
        self.framework_paths.dedup();

        self.include_paths.append(&mut other.include_paths);
        self.include_paths.sort();
        self.include_paths.dedup();

        self
    }

    /// Emits directives to make cargo link to the library.
    ///
    /// Note: If we are using pkg_config, then the necessary
    /// directives are emitted by that crate already.  Emitting these
    /// ourselves may be useful in the future if we add other ways to
    /// configure how to build against Nettle.
    #[allow(dead_code)]
    fn print_library(&self, mode: &str) {
	for p in &self.include_paths {
	    println!("cargo:include={}", p.display());
	}

	for p in &self.frameworks {
	    println!("cargo:rustc-link-lib=framework={}", p);
	}

	for p in &self.framework_paths {
	    println!("cargo:rustc-link-search=framework={}", p.display());
	}

        for p in &self.libs {
            println!("cargo:rustc-link-lib={}={}", mode, p);
        }

        for p in &self.link_paths {
            println!("cargo:rustc-link-search=native={}", p.display());
        }
    }
}

impl From<pkg_config::Library> for Library {
    fn from(l: pkg_config::Library) -> Library {
        Library {
            libs: l.libs,
            link_paths: l.link_paths,
            frameworks: l.frameworks,
            framework_paths: l.framework_paths,
            include_paths: l.include_paths,
        }
    }
}
