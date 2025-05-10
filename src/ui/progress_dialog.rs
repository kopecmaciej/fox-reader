use adw::{prelude::*, MessageDialog};
use gtk::{self, Box as GtkBox, ProgressBar};
use std::sync::{Arc, Mutex};

use crate::utils::progress_tracker::ProgressTracker;

pub struct ProgressDialog {
    dialog: MessageDialog,
    progress_bar: ProgressBar,
    tracker: ProgressTracker,
}

impl ProgressDialog {
    pub fn new(title: &str, message: &str, parent_window: Option<&gtk::Window>) -> Self {
        let progress_bar = ProgressBar::builder()
            .show_text(true)
            .text("0%")
            .hexpand(true)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(20)
            .margin_end(20)
            .build();

        let dialog = MessageDialog::builder()
            .heading(title)
            .body(message)
            .modal(true)
            .build();

        // Add the progress bar to the dialog
        if let Some(message_area) = dialog.child() {
            // Try to find a content area where we can add our progress bar
            if let Some(child) = message_area.first_child() {
                if let Ok(content_box) = child.downcast::<GtkBox>() {
                    content_box.append(&progress_bar);
                } else {
                    // If we can't find a GtkBox to append to, create one
                    let content_box = GtkBox::new(gtk::Orientation::Vertical, 10);
                    content_box.append(&progress_bar);
                    if let Ok(box_widget) = message_area.downcast::<GtkBox>() {
                        box_widget.append(&content_box);
                    }
                }
            }
        }

        // Present the dialog
        if let Some(window) = parent_window {
            dialog.set_transient_for(Some(window));
        }
        dialog.present();

        Self {
            dialog,
            progress_bar,
            tracker: ProgressTracker::default(),
        }
    }

    pub fn get_progress_callback(&self) -> Arc<Mutex<dyn FnMut(f32) + Send>> {
        let progress_callback = self.tracker.get_progress_callback();
        let progress_bar_weak = self.progress_bar.downgrade();

        // Connect the progress bar updates
        self.tracker.connect_to_progress_bar(&self.progress_bar);

        // Add text update
        let _timeout_id =
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                if let Some(pb) = progress_bar_weak.upgrade() {
                    let fraction = pb.fraction();
                    pb.set_text(Some(&format!("{:.0}%", fraction * 100.0)));
                    gtk::glib::ControlFlow::Continue
                } else {
                    gtk::glib::ControlFlow::Break
                }
            });

        progress_callback
    }

    pub fn close(&self) {
        self.dialog.close();
    }

    pub fn update_message(&self, message: &str) {
        self.dialog.set_body(message);
    }
    
    pub fn get_progress_bar(&self) -> &ProgressBar {
        &self.progress_bar
    }
}
