use gtk::{
    glib::{self, clone},
    prelude::*,
    AlertDialog, ApplicationWindow, Button,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::dispatcher::SpeechDispatcher;
use crate::runtime::runtime;
use crate::voice_manager::{Voice, VoiceManager};

pub const SAVE_VOICE_ICON: &str = "document-save";
pub const SET_VOICE_DEFAULT_ICON: &str = "starred";
pub const REMOVE_VOICE_ICON: &str = "user-trash";
pub const SET_AS_DEFAULT_ICON: &str = "object-select";

pub fn download_button(window: &ApplicationWindow, voice: Rc<RefCell<Voice>>) -> Button {
    let download_button = Button::builder().icon_name(SAVE_VOICE_ICON).build();

    if voice.borrow().downloaded {
        download_button.set_icon_name(SET_VOICE_DEFAULT_ICON);
    } else {
        download_button.set_icon_name(SAVE_VOICE_ICON);
    }

    download_button.connect_clicked(clone!(
        #[weak]
        window,
        #[strong]
        voice,
        move |button| {
            glib::spawn_future_local(clone!(
                #[weak]
                button,
                #[weak]
                window,
                #[strong]
                voice,
                async move {
                    if !voice.borrow().downloaded {
                        button.set_sensitive(false);
                        let files = voice.borrow().files.clone();

                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(&files).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        let mut voice_ref = voice.borrow_mut();
                        voice_ref.downloaded = true;
                        let language_code = voice_ref.language.code.clone();
                        let name = voice_ref.name.clone();
                        let key = voice_ref.key.clone();

                        if let Err(e) = SpeechDispatcher::add_new_voice(&language_code, &name, &key)
                        {
                            eprintln!("{}", e);
                            show_alert(&window, "Error while setting default voice");
                        }
                        button.set_icon_name(SET_VOICE_DEFAULT_ICON);
                        button.set_sensitive(true);
                    } else {
                        let key = voice.borrow().key.clone();
                        if let Err(e) = SpeechDispatcher::set_default_voice(&key) {
                            eprintln!("{}", e);
                            show_alert(&window, "Error while setting default voice");
                        }
                        button.set_icon_name(SET_AS_DEFAULT_ICON);
                    }
                }
            ));
        }
    ));

    download_button
}

pub fn remove_button(window: &ApplicationWindow, voice: Rc<RefCell<Voice>>) -> Button {
    let remove_button = Button::builder().icon_name(REMOVE_VOICE_ICON).build();
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
                show_alert(&window, &err_msg);
            }
            if let Err(e) = SpeechDispatcher::remove_voice(
                &mut_voice.language.code,
                &mut_voice.name,
                &mut_voice.key,
            ) {
                eprintln!("{}", e);
                show_alert(&window, "Error while setting default voice");
            };
            mut_voice.downloaded = false;
        }
    ));

    remove_button
}

pub fn show_alert(window: &ApplicationWindow, dialog: &str) {
    let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
    alert_dialog.show(Some(window));
}
