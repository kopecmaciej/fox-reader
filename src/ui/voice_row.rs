use crate::core::voice_manager::Voice;
use adw::subclass::prelude::*;
use gtk::glib::{self};
use std::cell::RefCell;
use std::rc::Rc;

mod voice_object {

    use super::*;
    use gtk::glib;

    #[derive(Debug, Default)]
    pub struct VoiceRow {
        pub voice: Rc<RefCell<Option<Voice>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VoiceRow {
        const NAME: &'static str = "VoiceObject";
        type Type = super::VoiceRow;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for VoiceRow {}
}

glib::wrapper! {
    pub struct VoiceRow(ObjectSubclass<voice_object::VoiceRow>);
}

impl VoiceRow {
    pub fn new(voice: Voice) -> Self {
        let obj: Self = glib::Object::new::<Self>();
        obj.imp().voice.replace(Some(voice));
        obj
    }

    pub fn get_voice(&self) -> Rc<RefCell<Option<Voice>>> {
        Rc::clone(&self.imp().voice)
    }
}
