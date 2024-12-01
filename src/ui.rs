use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, ListBox, Label};
use std::collections::HashMap;

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

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);

        button.connect_clicked(move |_| {
            if let Err(e) = self.fetch_and_display_voices(&list_box) {
                eprintln!("Error fetching and displaying voices: {}", e);
            }
        });

        let window = ApplicationWindow::builder()
            .application(&self.app)
            .title("My GTK App")
            .child(&button)
            .child(&list_box)
            .build();

        window.present();
    }

    fn fetch_and_display_voices(&self, list_box: &ListBox) -> Result<(), Box<dyn Error>> {
        let hf = HuggingFace::new();
        let voices = hf.parse_avaliable_voices()?;

        for voice in voices {
            let label = Label::builder()
                .label(&voice.name)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(12)
                .margin_end(12)
                .build();

            list_box.append(&label);
        }

        Ok(())
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.connect_activate(|app| {
            let ui = UI { app: app.clone() };
            ui.build_ui();
        });

        self.app.run()
    }
}
