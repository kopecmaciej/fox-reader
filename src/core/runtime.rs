use gtk::glib;
use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

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
