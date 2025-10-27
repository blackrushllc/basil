use std::{path::{Path, PathBuf}, process::Stdio, time::Duration};

use anyhow::{Context, Result};
use tokio::{io::AsyncReadExt, process::Command, time};

use crate::util;

#[derive(Debug, Clone, Default)]
pub struct CompileOutcome { pub changed: bool, pub stderr_tail: Option<String>, pub version: Option<String> }

pub async fn compile_if_stale(source: &Path, bytecode: &Path, bytecode_dir: Option<&Path>) -> Result<CompileOutcome> {
    let target_path = if let Some(dir) = bytecode_dir {
        let rel = source.file_stem().and_then(|s| s.to_str()).unwrap_or("script");
        let mut out = PathBuf::from(dir);
        util::ensure_parent_dir(&out)?;
        out.push(format!("{}.basilx", rel));
        out
    } else {
        PathBuf::from(bytecode)
    };

    let needs_rebuild = match (util::read_mtime(source), util::read_mtime(&target_path)) {
        (Ok(src), Ok(dst)) => src > dst,
        (Ok(_), Err(_)) => true,
        _ => true,
    };

    if !needs_rebuild { return Ok(CompileOutcome { changed: false, stderr_tail: None, version: None }); }

    // Prefer bcc if available; else fall back to basilc run which may emit bytecode
    let mut stderr_buf = Vec::new();

    #[cfg(feature = "process-runner")]
    {
        // try bcc <source> -o <bytecode>
        let mut cmd = Command::new(if cfg!(target_os = "windows") { "bcc.exe" } else { "bcc" });
        cmd.arg(source).arg("-o").arg(&target_path);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().context("spawn bcc")?;
        let mut stderr_pipe = child.stderr.take();
        let status = tokio::select! {
            res = child.wait() => { res }
            _ = time::sleep(Duration::from_secs(20)) => { 
                let _ = child.kill().await; 
                Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "bcc timeout"))
            }
        };
        match status {
            Ok(st) => {
                if !st.success() {
                    if let Some(mut err) = stderr_pipe.take() {
                        let mut buf = Vec::new();
                        let _ = err.read_to_end(&mut buf).await;
                        stderr_buf.extend_from_slice(&buf);
                    }
                }
            }
            Err(e) => {
                stderr_buf.extend_from_slice(format!("bcc failed: {}", e).as_bytes());
            }
        }
    }

    Ok(CompileOutcome { changed: true, stderr_tail: Some(util::stderr_tail(&stderr_buf, 4096)), version: None })
}
