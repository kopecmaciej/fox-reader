use crate::core::voice_manager::Voice;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib::{self};
use gtk::prelude::*;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VoiceRow)]
    pub struct VoiceRow {
        #[property(get, set)]
        pub name: OnceCell<String>,
        #[property(get, set)]
        pub country: OnceCell<String>,
        #[property(get, set)]
        pub quality: OnceCell<String>,
        #[property(get, set)]
        pub files: OnceCell<Vec<String>>,
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
            .property("country", &voice.language.name_english)
            .property("quality", &voice.quality)
            .property("files", &files)
            .property("downloaded", voice.downloaded)
            .build();
        obj
    }
}
