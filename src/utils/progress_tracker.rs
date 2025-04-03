use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use gtk::{
    glib::{self, clone, ControlFlow},
    prelude::*,
};

pub type ProgressCallback = Arc<Mutex<dyn FnMut(f32) + Send>>;

pub struct ProgressTracker {
    progress_value: Arc<Mutex<f32>>,
    timeout_id: Arc<Mutex<Option<glib::SourceId>>>,
    update_interval: Duration,
}

impl ProgressTracker {
    pub fn new(update_interval: Duration) -> Self {
        Self {
            progress_value: Arc::new(Mutex::new(0.0)),
            timeout_id: Arc::new(Mutex::new(None)),
            update_interval,
        }
    }

    pub fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }

    pub fn get_progress_callback(&self) -> ProgressCallback {
        let progress_value = Arc::clone(&self.progress_value);

        Arc::new(Mutex::new(move |progress: f32| {
            let mut value = progress_value.lock().unwrap();
            *value = progress;
        }))
    }

    pub fn connect_to_progress_bar(&self, progress_bar: &gtk::ProgressBar) -> impl Fn() {
        progress_bar.set_visible(true);
        progress_bar.set_fraction(0.0);

        let progress_value = Arc::clone(&self.progress_value);
        let timeout_id_arc = Arc::clone(&self.timeout_id);

        let timeout_id = glib::timeout_add_local(
            self.update_interval,
            clone!(
                #[weak]
                progress_bar,
                #[weak]
                progress_value,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let progress = *progress_value.lock().unwrap();
                    println!("{progress}");
                    progress_bar.set_fraction(progress as f64);
                    ControlFlow::Continue
                }
            ),
        );

        *timeout_id_arc.lock().unwrap() = Some(timeout_id);

        let cleanup = move || {
            if let Some(id) = timeout_id_arc.lock().unwrap().take() {
                id.remove();
            }
        };
        cleanup
    }

    pub fn track_with_progress_bar(
        &self,
        progress_bar: &gtk::ProgressBar,
    ) -> (Box<dyn FnOnce()>, Box<dyn FnOnce()>) {
        let cleanup = Arc::new(self.connect_to_progress_bar(progress_bar));
        let progress_bar_weak = progress_bar.downgrade();
        let cleanup_complete = Arc::clone(&cleanup);
        let cleanup_cancel = Arc::clone(&cleanup);

        // Function to call on successful completion
        let on_complete = Box::new(move || {
            if let Some(pb) = progress_bar_weak.upgrade() {
                pb.set_fraction(1.0);
            }
            cleanup_complete();
        });

        // Clone the progress bar reference for the cancel function
        let progress_bar_weak_cancel = progress_bar.downgrade();

        // Function to call on cancellation
        let on_cancel = Box::new(move || {
            if let Some(pb) = progress_bar_weak_cancel.upgrade() {
                pb.set_visible(false);
            }
            cleanup_cancel();
        });

        (on_complete, on_cancel)
    }

    pub fn get_progress(&self) -> f32 {
        *self.progress_value.lock().unwrap()
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if let Some(id) = self.timeout_id.lock().unwrap().take() {
            id.remove();
        }
    }
}
