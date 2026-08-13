use camino::Utf8PathBuf;
use tokio_util::sync::CancellationToken;

pub mod cgi;
pub mod compile;
pub mod config;
pub mod handlers;
pub mod script;
pub mod server;
pub mod static_files;
pub mod template;
pub mod util;

pub use server::serve;

#[derive(Clone)]
pub struct AppState {
    pub cfg: config::Config,
    pub etag_enabled: bool,
    pub bytecode_root: Utf8PathBuf,
    pub cancellation: CancellationToken,
}

impl AppState {
    pub fn new(cfg: config::Config) -> Self {
        let etag_enabled = cfg.etag;
        let bytecode_root = cfg.bytecode_dir.clone().unwrap_or_else(|| cfg.root.clone());
        let cancellation = CancellationToken::new();
        Self {
            cfg,
            etag_enabled,
            bytecode_root,
            cancellation,
        }
    }
}
