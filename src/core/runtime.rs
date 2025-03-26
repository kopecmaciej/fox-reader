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

pub async fn spawn_tokio<F, T, E>(fut: F) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static + Send,
{
    match runtime().spawn(fut).await {
        Ok(inner_result) => match inner_result {
            Ok(value) => Ok(value),
            Err(task_error) => Err(task_error.into()),
        },
        Err(join_error) => Err(Box::new(join_error)),
    }
}
