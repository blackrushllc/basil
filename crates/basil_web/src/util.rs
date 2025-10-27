use std::{fs, io, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use sha2::{Digest, Sha256};

pub fn read_mtime(p: &Path) -> Result<SystemTime> { Ok(fs::metadata(p)?.modified()?) }

pub fn etag_weak_for_meta(p: &Path) -> Result<String> {
    let md = fs::metadata(p)?;
    let mtime = md.modified().unwrap_or(SystemTime::UNIX_EPOCH).duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let size = md.len();
    let mut h = Sha256::new();
    h.update(size.to_le_bytes());
    h.update(mtime.to_le_bytes());
    let sum = h.finalize();
    Ok(format!("W/\"{}\"", B64.encode(sum)))
}

pub fn safe_join(root: &Path, req_path: &str) -> Result<PathBuf> {
    // Canonicalize root for security checks, but build the returned path from the original root
    // so callers can reliably test `starts_with(root)` even on Windows where canonicalize may
    // introduce a verbatim prefix (\\?\C:\...).
    let canon_root = fs::canonicalize(root).context("canonicalize root")?;

    // Build under the provided root path
    let mut result = PathBuf::from(root);

    // Walk the requested path using OS components; ignore absolute markers
    for comp in std::path::Path::new(req_path).components() {
        use std::path::Component;
        match comp {
            Component::Prefix(_) | Component::RootDir | Component::CurDir => {
                // drop drive letters (Windows), leading slashes, and '.'
            }
            Component::ParentDir => {
                return Err(anyhow::anyhow!("path traversal attempted"));
            }
            Component::Normal(seg) => {
                result.push(seg);
            }
        }
    }

    // If the resulting path exists, enforce that its canonical form stays under the canonical root
    if result.exists() {
        let canon_res = fs::canonicalize(&result).unwrap_or(result.clone());
        if !canon_res.starts_with(&canon_root) {
            return Err(anyhow::anyhow!("path escapes root"));
        }
        return Ok(result);
    }

    // For non-existing targets, we built directly under `root`, so lexical prefix check suffices
    if !result.starts_with(root) {
        return Err(anyhow::anyhow!("path escapes root"));
    }

    Ok(result)
}

pub fn stderr_tail(bytes: &[u8], max: usize) -> String {
    let n = bytes.len();
    let start = n.saturating_sub(max);
    String::from_utf8_lossy(&bytes[start..]).to_string()
}

pub fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    Ok(())
}

pub fn change_ext(path: &Path, new_ext: &str) -> PathBuf {
    let mut p = PathBuf::from(path);
    p.set_extension(new_ext);
    p
}
