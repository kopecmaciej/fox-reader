use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Label, ListBox, ListBoxRow};
use std::error::Error;

use crate::hf::HuggingFace;

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
        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);

        if let Err(e) = self.fetch_and_display_voices(&list_box) {
            eprintln!("Error fetching and displaying voices: {}", e);
        }

        let window = ApplicationWindow::builder()
            .application(&self.app)
            .title("Piper Reader")
            .child(&list_box)
            .build();

        window.present();
    }

    fn fetch_and_display_voices(&self, list_box: &ListBox) -> Result<(), Box<dyn Error>> {
        let hf = HuggingFace::new();
        let voices = hf.parse_avaliable_voices()?;

        for voice in voices {
            let row = ListBoxRow::new();

            let label = Label::builder()
                .label(voice.name)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(12)
                .margin_end(12)
                .build();

            let download_button = Button::with_label("Download");
            let voice_name = voice.name.clone();
            download_button.connect_clicked(clone!(@strong voice_name => move |_| {
                println!("Downloading voice: {}", voice_name);
            }));

            let remove_button = Button::with_label("Remove");
            let voice_name = voice.name.clone();
            remove_button.connect_clicked(clone!(@strong voice_name => move |_| {
                println!("Removing voice: {}", voice_name);
            }));

            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            hbox.append(&label);
            hbox.append(&download_button);
            hbox.append(&remove_button);

            row.set_child(Some(&hbox));
            list_box.append(&row);
        }

        Ok(())
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.connect_activate(clone!(@strong self.app as app => move |_| {
            let ui = UI { app };
            ui.build_ui();
        }));

        self.app.run()
    }
}
