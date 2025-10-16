use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

// Global Tokio runtime for basilc CLI, so the whole process has a Tokio context.
pub static TOKIO_MAIN_RT: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .thread_name("basil-main")
        .build()
        .expect("Failed to build Tokio runtime (basilc)")
});
