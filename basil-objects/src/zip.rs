use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use basil_common::{Result, BasilError};

#[cfg(feature = "obj-zip")]
use walkdir::WalkDir;
#[cfg(feature = "obj-zip")]
use zip::write::FileOptions;
#[cfg(feature = "obj-zip")]
use zip::CompressionMethod;

// This module provides ZIP helpers used by the VM builtins.
// We also expose a no-op register() to satisfy the object hub pattern.

pub fn register(_reg: &mut crate::Registry) {
    // No object types to register; ZIP is exposed via global builtins (ZIP_*) in the VM.
}

fn to_err<E: std::fmt::Display>(prefix: &str, e: E) -> BasilError {
    BasilError(format!("{prefix}: {e}"))
}

#[cfg(feature = "obj-zip")]
fn norm_entry_name(base: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

#[cfg(feature = "obj-zip")]
pub fn zip_extract_all(zip_path: &str, dest_dir: &str) -> Result<()> {
    let file = File::open(zip_path).map_err(|e| to_err("ZIP_EXTRACT_ALL open", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| to_err("ZIP_EXTRACT_ALL parse", e))?;

    std::fs::create_dir_all(dest_dir).map_err(|e| to_err("ZIP_EXTRACT_ALL mkdir", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| to_err("ZIP_EXTRACT_ALL entry", e))?;
        let outpath = Path::new(dest_dir).join(entry.mangled_name());

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| to_err("ZIP_EXTRACT_ALL mkdir entry", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| to_err("ZIP_EXTRACT_ALL mkparent", e))?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| to_err("ZIP_EXTRACT_ALL create", e))?;
            io::copy(&mut entry, &mut outfile).map_err(|e| to_err("ZIP_EXTRACT_ALL copy", e))?;
        }
    }

    Ok(())
}

#[cfg(feature = "obj-zip")]
pub fn zip_compress_file(src_path: &str, zip_path: &str, entry_name: Option<&str>) -> Result<()> {
    let entry_name_owned;
    let entry_name = match entry_name {
        Some(n) => n,
        None => {
            entry_name_owned = Path::new(src_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            &entry_name_owned
        }
    };

    let file = File::create(zip_path).map_err(|e| to_err("ZIP_COMPRESS_FILE create zip", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(entry_name, opts).map_err(|e| to_err("ZIP_COMPRESS_FILE start", e))?;
    let mut src = File::open(src_path).map_err(|e| to_err("ZIP_COMPRESS_FILE open src", e))?;
    let mut buf = Vec::new();
    src.read_to_end(&mut buf).map_err(|e| to_err("ZIP_COMPRESS_FILE read", e))?;
    zip.write_all(&buf).map_err(|e| to_err("ZIP_COMPRESS_FILE write", e))?;
    zip.finish().map_err(|e| to_err("ZIP_COMPRESS_FILE finish", e))?;

    Ok(())
}

#[cfg(feature = "obj-zip")]
pub fn zip_compress_dir(src_dir: &str, zip_path: &str) -> Result<()> {
    let base = PathBuf::from(src_dir);
    let file = File::create(zip_path).map_err(|e| to_err("ZIP_COMPRESS_DIR create zip", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);

    for entry in WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = norm_entry_name(&base, path);

        if path.is_dir() {
            zip.add_directory(format!("{name}/"), opts).map_err(|e| to_err("ZIP_COMPRESS_DIR add dir", e))?;
        } else {
            zip.start_file(name, opts).map_err(|e| to_err("ZIP_COMPRESS_DIR start file", e))?;
            let mut f = File::open(path).map_err(|e| to_err("ZIP_COMPRESS_DIR open file", e))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| to_err("ZIP_COMPRESS_DIR read file", e))?;
            zip.write_all(&buf).map_err(|e| to_err("ZIP_COMPRESS_DIR write file", e))?;
        }
    }

    zip.finish().map_err(|e| to_err("ZIP_COMPRESS_DIR finish", e))?;
    Ok(())
}

#[cfg(feature = "obj-zip")]
pub fn zip_list(zip_path: &str) -> Result<String> {
    let file = File::open(zip_path).map_err(|e| to_err("ZIP_LIST open", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| to_err("ZIP_LIST parse", e))?;
    let mut out = String::new();

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| to_err("ZIP_LIST entry", e))?;
        if i > 0 { out.push('\n'); }
        out.push_str(entry.name());
    }
    Ok(out)
}
