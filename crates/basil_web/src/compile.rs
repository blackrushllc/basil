use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::util;

#[derive(Debug, Clone, Default)]
pub struct CompileOutcome {
    pub changed: bool,
    pub stderr_tail: Option<String>,
    pub version: Option<String>,
}

// Note: basilc handles loading/running and will (re)build bytecode as needed.
// We keep this function to compute the target bytecode path and ensure directories exist,
// but we do not invoke any external compiler here.
pub async fn compile_if_stale(
    source: &Path,
    bytecode: &Path,
    bytecode_dir: Option<&Path>,
) -> Result<CompileOutcome> {
    let target_path = if let Some(dir) = bytecode_dir {
        let rel = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("script");
        let mut out = PathBuf::from(dir);
        util::ensure_parent_dir(&out)?;
        out.push(format!("{}.basilx", rel));
        out
    } else {
        PathBuf::from(bytecode)
    };

    // Ensure the parent directory exists for potential bytecode output (managed by basilc).
    let _ = util::ensure_parent_dir(&target_path);

    // No external compilation is performed here; basilc will rebuild on demand.
    Ok(CompileOutcome {
        changed: false,
        stderr_tail: None,
        version: None,
    })
}
