use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use axum::{body::Body, http::Request, response::Response};
use tokio::process::Command;

use crate::{util, AppState};

pub async fn file_contains_basil(path: &Path) -> Result<bool> {
    let content = tokio::fs::read(path).await?;
    Ok(content.windows(8).any(|w| w == b"<?basil "))
}

pub async fn render_html_with_basil(app: &AppState, _req: Request<Body>, path: PathBuf) -> Result<Response> {
    // Read entire file for now; TODO: incremental streaming
    let bytes = tokio::fs::read(&path).await?;
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + 1024);

    // Simple tokenizer for <?basil ... ?> blocks
    let open = b"<?basil";
    let close = b"?>";

    // Implement a pass using positions
    let mut pos = 0;
    loop {
        let next = find_subslice(&bytes, open, pos);
        match next {
            None => { out.extend_from_slice(&bytes[pos..]); break; }
            Some(start) => {
                // Copy literal prefix
                out.extend_from_slice(&bytes[pos..start]);
                let code_start = start + open.len();
                // Skip whitespace
                let mut cs = code_start;
                while cs < bytes.len() && (bytes[cs] == b' ' || bytes[cs] == b'\t' || bytes[cs] == b'\r' || bytes[cs] == b'\n') { cs += 1; }
                if let Some(end) = find_subslice(&bytes, close, cs) {
                    let code_bytes = &bytes[cs..end];
                    let rendered = eval_inline_basil(app, path.as_path(), code_bytes).await.unwrap_or_else(|e| format!("<pre class=\"error\">{}</pre>", e));
                    out.extend_from_slice(rendered.as_bytes());
                    pos = end + close.len();
                } else {
                    // No close; copy remainder and stop
                    out.extend_from_slice(&bytes[start..]);
                    break;
                }
            }
        }
    }

    Ok(Response::new(Body::from(out)))
}

fn find_subslice(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() { return Some(from); }
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}

async fn eval_inline_basil(app: &AppState, _parent: &Path, code: &[u8]) -> Result<String> {
    // Create cache directory .basilcache/inline
    let mut cache_dir = app.bytecode_root.clone().into_std_path_buf();
    cache_dir.push(".basilcache");
    cache_dir.push("inline");
    tokio::fs::create_dir_all(&cache_dir).await.ok();

    // Hash code for stable filename
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code);
    let hash = hasher.finalize();
    let name = format!("__inline_{:x}.basil", hash[0..8].iter().fold(0u64, |acc, b| (acc << 8) | (*b as u64)));
    let src_path = cache_dir.join(name);

    // Write source file (if not exists or changed)
    tokio::fs::write(&src_path, code).await?;

    // Run via basilc run; capture stdout
    let output = Command::new(if cfg!(target_os = "windows") { "basilc.exe" } else { "basilc" })
        .arg("run")
        .arg(&src_path)
        .output()
        .await
        .context("run inline basil")?;

    if !output.status.success() {
        let tail = util::stderr_tail(&output.stderr, 2000);
        return Ok(format!("<pre class=\"error\">{}</pre>", tail));
    }

    // Parse CGI-like headers if present; otherwise treat as text/plain and embed
    let so = crate::cgi::split_headers_and_body(&output.stdout);
    Ok(String::from_utf8_lossy(&so.body).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write as _;

    #[tokio::test]
    async fn detects_basil_tag() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"<h1><?basil PRINT \"X\" ?> ok").unwrap();
        let b = file_contains_basil(f.path()).await.unwrap();
        assert!(b);
    }
}
