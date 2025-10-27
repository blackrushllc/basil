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
    let mut result = PathBuf::from(root);
    for seg in req_path.split('/') {
        if seg.is_empty() || seg == "." { continue; }
        if seg == ".." { return Err(anyhow::anyhow!("path traversal attempted")); }
        result.push(seg);
    }
    let canon_root = fs::canonicalize(root).context("canonicalize root")?;
    let canon = fs::canonicalize(&result).unwrap_or(result.clone());
    if !canon.starts_with(&canon_root) {
        return Err(anyhow::anyhow!("path escapes root"));
    }
    Ok(canon)
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
