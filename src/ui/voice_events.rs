use gio::prelude::ObjectExt;
use glib::subclass::Signal;
use gtk::glib;
use std::sync::{Arc, OnceLock};

mod imp {
    use std::sync::OnceLock;

    use super::*;
    use glib::subclass::prelude::*;

    #[derive(Default)]
    pub struct EventEmmiter;

    #[glib::object_subclass]
    impl ObjectSubclass for EventEmmiter {
        const NAME: &'static str = "EventEmitter";
        type Type = super::EventEmitter;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for EventEmmiter {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("voice-set-default")
                        .param_types([glib::types::Type::STRING])
                        .build(),
                    Signal::builder("pdf-audio-play")
                        .param_types([glib::types::Type::U32])
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct EventEmitter(ObjectSubclass<imp::EventEmmiter>);
}

impl Default for EventEmitter {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl EventEmitter {
    pub fn emit_voice_set_default(&self, voice_key: String) {
        self.emit_by_name::<()>("voice-set-default", &[&voice_key]);
    }

    pub fn emit_audio_play(&self, id: u32) {
        self.emit_by_name::<()>("pdf-audio-play", &[&id]);
    }
}

pub fn event_emiter() -> Arc<EventEmitter> {
    static INSTANCE: OnceLock<Arc<EventEmitter>> = OnceLock::new();
    INSTANCE
        .get_or_init(|| Arc::new(EventEmitter::default()))
        .clone()
}
