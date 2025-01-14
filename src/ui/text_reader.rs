use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;

use crate::{
    core::{
        runtime::runtime,
        tts::{TTSEvent, Tts},
    },
    utils::text_highlighter::TextHighlighter,
};

use super::{dialogs, voice_row::VoiceRow};

mod imp {

    use crate::utils::text_highlighter::TextHighlighter;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/text_reader.ui")]
    pub struct TextReader {
        #[template_child]
        pub text_input: TemplateChild<gtk::TextView>,
        #[template_child]
        pub voice_selector: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
        pub text_highlighter: RefCell<TextHighlighter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TextReader {
        const NAME: &'static str = "TextReader";
        type Type = super::TextReader;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TextReader {}
    impl WidgetImpl for TextReader {}
    impl BinImpl for TextReader {}
}

glib::wrapper! {
    pub struct TextReader(ObjectSubclass<imp::TextReader>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for TextReader {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl TextReader {
    pub fn init(&self) {
        let imp = self.imp();
        self.read_text_by_selected_voice();
        imp.text_highlighter
            .replace(TextHighlighter::new(imp.text_input.buffer()));
    }

    pub fn populate_voice_selector(&self, downloaded_rows: Vec<VoiceRow>) {
        let model = gio::ListStore::new::<VoiceRow>();
        model.extend_from_slice(&downloaded_rows);
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
                    if let Some(label) = list_item.child().and_downcast::<gtk::Label>() {
                        let text = format!("{} ({}) - {}", v.name(), v.quality(), v.language());
                        label.set_text(&text);
                    }
                }
            }
        });

        let voice_selector = &self.imp().voice_selector;
        voice_selector.set_factory(Some(&factory));
        voice_selector.set_model(Some(&model));
    }

    pub fn get_voice_selector(&self) -> &TemplateChild<gtk::DropDown> {
        &self.imp().voice_selector
    }

    pub fn read_text_by_selected_voice(&self) {
        let imp = self.imp();
        let tts = Tts::new();

        self.imp().play_button.connect_clicked(clone!(
            #[weak]
            imp,
            move |button| {
                if tts.is_running() {
                    runtime().block_on(async {
                        tts.stop().await;
                    });
                    imp.text_highlighter.borrow().clear();
                    button.set_label("Play");
                    return;
                }

                let buffer = imp.text_input.buffer();
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string()
                    .replace("\"", "'");

                if let Some(item) = imp.voice_selector.selected_item() {
                    if let Some(voice_row) = item.downcast_ref::<VoiceRow>() {
                        let voice = voice_row.key();
                        button.set_label("Stop");

                        glib::spawn_future_local(clone!(
                            #[strong]
                            tts,
                            async move {
                                while let Ok(event) = tts.receiver.lock().await.recv().await {
                                    match event {
                                        TTSEvent::Progress {
                                            offset_start,
                                            offset_end,
                                        } => {
                                            println!("{offset_start}, {offset_end}");
                                            imp.text_highlighter
                                                .borrow()
                                                .highlight(offset_start, offset_end);
                                        }
                                        TTSEvent::Terminate | TTSEvent::Done => {
                                            imp.text_highlighter.borrow().clear();
                                            break;
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        ));

                        glib::spawn_future_local(clone!(
                            #[weak]
                            button,
                            #[strong]
                            tts,
                            async move {
                                if let Err(e) = tts.read_text_by_voice(&voice, &text).await {
                                    let err_msg =
                                        format!("Erro while reading text by given voice, {}", e);
                                    dialogs::show_error_dialog(&err_msg, &button);
                                }

                                button.set_label("Play");
                            }
                        ));
                    }
                }
            }
        ));
    }
}
