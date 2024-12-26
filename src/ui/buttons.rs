use crate::core::runtime::runtime;
use crate::core::voice_manager::VoiceManager;
use gtk::glib::{self, clone, Object};
use gtk::prelude::{ButtonExt, WidgetExt};
use gtk::subclass::prelude::*;
use gtk::Button;

pub const PLAY_ICON: &str = "media-playback-start-symbolic";
pub const DOWNLOAD_VOICE_ICON: &str = "folder-download-symbolic";
pub const SET_VOICE_DEFAULT_ICON: &str = "starred";
pub const REMOVE_VOICE_ICON: &str = "edit-delete";
pub const SET_AS_DEFAULT_ICON: &str = "object-select";

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct RowButton {
        icon: String,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RowButton {
        const NAME: &'static str = "RowButton";
        type Type = super::RowButton;
        type ParentType = gtk::Button;
    }

    impl ObjectImpl for RowButton {}

    impl WidgetImpl for RowButton {}

    impl ButtonImpl for RowButton {}
}

glib::wrapper! {
    pub struct RowButton(ObjectSubclass<imp::RowButton>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

pub struct DownloadButton {
    button: Button,
}

impl DownloadButton {
    pub fn new(files: Vec<String>) -> Self {
        let button = gtk::Button::builder()
            .icon_name(DOWNLOAD_VOICE_ICON)
            .build();

        button.connect_clicked(clone!(
            #[strong]
            files,
            move |button| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    #[strong]
                    files,
                    async move {
                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(files).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        button.set_sensitive(false);
                    }
                ));
            }
        ));

        Self { button }
    }

    pub fn widget(&self) -> &Button {
        &self.button
    }
}

// OLD APPRAOCH
#[derive(Debug, Default)]
pub struct ActionButtons {
    pub play_button: Button,
    pub download_button: Button,
    pub set_default_button: Button,
    pub remove_button: Button,
}

impl ActionButtons {
    pub fn new() -> Self {
        let play_button = Button::builder().icon_name(PLAY_ICON).build();
        let download_button = Button::builder().icon_name(DOWNLOAD_VOICE_ICON).build();
        let set_default_button = Button::builder().icon_name(SET_VOICE_DEFAULT_ICON).build();
        let remove_button = Button::builder().icon_name(REMOVE_VOICE_ICON).build();
        Self {
            play_button,
            download_button,
            set_default_button,
            remove_button,
        }
    }

    pub fn download_button_on_click(&self, files: Vec<String>) {
        self.download_button.connect_clicked(clone!(
            #[strong]
            files,
            move |button| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    #[strong]
                    files,
                    async move {
                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(files).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        button.set_sensitive(false);
                    }
                ));
            }
        ));
    }
}
