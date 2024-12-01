use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button};

const APP_ID: &str = "piper-reader";

pub struct UI {
    app: Application,
}

impl UI {
    pub fn new() -> Self {
        Self {
            app: Application::builder().application_id(APP_ID).build(),
        }
    }

    pub fn build_ui(&self) {
        // Create a button with label and margins
        let button = Button::builder()
            .label("Press me!")
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        // Connect to "clicked" signal of `button`
        button.connect_clicked(|button| {
            // Set the label to "Hello World!" after the button has been clicked on
            button.set_label("Hello World!");
        });

        // Create a window
        let window = ApplicationWindow::builder()
            .application(&self.app)
            .title("My GTK App")
            .child(&button)
            .build();

        // Present window
        window.present();
    }
}
