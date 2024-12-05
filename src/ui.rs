use gtk::{
    prelude::*, AlertDialog, Application, ApplicationWindow, Box as GtkBox, Button, ListBox,
    Orientation, ScrolledWindow,
};
use std::error::Error;

use crate::hf::{HuggingFace, Voice};

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
            let row_box = self.add_voice_row(voice);
            list_box.append(&row_box);
        }

        Ok(list_box)
    }

    fn add_voice_row(&self, voice: Voice) -> GtkBox {
        let row_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();

        let download_button = self.add_download_button();
        let remove_button = Button::with_label("Remove");
        let row = gtk::Label::new(Some(&voice.key));

        row_box.append(&row);
        row_box.append(&download_button);
        row_box.append(&remove_button);

        row_box
    }

    fn add_download_button(&self) -> Button {
        let download_button = Button::with_label("Download");
        let window = self.window.clone();
        download_button.connect_clicked(move |_| {});
        download_button
    }

    fn _show_download_alert(window: &ApplicationWindow, dialog: &str) {
        let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
        alert_dialog.show(Some(window));
    }
}
