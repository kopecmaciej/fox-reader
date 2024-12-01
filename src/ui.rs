use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, Label, ListBox,
    Orientation, SelectionMode,
};

use crate::hf::{HuggingFace, Voice};
use std::error::Error;

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

    pub fn run(&self) -> glib::ExitCode {
        self.app.connect_activate(glib::clone!(@weak self.app as app => move |_| {
            let window = self.build_window(&app);
            window.present();
        }));

        self.app.run()
    }

    fn build_window(&self, app: &Application) -> ApplicationWindow {
        // Create main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Piper Reader")
            .default_width(600)
            .default_height(400)
            .build();

        // Create and configure list box
        let list_box = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        // Populate voices
        if let Err(e) = self.populate_voices(&list_box) {
            eprintln!("Error populating voices: {}", e);
            self.show_error_in_list(&list_box, &e.to_string());
        }

        window.set_child(Some(&list_box));
        window
    }

    fn populate_voices(&self, list_box: &ListBox) -> Result<(), Box<dyn Error>> {
        let hf = HuggingFace::new();
        let voices = hf.parse_avaliable_voices()?;

        for voice in voices {
            self.add_voice_row(list_box, voice);
        }

        Ok(())
    }

    fn add_voice_row(&self, list_box: &ListBox, voice: Voice) {
        let voice_name = voice.name.clone();
        
        // Create row container
        let row_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        // Add voice name label
        let label = Label::builder()
            .label(&voice_name)
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();
        row_box.append(&label);

        // Add download button
        let download_button = Button::with_label("Download");
        download_button.connect_clicked(glib::clone!(@strong voice_name => move |_| {
            println!("Downloading voice: {}", voice_name);
        }));
        row_box.append(&download_button);

        // Add remove button
        let remove_button = Button::with_label("Remove");
        remove_button.connect_clicked(glib::clone!(@strong voice_name => move |_| {
            println!("Removing voice: {}", voice_name);
        }));
        row_box.append(&remove_button);

        list_box.append(&row_box);
    }

    fn show_error_in_list(&self, list_box: &ListBox, error_msg: &str) {
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
