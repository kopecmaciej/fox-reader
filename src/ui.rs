use gtk::{prelude::*, Button, Orientation};
use gtk::{Application, ApplicationWindow, Box as GtkBox, ListBox, ScrolledWindow};
use std::error::Error;

use crate::hf::HuggingFace;

pub struct UI {
    window: ApplicationWindow,
    hf: HuggingFace,
}

impl UI {
    pub fn new(app: &Application) -> Self {
        let hf = HuggingFace::new();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Piper Reader")
            .build();

        window.present();

        Self { hf, window }
    }

    pub fn setup_ui(&self) {
        match self.list_avaliable_voices() {
            Ok(list_box) => {
                let scrolled_window = self.wrap_in_scrolled_window();
                scrolled_window.set_child(Some(&list_box));
            }
            Err(e) => eprintln!("Failed to list available voices: {}", e),
        }
    }

    fn wrap_in_scrolled_window(&self) -> ScrolledWindow {
        let sw = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        self.window.set_child(Some(&sw));
        sw
    }

    fn list_avaliable_voices(&self) -> Result<ListBox, Box<dyn Error>> {
        let voices = self.hf.parse_avaliable_voices()?;

        let list_box = ListBox::builder().build();

        for voice in voices {
            let row_box = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(12)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(6)
                .margin_end(6)
                .build();

            let download_button = Button::with_label("Download");
            let remove_button = Button::with_label("Remove");
            let row = gtk::Label::new(Some(&voice.name));

            row_box.append(&row);
            row_box.append(&download_button);
            row_box.append(&remove_button);

            list_box.append(&row_box);
        }

        Ok(list_box)
    }
}
