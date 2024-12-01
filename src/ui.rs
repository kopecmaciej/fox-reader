use gtk::glib;
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
        let button = Button::builder()
            .label("Press me!")
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        button.connect_clicked(|button| {
            button.set_label("Hello World!");
        });

        let window = ApplicationWindow::builder()
            .application(&self.app)
            .title("My GTK App")
            .child(&button)
            .build();

        window.present();
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.connect_activate(|app| {
            let ui = UI { app: app.clone() };
            ui.build_ui();
        });

        self.app.run()
    }
}
