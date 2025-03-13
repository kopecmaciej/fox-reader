use std::cell::RefCell;

use crate::core::{runtime::runtime, tts::Tts};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use super::{
    dialogs,
    voice_events::{voice_events, VoiceEvent},
    voice_row::VoiceRow,
};

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
        pub play_handler: RefCell<Option<Box<dyn Fn()>>>,
        pub stop_handler: RefCell<Option<Box<dyn Fn()>>>,
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

    fn connect_voice_events(&self) {
        let voice_events = voice_events();

        voice_events.connect_local(
            "voice-downloaded",
            false,
            clone!(
                #[weak(rename_to=this)]
                self,
                #[upgrade_or]
                None,
                move |args| {
                    let voice_key = args[1].get::<String>().unwrap();
                    this.refresh_voice_selector(VoiceEvent::Downloaded(voice_key));
                    None
                }
            ),
        );

        voice_events.connect_local(
            "voice-deleted",
            false,
            clone!(
                #[weak(rename_to=this)]
                self,
                #[upgrade_or]
                None,
                move |args| {
                    let voice_key = args[1].get::<String>().unwrap();
                    this.refresh_voice_selector(VoiceEvent::Deleted(voice_key));
                    None
                }
            ),
        );
    }

    pub fn set_read_handler<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.imp().play_handler.replace(Some(Box::new(handler)));
    }

    pub fn set_stop_handler<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.imp().stop_handler.replace(Some(Box::new(handler)));
    }

    fn refresh_voice_selector(&self, event: VoiceEvent) {
        let voice_selector = &self.imp().voice_selector;
        if let Some(model) = voice_selector.model() {
            if let Some(filter_model) = model.downcast_ref::<gtk::FilterListModel>() {
                if let Some(base_model) = filter_model.model().and_downcast::<gio::ListStore>() {
                    match event {
                        VoiceEvent::Downloaded(voice_key) => {
                            for i in 0..base_model.n_items() {
                                if let Some(voice_row) =
                                    base_model.item(i).and_downcast::<VoiceRow>()
                                {
                                    if voice_row.key() == voice_key {
                                        voice_row.set_downloaded(true);
                                        base_model.items_changed(i, 1, 1);

                                        if let Some(filter) = filter_model
                                            .filter()
                                            .and_downcast::<gtk::CustomFilter>()
                                        {
                                            filter.changed(gtk::FilterChange::Different);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        VoiceEvent::Deleted(voice_key) => {
                            for i in 0..base_model.n_items() {
                                if let Some(voice_row) =
                                    base_model.item(i).and_downcast::<VoiceRow>()
                                {
                                    if voice_row.key() == voice_key {
                                        voice_row.set_downloaded(false);
                                        base_model.items_changed(i, 1, 1);

                                        if let Some(filter) = filter_model
                                            .filter()
                                            .and_downcast::<gtk::CustomFilter>()
                                        {
                                            filter.changed(gtk::FilterChange::Different);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
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
            imp,
            #[weak]
            tts,
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
                button.set_icon_name("media-playback-pause-symbolic");

                imp.stop_button.set_sensitive(true);

                if let Some(handler) = imp.play_handler.borrow().as_ref() {
                    handler();
                } else {
                    dialogs::show_error_dialog("No read handler configured", button);
                }

                button.set_icon_name("media-playback-start-symbolic");
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

    pub fn get_selected_voice(&self) -> Option<String> {
        if let Some(item) = self.imp().voice_selector.selected_item() {
            if let Some(voice_row) = item.downcast_ref::<VoiceRow>() {
                let voice = voice_row.key();
                return Some(voice);
            }
        }
        None
    }

    pub fn populate_voice_selector(&self, all_rows: &[VoiceRow]) {
        let model = gio::ListStore::new::<VoiceRow>();
        model.extend_from_slice(all_rows);

        let filter = gtk::CustomFilter::new(move |obj| {
            if let Some(voice_row) = obj.downcast_ref::<VoiceRow>() {
                return voice_row.downloaded();
            }
            false
        });

        let filtered_model = gtk::FilterListModel::new(Some(model), Some(filter));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                let label = gtk::Label::builder().xalign(0.0).build();
                list_item.set_child(Some(&label));
            }
        });

        factory.connect_bind(|_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                if let Some(v) = list_item.item().and_downcast::<VoiceRow>() {
                    list_item.set_accessible_label(&v.key());
                    if let Some(label) = list_item.child().and_downcast::<gtk::Label>() {
                        let text = format!("{} ({}) - {}", v.name(), v.quality(), v.language());
                        label.set_text(&text);
                    }
                }
            }
        });

        let voice_selector = &self.imp().voice_selector;
        voice_selector.set_factory(Some(&factory));
        voice_selector.set_model(Some(&filtered_model));
    }

    pub fn get_speed(&self) -> f64 {
        self.imp().speed_spin.value()
    }
}
