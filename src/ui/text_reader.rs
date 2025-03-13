use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;

use crate::{core::tts::TTSEvent, utils::text_highlighter::TextHighlighter};

use super::dialogs;

mod imp {

    use crate::{ui::audio_controls::AudioControls, utils::text_highlighter::TextHighlighter};

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/text_reader.ui")]
    pub struct TextReader {
        #[template_child]
        pub text_input: TemplateChild<gtk::TextView>,
        #[template_child]
        pub audio_controls: TemplateChild<AudioControls>,
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

impl TextReader {
    pub fn init(&self, highlight_color: gtk::gdk::RGBA) {
        let imp = self.imp();
        imp.audio_controls.init();
        imp.text_highlighter
            .replace(TextHighlighter::new(imp.text_input.buffer(), 100));
        self.set_highlight_color(highlight_color);
        self.init_audio_control_buttons();
    }

    pub fn set_text_font(&self, font_desc: gtk::pango::FontDescription) {
        let css_provider = gtk::CssProvider::new();
        let font_family = font_desc.family().unwrap_or("Sans".to_string().into());
        let font_size = font_desc.size() / gtk::pango::SCALE;

        let css = format!(
            "textview.text-input {{ font-family: {}; font-size: {}pt; }}",
            font_family, font_size
        );

        css_provider.load_from_string(&css);

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not get default display"),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    pub fn set_highlight_color(&self, rgba: gtk::gdk::RGBA) {
        self.imp()
            .text_highlighter
            .borrow_mut()
            .set_highlight_color(rgba);
    }

    pub fn init_audio_control_buttons(&self) {
        let imp = self.imp();

        imp.audio_controls.set_stop_handler(clone!(
            #[weak]
            imp,
            move || {
                imp.text_highlighter.borrow().clear();
                imp.text_input.set_editable(true);
            }
        ));

        imp.audio_controls.set_read_handler(clone!(
            #[weak(rename_to=this)]
            self,
            #[weak]
            imp,
            move || {
                if imp.text_highlighter.borrow().is_buffer_empty() {
                    return;
                }
                imp.text_input.set_editable(false);
                let cleaned = imp.text_highlighter.borrow_mut().normalize_text();
                imp.text_input.buffer().set_text(&cleaned);
                imp.text_highlighter.borrow().generate_reading_blocks();

                glib::spawn_future_local(clone!(
                    #[weak]
                    this,
                    #[weak]
                    imp,
                    async move {
                        while let Ok(event) =
                            imp.audio_controls.imp().tts.sender.subscribe().recv().await
                        {
                            match event {
                                TTSEvent::Progress { block_id } => {
                                    imp.text_highlighter.borrow().highlight(block_id);
                                }
                                TTSEvent::Error(e) => {
                                    dialogs::show_error_dialog(&e, &this);
                                    imp.text_highlighter.borrow().clear();
                                    imp.text_input.set_editable(true);
                                    break;
                                }
                                TTSEvent::Next | TTSEvent::Prev | TTSEvent::Repeat => {
                                    imp.text_highlighter.borrow().clear();
                                }
                                TTSEvent::Stop => {
                                    break;
                                }
                            }
                        }
                    }
                ));

                glib::spawn_future_local(clone!(
                    #[weak]
                    this,
                    #[weak]
                    imp,
                    async move {
                        let readings_blocks =
                            imp.text_highlighter.borrow().get_reading_blocks().unwrap();

                        if let Some(voice) = imp.audio_controls.get_selected_voice() {
                            if let Err(e) = imp
                                .audio_controls
                                .imp()
                                .tts
                                .read_blocks_by_voice(voice, readings_blocks, 0)
                                .await
                            {
                                let err_msg =
                                    format!("Error while reading text by given voice, {}", e);
                                dialogs::show_error_dialog(&err_msg, &this);
                            }
                            imp.text_highlighter.borrow().clear();
                            imp.text_input.set_editable(true);
                        }
                    }
                ));
            }
        ));
    }
}
