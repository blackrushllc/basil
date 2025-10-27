use anyhow::Result;
use basil_web::{config::Config, serve};

fn main() -> Result<()> {
    // We'll let the library parse args+env
    let cfg = Config::from_env_and_args(std::env::args().skip(1).map(|s| s.to_string()))?;

    // Initialize tracing in library serve(); we also warn here on public bind
    if cfg.host == "0.0.0.0" || cfg.host == "::" {
        eprintln!("Warning: Dev server binding to public interface. Do NOT use in production.");
    }

    // Run server
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { serve(cfg).await })
}
