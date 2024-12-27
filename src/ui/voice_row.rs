use crate::core::runtime::runtime;
use crate::core::speech_dispatcher::SpeechDispatcher;
use crate::core::voice_manager::{Voice, VoiceManager};
use adw::subclass::prelude::*;
use adw::Spinner;
use glib::Properties;
use gtk::glib::{self, clone};
use gtk::{prelude::*, Button};

pub const PLAY_ICON: &str = "media-playback-start-symbolic";
pub const DOWNLOAD_VOICE_ICON: &str = "folder-download-symbolic";
pub const SET_VOICE_DEFAULT_ICON: &str = "starred";
pub const DELETE_VOICE_ICON: &str = "edit-delete";
pub const SET_AS_DEFAULT_ICON: &str = "object-select";

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

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
        pub files: OnceLock<Vec<String>>,
        #[property(get, set)]
        pub downloaded: RefCell<bool>,
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
        let files = voice
            .files
            .clone()
            .into_keys()
            .filter(|f| f.ends_with("json") || f.ends_with("onnx"))
            .collect::<Vec<String>>();

        let obj: Self = glib::Object::builder()
            .property("name", &voice.name)
            .property("key", &voice.key)
            .property("country", &voice.language.name_english)
            .property("language_code", &voice.language.code)
            .property("quality", &voice.quality)
            .property("files", &files)
            .property("downloaded", voice.downloaded)
            .build();
        obj
    }

    pub fn setup_play_button() -> Button {
        Button::builder().icon_name(PLAY_ICON).build()
    }

    pub fn setup_action_buttons() -> (Button, Button, Button) {
        let download_button = Button::builder().icon_name(DOWNLOAD_VOICE_ICON).build();
        let set_default_button = Button::builder().icon_name(SET_VOICE_DEFAULT_ICON).build();
        let delete_button = Button::builder().icon_name(DELETE_VOICE_ICON).build();
        (download_button, set_default_button, delete_button)
    }

    pub fn handle_download_click(&self, download_button: &Button) {
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

                        let files = this.files();
                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(files).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        if let Err(e) = SpeechDispatcher::add_new_voice(
                            &this.language_code(),
                            &this.name(),
                            &this.key(),
                        ) {
                            eprintln!("{}", e);
                        }
                        if let Some(grid) = grid {
                            grid.remove(&spinner);
                            grid.attach(&button, 0, 0, 1, 1);
                        }
                        button.set_sensitive(false);
                    }
                ));
            }
        ));
    }

    pub fn handle_delete_click(&self, remove_button: &Button) {
        remove_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                if let Err(e) = VoiceManager::delete_voice(this.files()) {
                    let err_msg = format!("Failed to remove voice: {}", e);
                    eprintln!("{}", err_msg);
                }
                if let Err(e) =
                    SpeechDispatcher::remove_voice(&this.language_code(), &this.name(), &this.key())
                {
                    eprintln!("{}", e);
                };
                this.set_downloaded(false);
                button.set_sensitive(false);
            }
        ));
    }

    pub fn handle_set_default_click(&self, set_default_button: &Button) {
        set_default_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                if let Err(e) = SpeechDispatcher::set_default_voice(&this.key()) {
                    eprintln!("{}", e);
                }
                button.set_icon_name(SET_AS_DEFAULT_ICON);
            }
        ));
    }
}
