use std::cell::RefCell;
use std::sync::Arc;

use crate::core::runtime::{runtime, spawn_tokio};
use crate::core::speech_dispatcher::SpeechDispatcher;
use crate::core::voice_manager::{Voice, VoiceManager};
use crate::settings::Settings;
use crate::ui::dialogs;
use crate::utils::audio_player::AudioPlayer;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib::{self, clone};
use gtk::{prelude::*, Button};

pub const PLAY_ICON: &str = "media-playback-start-symbolic";
pub const STOP_ICON: &str = "media-playback-stop-symbolic";
pub const SET_AS_DEFAULT_ICON: &str = "starred";
pub const DEFAULT_VOICE_ICON: &str = "object-select";

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VoiceRow)]
    pub struct VoiceRow {
        #[property(get, set)]
        pub name: OnceCell<String>,
        #[property(get, set)]
        pub key: OnceCell<String>,
        #[property(get, set)]
        pub language: OnceCell<String>,
        #[property(get, set)]
        pub language_code: OnceCell<String>,
        #[property(get, set)]
        pub quality: OnceCell<String>,
        #[property(get, set)]
        pub traits: OnceCell<String>,
        #[property(get, set)]
        pub is_default: RefCell<bool>,
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
        let mut language = voice.language.name_english.clone();

        if !voice.name.contains("🇺🇸")
            && !voice.name.contains("🇬🇧")
            && !voice.name.contains("🇯🇵")
            && !voice.name.contains("🇨🇳")
            && !voice.name.contains("🇪🇸")
            && !voice.name.contains("🇫🇷")
            && !voice.name.contains("🇮🇳")
            && !voice.name.contains("🇮🇹")
            && !voice.name.contains("🇧🇷")
        {
            if language == "English" {
                language = format!("{} ({})", language, voice.language.region);
            } else if !voice.language.region.is_empty()
                && voice.language.region != voice.language.name_english
            {
                language = format!("{} ({})", language, voice.language.region);
            }
        } else {
            language = voice.language.name_english.clone();
        }

        let obj: Self = glib::Object::builder()
            .property("name", &voice.name)
            .property("key", &voice.key)
            .property("language", language)
            .property("language_code", &voice.language.code)
            .property("quality", &voice.quality)
            .property("traits", &voice.traits)
            .property("is_default", voice.is_default.unwrap_or(false))
            .build();
        obj
    }

    pub fn setup_action_buttons() -> (Button, Button) {
        let play_button = Button::builder()
            .icon_name(PLAY_ICON)
            .tooltip_text("Play Sample")
            .css_name("play-button")
            .build();
        let set_default_button = Button::builder()
            .icon_name(SET_AS_DEFAULT_ICON)
            .tooltip_text("Set as default")
            .css_name("default-button")
            .build();

        (play_button, set_default_button)
    }

    pub fn handle_play_actions(&self, play_button: &Button) {
        let audio_player = Arc::new(AudioPlayer::default());
        play_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                let is_playing = button.icon_name().is_some_and(|icon| icon == STOP_ICON);
                if is_playing {
                    runtime().block_on(async { audio_player.stop() });
                    button.set_icon_name(PLAY_ICON);
                    return;
                }
                button.set_icon_name(STOP_ICON);
                let audio_player = audio_player.clone();
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    async move {
                        let voice_style = this.key();
                        let sample_text = "Hello, this is a sample of this voice.";

                        println!("Playing sample for voice: {}", this.key());

                        match spawn_tokio(async move {
                            VoiceManager::generate_kokoros_speech(sample_text, &voice_style, 1.0)
                                .await
                        })
                        .await
                        {
                            Ok(audio_buffer) => {
                                if let Err(e) =
                                    spawn_tokio(
                                        async move { audio_player.play_audio(audio_buffer) },
                                    )
                                    .await
                                {
                                    dialogs::show_error_dialog(&e.to_string(), &button);
                                }
                            }
                            Err(e) => dialogs::show_error_dialog(&e.to_string(), &button),
                        }

                        button.set_icon_name(PLAY_ICON);
                    }
                ));
            }
        ));
    }

    pub fn handle_set_default_actions(&self, set_default_button: &Button) {
        set_default_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                if let Err(e) = SpeechDispatcher::set_default_voice(&this.key()) {
                    let err_msg = format!("Failed to set voice as default: {}", e);
                    dialogs::show_error_dialog(&err_msg, button);
                    return;
                }

                let settings = Settings::default();
                settings.set_default_voice(&this.key());

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
