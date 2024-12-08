use gtk::{
    prelude::*, AlertDialog, Application, ApplicationWindow, Builder, Button, Grid, Label,
    SearchEntry,
};
use std::cell::RefCell;
use std::{error::Error, rc::Rc};

use crate::dispatcher::SpeechDispatcher;
use crate::hf::{Voice, VoiceManager};

pub struct UI {
    window: ApplicationWindow,
    hf: Rc<VoiceManager>,
    dispatcher: SpeechDispatcher,
    search_entry: SearchEntry,
    app_window: Builder,
    voices_box: Builder,
}

impl UI {
    pub fn new(app: &Application) -> Self {
        let app_window = Builder::from_resource("/org/piper-reader/app_window.ui");
        let voices_box = Builder::from_resource("/org/piper-reader/voices_box.ui");

        let window: ApplicationWindow = app_window.object("window").expect("Failed to load window");
        window.set_application(Some(app));

        let search_entry: SearchEntry = voices_box
            .object("search_entry")
            .expect("Failed to load search entry");

        let hf = Rc::new(VoiceManager::new());
        let dispatcher = SpeechDispatcher::new();

        Self {
            window,
            hf,
            dispatcher,
            search_entry,
            app_window,
            voices_box,
        }
    }

    pub fn setup_ui(&self) {
        self.window.present();

        self.dispatcher
            .initialize_config()
            .expect("Failed initializing config");

        match self.list_avaliable_voices() {
            Ok(()) => {
                let scrolled_window: gtk::ScrolledWindow = self
                    .app_window
                    .object("scrolled_window")
                    .expect("Failed to load scrolled window");

                // Use the correct widget from the builder
                let voices_box_widget: gtk::Box = self
                    .voices_box
                    .object("box_container")
                    .expect("Failed to load voices box");

                scrolled_window.set_child(Some(&voices_box_widget));
            }
            Err(e) => eprintln!("Failed to list available voices: {}", e),
        }
    }

    fn list_avaliable_voices(&self) -> Result<(), Box<dyn Error>> {
        let voices = self.hf.list_all_avaliable_voices()?;

        let grid: gtk::Grid = self
            .voices_box
            .object("voices_grid")
            .expect("Failed to load voices grid");

        for (i, (_, voice)) in voices.into_iter().enumerate() {
            self.add_voice_row(voice, &grid, i as i32);
        }

        Ok(())
    }

    fn add_voice_row(&self, voice: Voice, grid: &Grid, index: i32) {
        let voice_rc = Rc::new(RefCell::new(voice));

        let label = Label::new(Some(&voice_rc.borrow().key));
        label.set_halign(gtk::Align::Start);
        let download_button = self.add_download_button(Rc::clone(&voice_rc));
        let remove_button = self.add_remove_button(Rc::clone(&voice_rc));

        download_button
            .bind_property("sensitive", &remove_button, "sensitive")
            .invert_boolean()
            .build();

        remove_button
            .bind_property("sensitive", &download_button, "sensitive")
            .invert_boolean()
            .build();

        grid.attach(&label, 0, index, 1, 1);
        grid.attach(&download_button, 1, index, 1, 1);
        grid.attach(&remove_button, 2, index, 1, 1);
    }

    fn add_download_button(&self, voice: Rc<RefCell<Voice>>) -> Button {
        let download_button = Button::with_label("Download");
        let window = self.window.clone();
        let hf = self.hf.clone();

        if voice.borrow().downloaded {
            download_button.set_sensitive(false);
        }

        download_button.connect_clicked(move |button| {
            button.set_sensitive(false);
            let mut mut_voice = voice.borrow_mut();
            if let Err(e) = hf.download_voice(&mut_voice.files) {
                let err_msh = format!("Failed to download voice: {}", e);
                Self::show_download_alert(&window, &err_msh);
                mut_voice.downloaded = true;
            }
        });

        download_button
    }

    fn add_remove_button(&self, voice: Rc<RefCell<Voice>>) -> Button {
        let remove_button = Button::with_label("Remove");
        remove_button.set_sensitive(false);
        let window = self.window.clone();
        let hf = self.hf.clone();

        if voice.borrow().downloaded {
            remove_button.set_sensitive(true);
        }

        remove_button.connect_clicked(move |button| {
            button.set_sensitive(false);
            let mut mut_voice = voice.borrow_mut();
            if let Err(e) = hf.delete_voice(&mut_voice.files) {
                let err_msg = format!("Failed to remove voice: {}", e);
                Self::show_download_alert(&window, &err_msg);
                mut_voice.downloaded = true;
            }
        });

        remove_button
    }

    fn show_download_alert(window: &ApplicationWindow, dialog: &str) {
        let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
        alert_dialog.show(Some(window));
    }
}
