use crate::core::voice_manager::Voice;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib::{self};
use gtk::prelude::*;
use gtk::Button;
use std::cell::RefCell;
use std::rc::Rc;

mod row {

    use std::cell::OnceCell;

    use super::*;
    use gtk::glib;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VoiceRow)]
    pub struct VoiceRow {
        #[property(get, set)]
        pub name: OnceCell<String>,
        #[property(get, set)]
        pub country: OnceCell<String>,
        pub voice: Rc<RefCell<Option<Voice>>>,
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
    pub struct VoiceRow(ObjectSubclass<row::VoiceRow>);
}

pub const SAVE_VOICE_ICON: &str = "document-save";
pub const SET_VOICE_DEFAULT_ICON: &str = "starred";
pub const REMOVE_VOICE_ICON: &str = "edit-delete";
pub const SET_AS_DEFAULT_ICON: &str = "object-select";

impl VoiceRow {
    pub fn new(voice: Voice) -> Self {
        let obj: Self = glib::Object::builder()
            .property("name", &voice.name)
            .property("country", &voice.language.name_english)
            .build();
        obj.imp().voice.replace(Some(voice));
        obj
    }

    pub fn get_voice(&self) -> Rc<RefCell<Option<Voice>>> {
        Rc::clone(&self.imp().voice)
    }

    pub fn download_button(&self) -> Button {
        let download_button = Button::builder().icon_name(SAVE_VOICE_ICON).build();

        download_button
    }
}
