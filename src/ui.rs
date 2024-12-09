use gtk::{
    glib::{self, clone},
    prelude::*,
    AlertDialog, Application, ApplicationWindow, Builder, Button, Grid, Label, SearchEntry,
};
use std::{cell::RefCell, collections::BTreeMap};
use std::{error::Error, rc::Rc};

use crate::{
    dispatcher::SpeechDispatcher,
    hf::{Voice, VoiceManager},
};

pub struct UI {
    window: ApplicationWindow,
    app_window: Builder,
    voices_box: Builder,
}

impl UI {
    pub fn new(app: &Application) -> Self {
        let app_window = Builder::from_resource("/org/piper-reader/app_window.ui");
        let voices_box = Builder::from_resource("/org/piper-reader/voices_box.ui");

        let window: ApplicationWindow = app_window.object("window").expect("Failed to load window");
        window.set_application(Some(app));

        Self {
            window,
            app_window,
            voices_box,
        }
    }

    pub fn setup_ui(&self) {
        self.window.present();

        SpeechDispatcher::initialize_config().expect("Failed initializing config");

        let scrolled_window: gtk::ScrolledWindow = self
            .app_window
            .object("scrolled_window")
            .expect("Failed to load scrolled window");

        let voices_box_widget: gtk::Box = self
            .voices_box
            .object("box_container")
            .expect("Failed to load voices box");

        scrolled_window.set_child(Some(&voices_box_widget));

        self.list_avaliable_voices()
            .expect("Failed to list available voices: {}")
    }

    fn list_avaliable_voices(&self) -> Result<(), Box<dyn Error>> {
        let voices = VoiceManager::list_all_avaliable_voices()?;

        let grid: gtk::Grid = self
            .voices_box
            .object("voices_grid")
            .expect("Failed to load voices grid");

        let search_entry: SearchEntry = self
            .voices_box
            .object("search_entry")
            .expect("Failed to load search entry");

        self.filter_voices(&search_entry, &grid, voices.clone());

        for (i, (_, voice)) in voices.iter().enumerate() {
            let voice_rc = Rc::new(RefCell::new(voice.clone()));
            Self::add_voice_row(&self.window, voice_rc, &grid, i as i32);
        }

        Ok(())
    }

    fn filter_voices(
        &self,
        search_entry: &SearchEntry,
        grid: &Grid,
        voices: BTreeMap<String, Voice>,
    ) {
        search_entry.connect_search_changed(clone!(
            #[weak]
            grid,
            #[weak(rename_to=window)]
            self.window,
            move |search| {
                let input = search.text().to_lowercase();
                println!("{}", input);
                clear_grid(&grid);
                for (i, (_, voice)) in voices.iter().enumerate() {
                    if input.is_empty() || voice.key.to_lowercase().contains(&input) {
                        let voice_rc = Rc::new(RefCell::new(voice.clone()));
                        Self::add_voice_row(&window, voice_rc, &grid, i as i32);
                    }
                }
            }
        ));
    }

    fn add_voice_row(
        window: &ApplicationWindow,
        voice_rc: Rc<RefCell<Voice>>,
        grid: &Grid,
        index: i32,
    ) {
        let label = Label::new(Some(&voice_rc.borrow().key));
        label.set_halign(gtk::Align::Start);
        let download_button = Self::add_download_button(window, Rc::clone(&voice_rc));
        let remove_button = Self::add_remove_button(window, Rc::clone(&voice_rc));

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

    fn add_download_button(window: &ApplicationWindow, voice: Rc<RefCell<Voice>>) -> Button {
        let download_button = Button::with_label("Download");

        if voice.borrow().downloaded {
            download_button.set_sensitive(false);
        }

        download_button.connect_clicked(clone!(
            #[weak]
            window,
            move |button| {
                button.set_sensitive(false);
                let mut mut_voice = voice.borrow_mut();
                if let Err(e) = VoiceManager::download_voice(&mut_voice.files) {
                    let err_msh = format!("Failed to download voice: {}", e);
                    Self::show_download_alert(&window, &err_msh);
                    mut_voice.downloaded = true;
                }
            }
        ));

        download_button
    }

    fn add_remove_button(window: &ApplicationWindow, voice: Rc<RefCell<Voice>>) -> Button {
        let remove_button = Button::with_label("Remove");
        remove_button.set_sensitive(false);

        if voice.borrow().downloaded {
            remove_button.set_sensitive(true);
        }

        remove_button.connect_clicked(clone!(
            #[weak]
            window,
            move |button| {
                button.set_sensitive(false);
                let mut mut_voice = voice.borrow_mut();
                if let Err(e) = VoiceManager::delete_voice(&mut_voice.files) {
                    let err_msg = format!("Failed to remove voice: {}", e);
                    Self::show_download_alert(&window, &err_msg);
                    mut_voice.downloaded = true;
                }
            }
        ));

        remove_button
    }

    fn show_download_alert(window: &ApplicationWindow, dialog: &str) {
        let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
        alert_dialog.show(Some(window));
    }
}

fn clear_grid(grid: &Grid) {
    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }
}
