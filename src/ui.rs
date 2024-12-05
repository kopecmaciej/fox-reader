use gtk::{
    prelude::*, AlertDialog, Application, ApplicationWindow, Box as GtkBox, Button, ListBox,
    Orientation, ScrolledWindow,
};
use std::{error::Error, rc::Rc};

use crate::hf::{Voice, VoiceManager};

pub struct UI {
    window: ApplicationWindow,
    hf: Rc<VoiceManager>,
}

impl UI {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Piper Reader")
            .build();

        window.present();

        let hf = Rc::new(VoiceManager::new());

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

        for (_, voice) in voices {
            let row_box = self.add_voice_row(voice);
            list_box.append(&row_box);
        }

        Ok(list_box)
    }

    fn add_voice_row(&self, voice: Voice) -> GtkBox {
        let voice_rc = Rc::new(voice);
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

        let row = gtk::Label::new(Some(&voice_rc.key));
        let download_button = self.add_download_button(Rc::clone(&voice_rc));
        let remove_button = self.add_remove_button(Rc::clone(&voice_rc));

        row_box.append(&row);
        row_box.append(&download_button);
        row_box.append(&remove_button);

        row_box
    }

    fn add_download_button(&self, voice: Rc<Voice>) -> Button {
        let download_button = Button::with_label("Download");
        let window = self.window.clone();
        let hf = self.hf.clone();

        download_button.connect_clicked(move |button| {
            button.set_sensitive(false);
            if let Err(e) = hf.download_voice(&voice.files) {
                let err_msh = format!("Failed to download voice: {}", e);
                Self::show_download_alert(&window, &err_msh);
            }
        });

        download_button
    }

    fn add_remove_button(&self, voice: Rc<Voice>) -> Button {
        let remove_button = Button::with_label("Remove");
        let window = self.window.clone();
        let hf = self.hf.clone();

        remove_button.connect_clicked(move |button| {
            button.set_sensitive(false);
            if let Err(e) = hf.delete_voice(&voice.files) {
                let err_msg = format!("Failed to remove voice: {}", e);
                Self::show_download_alert(&window, &err_msg);
            }
        });

        remove_button
    }

    fn show_download_alert(window: &ApplicationWindow, dialog: &str) {
        let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
        alert_dialog.show(Some(window));
    }
}
