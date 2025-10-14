use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("basil-aws")
        .build()
        .expect("failed to build tokio runtime")
});
