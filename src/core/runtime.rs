use gtk::glib;
use std::{future::Future, sync::OnceLock};

use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?;
    RUNTIME.set(rt).map_err(|_| "Runtime already initialized")?;
    Ok(())
}

pub fn spawn_tokio_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    let rt = RUNTIME.get().expect("Runtime not initialized");

    glib::MainContext::default().spawn_local(async move {
        let _guard = rt.enter();
        future.await;
    });
}
