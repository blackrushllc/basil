//! Optionally builds the manual page.

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    build_man_pages()
}

#[cfg(not(feature = "clap_mangen"))]
fn build_man_pages() -> Result<()> {
    if cfg!(feature = "clap") {
	println!("cargo:warning=To build the man pages, \
		  enable the clap_mangen feature");
    }
    Ok(())
}

#[cfg(feature = "clap_mangen")]
pub mod cli {
    include!("src/cli.rs");
}


#[cfg(feature = "clap_mangen")]
fn build_man_pages() -> Result<()> {
    // Man page support.
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR")
            .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?);

    use clap::CommandFactory;
    let man = clap_mangen::Man::new(cli::Opt::command());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    let filename = out_dir.join("sha1cdsum.1");
    println!("cargo:warning=writing man page to {}", filename.display());
    std::fs::write(filename, buffer)?;

    Ok(())
}
