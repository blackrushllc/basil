use anyhow::Result;
use basil_web::{config::Config, serve};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() && (args[0] == "-v" || args[0] == "--version") {
        basil_common::print_suite_version("basil-serve", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // We'll let the library parse args+env
    let cfg = Config::from_env_and_args(args.into_iter())?;

    // Initialize tracing in library serve(); we also warn here on public bind
    if cfg.host == "0.0.0.0" || cfg.host == "::" {
        eprintln!("Warning: Dev server binding to public interface. Do NOT use in production.");
    }

    // Run server
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { serve(cfg).await })
}
