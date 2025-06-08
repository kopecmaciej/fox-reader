use std::cell::RefCell;

use crate::core::{runtime::runtime, tts::Tts};
use crate::settings::Settings;
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use super::{
    dialogs,
    helpers::voice_selector,
    voice_events::event_emiter,
    voice_row::VoiceRow,
};

type PlayHandler = RefCell<Option<Box<dyn Fn(u32)>>>;
type StopHandler = RefCell<Option<Box<dyn Fn()>>>;

mod imp {

    use std::sync::Arc;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/audio_controls.ui")]
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
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl AudioControls {
        #[template_callback]
        fn on_play_button_clicked(&self, button: &gtk::Button) {
            let obj = self.obj();

            let was_paused = runtime().block_on(obj.imp().tts.pause_if_playing());
            if was_paused {
                button.set_icon_name("media-playback-start-symbolic");
                return;
            }
            let was_resumed = runtime().block_on(obj.imp().tts.resume_if_paused());
            if was_resumed {
                button.set_icon_name("media-playback-pause-symbolic");
                return;
            }
            obj.start_audio(0);
        }

        #[template_callback]
        fn on_stop_button_clicked(&self, button: &gtk::Button) {
            let obj = self.obj();
            let stoped = runtime().block_on(async {
                if (obj.imp().tts.stop(true).await).is_err() {
                    return false;
                }
                true
            });
            if stoped {
                button.set_sensitive(false);
                obj.imp().play_button.set_sensitive(true);
                obj.imp()
                    .play_button
                    .set_icon_name("media-playback-start-symbolic");
                if let Some(handler) = obj.imp().stop_handler.borrow().as_ref() {
                    handler();
                }
            }
        }

        #[template_callback]
        fn on_prev_button_clicked(&self, button: &gtk::Button) {
            let obj = self.obj();
            if let Err(e) = runtime().block_on(async {
                if obj.imp().tts.is_playing() {
                    obj.imp().tts.prev().await
                } else {
                    Ok(())
                }
            }) {
                dialogs::show_error_dialog(&e.to_string(), button)
            }
        }
        #[template_callback]
        fn on_next_button_clicked(&self, button: &gtk::Button) {
            let obj = self.obj();
            if let Err(e) = runtime().block_on(async {
                if obj.imp().tts.is_playing() {
                    obj.imp().tts.next().await
                } else {
                    Ok(())
                }
            }) {
                dialogs::show_error_dialog(&e.to_string(), button)
            }
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
    }

    pub fn set_default_voice_from_settings(&self) {
        let settings = Settings::default();
        let default_voice_key = settings.get_default_voice();
        
        if !default_voice_key.is_empty() {
            println!("Setting default voice from settings: {}", default_voice_key);
            voice_selector::set_selected_voice_by_key(&self.imp().voice_selector, &default_voice_key);
        } else {
            println!("No default voice set in settings");
        }
    }

    pub fn connect_pdf_audio_events(&self) {
        let voice_events = event_emiter();

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

    pub fn start_audio(&self, id: u32) {
        let imp = self.imp();
        let button = &imp.play_button;
        button.set_icon_name("media-playback-pause-symbolic");
        imp.stop_button.set_sensitive(true);

        if let Some(handler) = imp.play_handler.borrow().as_ref() {
            handler(id);
        } else {
            dialogs::show_error_dialog("No read handler configured", self);
        }
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
        
        self.set_default_voice_from_settings();
    }

    fn setup_signals(&self) {
        let imp = self.imp();
        let tts = imp.tts.clone();

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
