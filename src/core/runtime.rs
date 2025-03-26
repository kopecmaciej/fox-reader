use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime")
    })
}

pub async fn spawn_tokio<F, T, E>(fut: F) -> Result<T, Box<dyn std::error::Error + Send>>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static + Send,
{
    match runtime().spawn(fut).await {
        Ok(res) => match res {
            Ok(value) => Ok(value),
            Err(e) => Err(e.into()),
        },
        Err(e) => Err(Box::new(e)),
    }
}
