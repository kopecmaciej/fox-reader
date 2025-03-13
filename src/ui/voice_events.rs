#[derive(Debug, Clone)]
pub enum VoiceEvent {
    Downloaded(String),
    Deleted(String),
    SetDefault(String),
}

use gio::prelude::ObjectExt;
use glib::subclass::Signal;
use gtk::glib;
use std::sync::{Arc, OnceLock};

glib::wrapper! {
    pub struct VoiceEventEmitter(ObjectSubclass<imp::VoiceEventEmitter>);
}

mod imp {
    use std::sync::OnceLock;

    use super::*;
    use glib::subclass::prelude::*;

    #[derive(Default)]
    pub struct VoiceEventEmitter;

    #[glib::object_subclass]
    impl ObjectSubclass for VoiceEventEmitter {
        const NAME: &'static str = "VoiceEventEmitter";
        type Type = super::VoiceEventEmitter;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for VoiceEventEmitter {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("voice-downloaded")
                        .param_types([glib::types::Type::STRING])
                        .build(),
                    Signal::builder("voice-deleted")
                        .param_types([glib::types::Type::STRING])
                        .build(),
                    Signal::builder("voice-set-default")
                        .param_types([glib::types::Type::STRING])
                        .build(),
                ]
            })
        }
    }
}

impl Default for VoiceEventEmitter {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl VoiceEventEmitter {
    pub fn emit_voice_downloaded(&self, voice_key: String) {
        self.emit_by_name::<()>("voice-downloaded", &[&voice_key]);
    }

    pub fn emit_voice_deleted(&self, voice_key: String) {
        self.emit_by_name::<()>("voice-deleted", &[&voice_key]);
    }

    pub fn emit_voice_set_default(&self, voice_key: String) {
        self.emit_by_name::<()>("voice-set-default", &[&voice_key]);
    }
}

// Create a global instance for easy access
pub fn voice_events() -> Arc<VoiceEventEmitter> {
    static INSTANCE: OnceLock<Arc<VoiceEventEmitter>> = OnceLock::new();
    INSTANCE
        .get_or_init(|| Arc::new(VoiceEventEmitter::default()))
        .clone()
}
