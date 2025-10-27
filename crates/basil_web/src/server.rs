use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

use crate::{config::Config, handlers, AppState};

pub async fn serve(cfg: Config) -> anyhow::Result<()> {
    // Init tracing only once
    if tracing::dispatcher::has_been_set() == false {
        let level = match cfg.log {
            tracing::Level::TRACE => "trace",
            tracing::Level::DEBUG => "debug",
            tracing::Level::INFO => "info",
            tracing::Level::WARN => "warn",
            tracing::Level::ERROR => "error",
        };
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
        fmt().with_env_filter(filter).try_init().ok();
    }

    if cfg.host == "0.0.0.0" || cfg.host == "::" {
        tracing::warn!(host = %cfg.host, "Dev server binding to a public interface. Do NOT use in production.");
    } else if cfg.host != "127.0.0.1" && cfg.host != "[::1]" {
        tracing::warn!(host = %cfg.host, "Non-localhost bind; this dev server is not production-hardened.");
    }

    let state = Arc::new(AppState::new(cfg.clone()));
    let app = Router::new()
        .fallback(get(handlers::entry))
        .with_state(state.clone());

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tracing::info!("basil-serve listening on http://{}", bound);

    // TODO: watch filesystem changes with notify to invalidate caches

    axum::serve(listener, app).await?;
    Ok(())
}
