use adw::prelude::*;
use adw::AlertDialog;
use gtk::{self, glib, prelude::IsA};
use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;

use crate::core::runtime::spawn_tokio;
use crate::utils::kokoros_downloader::KokorosDownloader;
use crate::utils::progress_tracker::ProgressTracker;

pub struct KokorosDownloadDialog {
    dialog: AlertDialog,
    progress_bar: gtk::ProgressBar,
    status_label: gtk::Label,
    downloader: Arc<Mutex<KokorosDownloader>>,
    is_cancelled: Arc<AtomicBool>,
}

impl KokorosDownloadDialog {
    pub fn new(_parent: &impl IsA<gtk::Widget>) -> Self {
        let dialog = AlertDialog::builder()
            .heading("Preparing Voice Engine")
            .body("Downloading required voice files...")
            .build();

        dialog.add_response("cancel", "Cancel");
        dialog.set_response_appearance("cancel", adw::ResponseAppearance::Destructive);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);

        let status_label = gtk::Label::new(Some("Initializing..."));
        status_label.set_halign(gtk::Align::Start);
        vbox.append(&status_label);

        let progress_bar = gtk::ProgressBar::new();
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("0%"));
        vbox.append(&progress_bar);

        dialog.set_extra_child(Some(&vbox));

        let downloader = Arc::new(Mutex::new(KokorosDownloader::new(
            ProgressTracker::default(),
        )));

        let is_cancelled = Arc::new(AtomicBool::new(false));

        Self {
            dialog,
            progress_bar,
            status_label,
            downloader,
            is_cancelled,
        }
    }

    pub async fn download_and_show(
        &self,
        parent: &impl IsA<gtk::Widget>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.dialog.present(Some(parent));

        let progress_tracker = ProgressTracker::default();
        let progress_callback = progress_tracker.get_progress_callback();

        let (on_complete, on_cancel) = progress_tracker.track_with_progress_bar(&self.progress_bar);

        let downloader = self.downloader.clone();
        let is_cancelled = self.is_cancelled.clone();

        let is_cancelled_for_dialog = is_cancelled.clone();
        self.dialog.connect_response(None, move |_, response| {
            if response == "cancel" {
                is_cancelled_for_dialog.store(true, Ordering::SeqCst);
            }
        });

        let download_task = spawn_tokio(async move {
            let is_cancelled_check = is_cancelled.clone();
            let cancel_check = async move {
                loop {
                    if is_cancelled_check.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            };

            tokio::select! {
                result = async {
                    match downloader
                        .lock()
                        .await
                        .download_required_files(Some(progress_callback))
                        .await
                    {
                        Ok(res) => Ok::<_, Box<dyn std::error::Error + Send + Sync>>(res),
                        Err(e) => Err(format!("Download failed: {}", e).into()),
                    }
                } => result,
                _ = cancel_check => Ok(())
            }
        });

        let download_result = download_task.await;

        match download_result {
            Ok(_) => {
                on_complete();
                self.status_label.set_text("Voice engine ready!");

                let dialog_clone = self.dialog.clone();
                glib::timeout_add_seconds_local(1, move || {
                    dialog_clone.close();
                    glib::ControlFlow::Break
                });

                Ok(())
            }
            Err(e) => {
                on_cancel();
                self.dialog.close();

                let err_msg = format!("Failed to download Kokoros files: {}", e);
                Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, err_msg))
                        as Box<dyn Error + Send + Sync>,
                )
            }
        }
    }
}
