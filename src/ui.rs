use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, Label, ListBox, Orientation,
    ScrolledWindow, SelectionMode,
};

use crate::downloader::Downloader; // Add this line
use crate::hf::{HuggingFace, Voice};
use std::error::Error;
use std::sync::{Arc, Mutex}; // Add these lines

const APP_ID: &str = "org.piper.reader";

pub struct UI {
    app: Application,
}

impl UI {
    pub fn new() -> Self {
        Self {
            app: Application::builder().application_id(APP_ID).build(),
        }
    }

    pub fn run(self) -> glib::ExitCode {
        let app_weak = self.app.downgrade();
        self.app.connect_activate(move |_| {
            let app = app_weak.upgrade().expect("Application not found");
            let window = ApplicationWindow::builder()
                .application(&app)
                .title("Piper Reader")
                .default_width(600)
                .default_height(400)
                .build();

            let list_box = ListBox::builder()
                .selection_mode(SelectionMode::None)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();

            let scrolled_window = ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .child(&list_box)
                .build();

            if let Err(e) = UI::populate_voices(&list_box) {
                eprintln!("Error populating voices: {}", e);
                UI::show_error_in_list(&list_box, &e.to_string());
            }

            window.set_child(Some(&scrolled_window));
            window.present();
        });

        self.app.run()
    }

    fn populate_voices(list_box: &ListBox) -> Result<(), Box<dyn Error>> {
        let hf = HuggingFace::new();
        let voices = hf.parse_avaliable_voices()?;

        let list_box_arc = Arc::new(Mutex::new(list_box.clone())); // Wrap ListBox in Arc<Mutex>
        for voice in voices {
            UI::add_voice_row(Arc::clone(&list_box_arc), voice); // Clone the Arc
        }

        Ok(())
    }

    fn add_voice_row(list_box: Arc<Mutex<ListBox>>, voice: Voice) {
        let voice_name = voice.name.clone();

        let row_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        let label = Label::builder()
            .label(&voice_name)
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();
        row_box.append(&label);

        let download_button = Button::with_label("Download");
        let voice_name_download = voice_name.clone();
        let list_box_clone = Arc::clone(&list_box); // Clone the Arc
        download_button.connect_clicked(move |_| {
            let hf = HuggingFace::new();
            match hf.get_avaliable_voices() {
                Ok(raw_json) => {
                    let value_data: serde_json::Value = serde_json::from_str(&raw_json).unwrap();
                    if let Some(voice_url) = value_data[&voice_name_download].as_str() {
                        let save_path = format!("./downloads/{}.wav", voice_name_download);
                        match Downloader::download_file(voice_url.to_string()) {
                            Ok(response) => {
                                if let Err(e) = Downloader::save_file(&save_path, response) {
                                    eprintln!("Failed to save file: {}", e);
                                    UI::show_error_in_list(&list_box_clone.lock().unwrap(), &e.to_string());
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to download file: {}", e);
                                UI::show_error_in_list(&list_box_clone.lock().unwrap(), &e.to_string());
                            }
                        }
                    } else {
                        eprintln!("Voice URL not found for: {}", voice_name_download);
                        UI::show_error_in_list(&list_box_clone.lock().unwrap(), &format!("Voice URL not found for: {}", voice_name_download));
                    }
                }
                Err(e) => {
                    eprintln!("Error getting available voices: {}", e);
                    UI::show_error_in_list(&list_box_clone.lock().unwrap(), &e.to_string());
                }
            }
        });
        row_box.append(&download_button);

        let remove_button = Button::with_label("Remove");
        let voice_name_remove = voice_name.clone();
        remove_button.connect_clicked(move |_| {
            println!("Removing voice: {}", voice_name_remove);
        });
        row_box.append(&remove_button);

        list_box.lock().unwrap().append(&row_box); // Use lock to access the ListBox
    }

    fn show_error_in_list(list_box: &ListBox, error_msg: &str) {
        let error_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        let error_label = Label::builder()
            .label(&format!("Error: {}", error_msg))
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();

        error_box.append(&error_label);
        list_box.append(&error_box);
    }
}
