use gtk::{
    glib::{self, clone},
    prelude::*,
    AlertDialog, ApplicationWindow, Button,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::hf::{Voice, VoiceManager};

pub const SAVE_VOICE_ICON: &str = "document-save";
pub const SET_VOICE_DEFAULT_ICON: &str = "starred";
pub const REMOVE_VOICE_ICON: &str = "user-trash";

pub fn download_button(window: &ApplicationWindow, voice: Rc<RefCell<Voice>>) -> Button {
    let download_button = Button::builder().icon_name("document-save").build();

    if voice.borrow().downloaded {
        download_button.set_icon_name(SET_VOICE_DEFAULT_ICON);
    } else {
        download_button.set_icon_name(SAVE_VOICE_ICON);
    }

    download_button.connect_clicked(clone!(
        #[weak]
        window,
        move |button| {
            let mut mut_voice = voice.borrow_mut();
            if mut_voice.downloaded {
                button.set_icon_name(SET_VOICE_DEFAULT_ICON);
            } else {
                button.set_icon_name(SAVE_VOICE_ICON);
            }
            if let Err(e) = VoiceManager::download_voice(&mut_voice.files) {
                let err_msh = format!("Failed to download voice: {}", e);
                show_download_alert(&window, &err_msh);
                mut_voice.downloaded = true;
            }
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
                show_download_alert(&window, &err_msg);
                mut_voice.downloaded = true;
            }
        }
    ));

    remove_button
}

pub fn show_download_alert(window: &ApplicationWindow, dialog: &str) {
    let alert_dialog = AlertDialog::builder().modal(true).detail(dialog).build();
    alert_dialog.show(Some(window));
}
