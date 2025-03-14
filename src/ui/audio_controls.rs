use std::cell::RefCell;

use crate::core::{runtime::runtime, tts::Tts};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use super::{
    dialogs,
    helpers::voice_selector,
    voice_events::{voice_events, VoiceEvent},
    voice_row::VoiceRow,
};

type PlayHandler = RefCell<Option<Box<dyn Fn(u32)>>>;
type StopHandler = RefCell<Option<Box<dyn Fn()>>>;

mod imp {

    use std::sync::Arc;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/audio_controls.ui")]
    pub struct AudioControls {
        #[template_child]
        pub voice_selector: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub prev_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub speed_spin: TemplateChild<gtk::SpinButton>,
        pub tts: Arc<Tts>,
        pub play_handler: PlayHandler,
        pub stop_handler: StopHandler,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudioControls {
        const NAME: &'static str = "AudioControls";
        type Type = super::AudioControls;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AudioControls {}
    impl WidgetImpl for AudioControls {}
    impl BoxImpl for AudioControls {}
}

glib::wrapper! {
    pub struct AudioControls(ObjectSubclass<imp::AudioControls>)
        @extends gtk::Widget, gtk::Box;
}

impl AudioControls {
    pub fn init(&self) {
        self.setup_signals();
        self.connect_voice_events();
    }

    pub fn connect_pdf_audio_events(&self) {
        let voice_events = voice_events();

        voice_events.connect_local(
            "pdf-audio-play",
            false,
            clone!(
                #[weak(rename_to=this)]
                self,
                #[upgrade_or]
                None,
                move |args| {
                    let id = args[1].get::<u32>().unwrap();
                    this.start_audio(id);
                    None
                }
            ),
        );
    }

    fn connect_voice_events(&self) {
        let voice_events = voice_events();

        let voice_selector = &self.imp().voice_selector;
        voice_events.connect_local(
            "voice-downloaded",
            false,
            clone!(
                #[weak]
                voice_selector,
                #[upgrade_or]
                None,
                move |args| {
                    let voice_key = args[1].get::<String>().unwrap();
                    voice_selector::refresh_voice_selector(
                        &voice_selector,
                        VoiceEvent::Downloaded(voice_key),
                    );
                    None
                }
            ),
        );

        voice_events.connect_local(
            "voice-deleted",
            false,
            clone!(
                #[weak]
                voice_selector,
                #[upgrade_or]
                None,
                move |args| {
                    let voice_key = args[1].get::<String>().unwrap();
                    voice_selector::refresh_voice_selector(
                        &voice_selector,
                        VoiceEvent::Deleted(voice_key),
                    );
                    None
                }
            ),
        );
    }

    pub fn start_audio(&self, id: u32) {
        let imp = self.imp();
        let button = &imp.play_button;
        button.set_icon_name("media-playback-pause-symbolic");

        if let Some(handler) = imp.play_handler.borrow().as_ref() {
            handler(id);
        } else {
            dialogs::show_error_dialog("No read handler configured", self);
        }

        button.set_icon_name("media-playback-start-symbolic");
    }

    pub fn set_read_handler<F>(&self, handler: F)
    where
        F: Fn(u32) + 'static,
    {
        self.imp().play_handler.replace(Some(Box::new(handler)));
    }

    pub fn set_stop_handler<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.imp().stop_handler.replace(Some(Box::new(handler)));
    }

    pub fn populate_voice_selector(&self, voices: &[VoiceRow]) {
        voice_selector::populate_voice_selector(&self.imp().voice_selector, voices);
    }

    fn setup_signals(&self) {
        let imp = self.imp();
        let tts = imp.tts.clone();

        imp.stop_button.connect_clicked(clone!(
            #[weak]
            imp,
            #[weak]
            tts,
            move |button| {
                let stoped = runtime().block_on(async {
                    if tts.is_playing() {
                        if (tts.stop(true).await).is_err() {
                            return false;
                        }
                        return true;
                    }
                    false
                });
                if stoped {
                    button.set_sensitive(false);
                    imp.play_button.set_sensitive(true);
                    if let Some(handler) = imp.stop_handler.borrow().as_ref() {
                        handler();
                    }
                }
            }
        ));

        imp.play_button.connect_clicked(clone!(
            #[weak]
            tts,
            #[weak(rename_to=this)]
            self,
            move |button| {
                let was_paused = runtime().block_on(tts.pause_if_playing());
                if was_paused {
                    button.set_icon_name("media-playback-start-symbolic");
                    return;
                }
                let was_resumed = runtime().block_on(tts.resume_if_paused());
                if was_resumed {
                    button.set_icon_name("media-playback-pause-symbolic");
                    return;
                }
                this.start_audio(0);
            }
        ));

        imp.next_button.connect_clicked(clone!(
            #[weak]
            tts,
            move |button| {
                if let Err(e) = runtime().block_on(async {
                    if tts.is_playing() {
                        tts.next().await
                    } else {
                        Ok(())
                    }
                }) {
                    dialogs::show_error_dialog(&e.to_string(), button)
                }
            }
        ));

        imp.prev_button.connect_clicked(clone!(
            #[weak]
            tts,
            move |button| {
                if let Err(e) = runtime().block_on(async {
                    if tts.is_playing() {
                        tts.prev().await
                    } else {
                        Ok(())
                    }
                }) {
                    dialogs::show_error_dialog(&e.to_string(), button)
                }
            }
        ));

        let debounce_duration = std::time::Duration::from_millis(300);
        let timeout_handle = RefCell::new(None::<glib::SourceId>);
        imp.speed_spin.connect_value_changed(clone!(
            #[weak]
            tts,
            move |spin| {
                tts.set_speed(spin.value());
                if !tts.is_playing() {
                    return;
                }

                if let Some(handle) = timeout_handle.borrow_mut().take() {
                    if glib::MainContext::default()
                        .find_source_by_id(&handle)
                        .is_some()
                    {
                        handle.remove();
                    }
                }

                *timeout_handle.borrow_mut() = Some(glib::timeout_add_local(
                    debounce_duration,
                    clone!(
                        #[weak]
                        tts,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move || {
                            let _ = runtime().block_on(tts.repeat_block());
                            glib::ControlFlow::Break
                        }
                    ),
                ));
            }
        ));
    }

    pub fn get_voice_selector(&self) -> &TemplateChild<gtk::DropDown> {
        &self.imp().voice_selector
    }

    pub fn get_selected_voice_key(&self) -> Option<String> {
        if let Some(voice_row) = voice_selector::get_selected_voice(&self.imp().voice_selector) {
            return Some(voice_row.key());
        }
        None
    }

    pub fn get_speed(&self) -> f64 {
        self.imp().speed_spin.value()
    }
}
