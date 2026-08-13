use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

fn main() {
    // 1) Generate embedded includes table for all platforms
    if let Err(e) = generate_embedded_includes() {
        // Don't hard fail the build if includes generation fails unexpectedly; surface a clear error.
        // However, in most cases this should not fail.
        panic!("Failed to generate embedded includes: {e}");
    }

    // 2) On Windows, also embed icon/version resources
    #[cfg(windows)]
    embed_win_resources();
}

#[cfg(windows)]
fn embed_win_resources() {
    let mut res = winres::WindowsResource::new();
    let ver = env!("CARGO_PKG_VERSION");
    res.set("FileVersion", ver);
    res.set("ProductVersion", ver);
    res.set("ProductName", "Basil BASIC");
    res.set("FileDescription", "Basil BASIC Interpreter (basilc)");
    res.set("CompanyName", "Blackrush LLC");
    res.set("LegalCopyright", "© Blackrush LLC");
    let icon_path = "../favicon.ico";
    if std::path::Path::new(icon_path).exists() {
        res.set_icon(icon_path);
    }
    res.compile()
        .expect("Failed to embed Windows resources for basilc");
}

fn generate_embedded_includes() -> std::io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let includes_root = manifest_dir.join("includes");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("embedded_includes.rs");

    // Collect all files under includes/, recursively.
    let mut entries: Vec<String> = Vec::new();
    if includes_root.exists() {
        collect_files(&includes_root, &includes_root, &mut entries)?;
        entries.sort();
    }

    let mut f = fs::File::create(&dest)?;
    writeln!(
        f,
        r#"#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFile {{ pub path: &'static str, pub contents: &'static [u8] }}

pub static EMBEDDED_FILES: &[EmbeddedFile] = &["#
    )?;

    for logical in entries {
        // Generate code like:
        // EmbeddedFile { path: "examples/hello.basil", contents: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/includes/examples/hello.basil")) },
        let include_code = format!(
            "concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/includes/{}\")",
            logical
        );
        writeln!(
            f,
            "    EmbeddedFile {{ path: {lp:?}, contents: include_bytes!({ic}) }},",
            lp = logical,
            ic = include_code
        )?;
    }

    writeln!(f, "];\n")?;
    println!("cargo:rerun-if-changed=includes");
    Ok(())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let p = ent.path();
        let meta = ent.metadata()?;
        if meta.is_dir() {
            collect_files(root, &p, out)?;
        } else if meta.is_file() {
            let rel = p.strip_prefix(root).unwrap();
            // Normalize to forward slashes for a stable logical path
            let logical = rel.to_string_lossy().replace('\\', "/");
            out.push(logical);
        }
    }
    Ok(())
}
