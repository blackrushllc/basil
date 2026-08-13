// basilc/src/embedded.rs
// Runtime helpers for embedded includes generated at build time.

#![allow(dead_code)]

// Pull in the generated table: defines EmbeddedFile and EMBEDDED_FILES
include!(concat!(env!("OUT_DIR"), "/embedded_includes.rs"));

use std::fs;
use std::path::{Path, PathBuf};

pub fn list_all_paths() -> impl Iterator<Item = &'static str> {
    EMBEDDED_FILES.iter().map(|f| f.path)
}

pub fn list_top_level_dirs() -> Vec<&'static str> {
    let mut dirs = Vec::new();
    for p in list_all_paths() {
        if let Some((first, _rest)) = p.split_once('/') {
            if !dirs.contains(&first) {
                dirs.push(first);
            }
        }
    }
    dirs
}

pub fn find_file(logical: &str) -> Option<&'static EmbeddedFile> {
    if let Some(f) = EMBEDDED_FILES.iter().find(|f| f.path == logical) {
        return Some(f);
    }
    // convenience: try "<name>.basil" for bare names like "upgrade"
    if !logical.contains('/') && !logical.ends_with(".basil") {
        let fallback = format!("{logical}.basil");
        EMBEDDED_FILES.iter().find(|f| f.path == fallback)
    } else {
        None
    }
}

pub fn has_dir(dir: &str) -> bool {
    let prefix = ensure_trailing_slash(dir);
    EMBEDDED_FILES.iter().any(|f| f.path.starts_with(&prefix))
}

pub fn write_single(logical: &str, dest_root: &Path) -> std::io::Result<PathBuf> {
    let file = find_file(logical).ok_or_else(|| not_found(logical))?;
    let out = resolved_output_path_for_file(logical, dest_root);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, file.contents)?;
    Ok(out)
}

pub fn extract_dir(dir: &str, dest_root: &Path) -> std::io::Result<()> {
    let prefix = ensure_trailing_slash(dir);
    let mut found_any = false;
    for f in EMBEDDED_FILES.iter() {
        if f.path.starts_with(&prefix) {
            found_any = true;
            let rel = &f.path[prefix.len()..]; // e.g. "hello.basil" or nested like "sub/x.basil"
            let out = join_normalized(&join_normalized(dest_root, dir), rel);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out, f.contents)?;
        }
    }
    if !found_any {
        return Err(not_found(dir));
    }
    Ok(())
}

fn not_found(name: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("No embedded entry {name:?}"),
    )
}

fn ensure_trailing_slash(s: &str) -> String {
    if s.ends_with('/') {
        s.to_string()
    } else {
        format!("{s}/")
    }
}

fn resolved_output_path_for_file(logical: &str, dest_root: &Path) -> PathBuf {
    // If logical contains a '/', treat as relative path under CWD.
    // If bare name like "upgrade", write "<cwd>/upgrade.basil".
    if logical.contains('/') || logical.ends_with(".basil") {
        join_normalized(dest_root, logical)
    } else {
        join_normalized(dest_root, &format!("{logical}.basil"))
    }
}

fn join_normalized(root: &Path, logical: &str) -> PathBuf {
    let mut out = PathBuf::from(root);
    for seg in logical.split('/') {
        if seg.is_empty() {
            continue;
        }
        out.push(seg);
    }
    out
}

pub fn is_unsafe_target(target: &str) -> bool {
    let p = Path::new(target);
    p.is_absolute() || target.contains("..")
}
