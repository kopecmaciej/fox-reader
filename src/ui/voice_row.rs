use std::cell::RefCell;

use crate::core::runtime::runtime;
use crate::core::speech_dispatcher::SpeechDispatcher;
use crate::core::voice_manager::{Voice, VoiceManager};
use crate::ui::dialogs;
use adw::subclass::prelude::*;
use adw::Spinner;
use glib::Properties;
use gtk::glib::{self, clone};
use gtk::{prelude::*, Button};

pub const PLAY_ICON: &str = "media-playback-start-symbolic";
pub const DOWNLOAD_VOICE_ICON: &str = "folder-download-symbolic";
pub const SET_AS_DEFAULT_ICON: &str = "starred";
pub const DELETE_VOICE_ICON: &str = "edit-delete";
pub const DEFAULT_VOICE_ICON: &str = "object-select";

mod imp {
    use std::{cell::OnceCell, collections::HashMap};

    use crate::core::voice_manager::File;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VoiceRow)]
    pub struct VoiceRow {
        #[property(get, set)]
        pub name: OnceCell<String>,
        #[property(get, set)]
        pub key: OnceCell<String>,
        #[property(get, set)]
        pub country: OnceCell<String>,
        #[property(get, set)]
        pub language_code: OnceCell<String>,
        #[property(get, set)]
        pub quality: OnceCell<String>,
        #[property(get, set)]
        pub downloaded: RefCell<bool>,
        #[property(get, set)]
        pub is_default: RefCell<bool>,
        pub files: RefCell<HashMap<String, File>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VoiceRow {
        const NAME: &'static str = "VoiceRow";
        type Type = super::VoiceRow;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for VoiceRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
}

glib::wrapper! {
    pub struct VoiceRow(ObjectSubclass<imp::VoiceRow>);
}

impl VoiceRow {
    pub fn new(voice: Voice) -> Self {
        let obj: Self = glib::Object::builder()
            .property("name", &voice.name)
            .property("key", &voice.key)
            .property("country", &voice.language.name_english)
            .property("language_code", &voice.language.code)
            .property("quality", &voice.quality)
            .property("downloaded", voice.downloaded)
            .property("is_default", voice.is_default.unwrap_or(false))
            .build();
        obj.imp().files.replace(voice.files);
        obj
    }

    pub fn setup_play_button() -> Button {
        Button::builder().icon_name(PLAY_ICON).build()
    }

    pub fn setup_action_buttons() -> (Button, Button, Button) {
        let download_button = Button::builder().icon_name(DOWNLOAD_VOICE_ICON).build();
        let set_default_button = Button::builder().icon_name(SET_AS_DEFAULT_ICON).build();
        let delete_button = Button::builder().icon_name(DELETE_VOICE_ICON).build();

        // on download delete btn becomes sensitive
        download_button
            .bind_property("sensitive", &delete_button, "sensitive")
            .invert_boolean()
            .sync_create()
            .build();

        // on download set default btn becomes sensitive
        download_button
            .bind_property("sensitive", &set_default_button, "sensitive")
            .invert_boolean()
            .sync_create()
            .build();

        // on delete set default btn becomes insensitive
        // also default btn will become insensitive as it's inverted from download btn
        delete_button
            .bind_property("sensitive", &download_button, "sensitive")
            .invert_boolean()
            .sync_create()
            .build();

        (download_button, set_default_button, delete_button)
    }

    pub fn handle_play_actions(&self, play_button: &Button) {
        play_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                button.set_sensitive(false);
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    async move {
                        let file_paths = this.imp().files.borrow().clone().into_keys().collect();

                        match VoiceManager::download_voice_samples(file_paths) {
                            Ok(audio_data) => {
                                if let Err(e) = VoiceManager::play_audio_data(audio_data) {
                                    let err_msg =
                                        format!("Failed to play voice sample. \nDetails: {}", e);
                                    dialogs::show_error_dialog(&err_msg, &button);
                                };
                            }
                            Err(e) => {
                                let err_msg =
                                    format!("Failed to play voice sample. \nDetails: {}", e);
                                dialogs::show_error_dialog(&err_msg, &button);
                            }
                        };

                        button.set_sensitive(true);
                    }
                ));
            }
        ));
    }

    pub fn handle_download_actions(&self, download_button: &Button) {
        download_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    async move {
                        let spinner = Spinner::builder().margin_start(8).margin_end(8).build();
                        spinner.set_visible(true);

                        let grid = button.parent().and_downcast::<gtk::Grid>();
                        if let Some(grid) = &grid {
                            grid.remove(&button);
                            grid.attach(&spinner, 0, 0, 1, 1);
                        }
                        let file_names = this.imp().files.borrow().clone().into_keys().collect();
                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(file_names).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        if let Err(e) = SpeechDispatcher::add_new_voice_to_config(
                            &this.language_code(),
                            &this.name(),
                            &this.key(),
                        ) {
                            let err_msg =
                                format!("Failed to add voice to config. \nDetails: {}", e);
                            dialogs::show_error_dialog(&err_msg, &button);
                        }
                        if let Some(grid) = grid {
                            grid.remove(&spinner);
                            grid.attach(&button, 0, 0, 1, 1);
                        }
                        button.set_sensitive(false);
                        this.set_downloaded(true);
                    }
                ));
            }
        ));
    }

    pub fn handle_delete_actions(&self, remove_button: &Button) {
        remove_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                let files = this.imp().files.borrow().clone().into_keys().collect();
                if let Err(e) = VoiceManager::delete_voice(files) {
                    let err_msg = format!("Failed to delete voice. \nDetails: {}", e);
                    dialogs::show_error_dialog(&err_msg, button);
                }
                if let Err(e) = SpeechDispatcher::delete_voice_from_config(
                    &this.language_code(),
                    &this.name(),
                    &this.key(),
                ) {
                    let err_msg = format!("Failed to update config file. \nDetails: {}", e);
                    dialogs::show_error_dialog(&err_msg, button);
                };
                button.set_sensitive(false);
                this.set_downloaded(false);
            }
        ));
    }

    pub fn handle_set_default_actions(&self, set_default_button: &Button) {
        set_default_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                if let Err(e) = SpeechDispatcher::set_default_voice(&this.key()) {
                    let err_msg = format!("Failed to set voice as default. \nDetails: {}", e);
                    dialogs::show_error_dialog(&err_msg, button);
                }
                this.set_is_default(true);
            }
        ));

        self.bind_property("is_default", set_default_button, "icon-name")
            .transform_to(|_, is_default: bool| {
                Some(if is_default {
                    DEFAULT_VOICE_ICON
                } else {
                    SET_AS_DEFAULT_ICON
                })
            })
            .sync_create()
            .build();
    }
}
