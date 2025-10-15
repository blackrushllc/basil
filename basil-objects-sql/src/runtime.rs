use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

// Global Tokio runtime, lazily initialized, multi-threaded
pub static TOKIO_RT: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .thread_name("basil-sql")
        .build()
        .expect("Failed to build Tokio runtime (SQL)")
});
