use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use std::time::Duration;
use tracing::Level;

#[derive(Clone, Debug)]
pub struct Config {
    pub root: Utf8PathBuf,
    pub host: String,
    pub port: u16,
    pub upload_limit: usize,
    pub script_timeout: Duration,
    pub watch: bool,
    pub etag: bool,
    pub index: String,
    pub bytecode_dir: Option<Utf8PathBuf>,
    pub log: Level,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "basil-serve")]
#[command(about = "Dev web server for Basil projects (static + CGI + templates)")]
struct Args {
    #[arg(short = 'v', long, action = clap::ArgAction::SetTrue, help = "Show version information")]
    version: bool,

    #[arg(long, env = "BASIL_SERVE_ROOT")]
    root: String,

    #[arg(long, env = "BASIL_SERVE_HOST", default_value = "127.0.0.1")]
    host: String,

    #[arg(long, env = "BASIL_SERVE_PORT", default_value_t = 8000)]
    port: u16,

    #[arg(long, env = "BASIL_SERVE_UPLOAD_LIMIT", default_value_t = 10 * 1024 * 1024)]
    upload_limit: usize,

    #[arg(long, env = "BASIL_SERVE_SCRIPT_TIMEOUT", default_value_t = 10)]
    script_timeout: u64,

    #[arg(long, env = "BASIL_SERVE_WATCH", default_value_t = false)]
    watch: bool,

    #[arg(long = "no-etag", env = "BASIL_SERVE_NO_ETAG", default_value_t = false)]
    no_etag: bool,

    #[arg(long, env = "BASIL_SERVE_INDEX", default_value = "index.html")]
    index: String,

    #[arg(long, env = "BASIL_SERVE_BYTECODE_DIR")]
    bytecode_dir: Option<String>,

    #[arg(long, env = "BASIL_SERVE_LOG", default_value = "info")]
    log: String,
}

impl Config {
    pub fn from_env_and_args<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let args_vec: Vec<String> = args.into_iter().collect();
        // clap Parser from any iterator requires setting from argv-like; we use try_parse_from
        // Build argv vector including a program name for clap
        let argv: Vec<String> = if args_vec.is_empty() {
            std::env::args().collect()
        } else {
            std::iter::once("basil-serve".to_string())
                .chain(args_vec.into_iter())
                .collect()
        };
        let a = Args::try_parse_from(argv)?;

        if a.root.is_empty() {
            return Err(anyhow!("--root is required"));
        }

        let root = Utf8PathBuf::from(a.root);
        let bytecode_dir = a.bytecode_dir.map(Utf8PathBuf::from);
        let log_level = match a.log.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            other => {
                tracing::warn!(level = %other, "Unknown log level, defaulting to info");
                Level::INFO
            }
        };

        Ok(Self {
            root,
            host: a.host,
            port: a.port,
            upload_limit: a.upload_limit,
            script_timeout: Duration::from_secs(a.script_timeout),
            watch: a.watch,
            etag: !a.no_etag,
            index: a.index,
            bytecode_dir,
            log: log_level,
        })
    }
}
