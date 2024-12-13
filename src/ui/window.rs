use gtk::{
    glib::{self, clone},
    prelude::*,
    Application, ApplicationWindow, Builder, Grid, Label, SearchEntry,
};
use std::{cell::RefCell, collections::BTreeMap};
use std::{error::Error, rc::Rc};

use crate::{
    dispatcher::SpeechDispatcher,
    voice_manager::{Voice, VoiceManager},
};

use super::widgets::{download_button, remove_button, SAVE_VOICE_ICON, SET_VOICE_DEFAULT_ICON};

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

        for (i, (_, voice)) in voices.iter().enumerate() {
            Self::add_voice_row(&self.window, voice, &grid, i as i32);
        }

        self.filter_voices(&search_entry, &grid, voices);

        Ok(())
    }

    fn filter_voices(
        &self,
        search_entry: &SearchEntry,
        grid: &Grid,
        voices: BTreeMap<String, Rc<RefCell<Voice>>>,
    ) {
        search_entry.connect_search_changed(clone!(
            #[weak]
            grid,
            #[weak(rename_to=window)]
            self.window,
            move |search| {
                let input = search.text().to_lowercase();
                clear_grid(&grid);
                for (i, (_, voice)) in voices.iter().enumerate() {
                    if input.is_empty() || voice.borrow().key.to_lowercase().contains(&input) {
                        Self::add_voice_row(&window, voice, &grid, i as i32);
                    }
                }
            }
        ));
    }

    fn add_voice_row(
        window: &ApplicationWindow,
        voice_rc: &Rc<RefCell<Voice>>,
        grid: &Grid,
        index: i32,
    ) {
        let label = Label::new(Some(&voice_rc.borrow().key));
        label.set_halign(gtk::Align::Start);
        let download_button = download_button(window, Rc::clone(&voice_rc));
        let remove_button = remove_button(window, Rc::clone(&voice_rc));

        download_button
            .bind_property("icon-name", &remove_button, "sensitive")
            .transform_to(|_, icon: String| {
                Some(if icon == SAVE_VOICE_ICON { false } else { true })
            })
            .build();

        remove_button
            .bind_property("sensitive", &download_button, "icon-name")
            .transform_to(|_, sensitive: bool| {
                Some(
                    if sensitive {
                        SET_VOICE_DEFAULT_ICON
                    } else {
                        SAVE_VOICE_ICON
                    }
                    .to_string(),
                )
            })
            .build();

        grid.attach(&label, 0, index, 1, 1);
        grid.attach(&download_button, 1, index, 1, 1);
        grid.attach(&remove_button, 2, index, 1, 1);
    }
}

fn clear_grid(grid: &Grid) {
    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }
}
