use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;
use std::sync::Arc;

use crate::{
    core::{
        runtime::runtime,
        tts::{TTSEvent, Tts},
    },
    utils::text_highlighter::TextHighlighter,
};

use super::{dialogs, voice_row::VoiceRow};

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
        pub tts: Arc<Tts>,
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
        self.init_audio_control_buttons();
        imp.text_highlighter
            .replace(TextHighlighter::new(imp.text_input.buffer(), 100));
    }

    pub fn populate_voice_selector(&self, downloaded_rows: Vec<VoiceRow>) {
        self.imp()
            .audio_controls
            .populate_voice_selector(downloaded_rows);
    }

    pub fn get_voice_selector(&self) -> &TemplateChild<gtk::DropDown> {
        self.imp().audio_controls.get_voice_selector()
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
        let tts = imp.tts.clone();
        let audio = imp.audio_controls.imp();

        audio.stop_button.connect_clicked(clone!(
            #[weak]
            imp,
            #[weak]
            audio,
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
                    imp.text_highlighter.borrow().clear();
                    button.set_sensitive(false);
                    audio.play_button.set_sensitive(true);
                    imp.text_input.set_editable(true);
                }
            }
        ));

        audio.play_button.connect_clicked(clone!(
            #[weak]
            imp,
            #[weak]
            audio,
            #[weak]
            tts,
            move |button| {
                if imp.text_highlighter.borrow().is_buffer_empty() {
                    return;
                }

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

                audio.stop_button.set_sensitive(true);
                let cleaned = imp.text_highlighter.borrow_mut().clean_text();
                imp.text_input.buffer().set_text(&cleaned);

                imp.text_input.set_editable(false);

                let readings_blocks = imp
                    .text_highlighter
                    .borrow()
                    .convert_text_blocks_into_reading_block();

                imp.text_highlighter
                    .borrow()
                    .set_text_blocks(readings_blocks);

                if let Some(item) = audio.voice_selector.selected_item() {
                    if let Some(voice_row) = item.downcast_ref::<VoiceRow>() {
                        let voice = voice_row.key();

                        glib::spawn_future_local(clone!(
                            #[weak]
                            button,
                            #[weak]
                            imp,
                            async move {
                                while let Ok(event) = tts.sender.subscribe().recv().await {
                                    match event {
                                        TTSEvent::Progress { block_id } => {
                                            imp.text_highlighter.borrow().highlight(block_id);
                                        }
                                        TTSEvent::Error(e) => {
                                            dialogs::show_error_dialog(&e, &button);
                                            imp.text_highlighter.borrow().clear();
                                            imp.text_input.set_editable(true);
                                            break;
                                        }
                                        TTSEvent::Next | TTSEvent::Prev => {
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
                            button,
                            #[weak]
                            imp,
                            async move {
                                let readings_blocks =
                                    imp.text_highlighter.borrow().get_reading_blocks().unwrap();

                                if let Err(e) =
                                    imp.tts.read_block_by_voice(&voice, readings_blocks).await
                                {
                                    let err_msg =
                                        format!("Error while reading text by given voice, {}", e);
                                    dialogs::show_error_dialog(&err_msg, &button);
                                }

                                button.set_icon_name("media-playback-start-symbolic");
                                imp.text_highlighter.borrow().clear();
                                imp.text_input.set_editable(true);
                            }
                        ));
                    }
                }
            }
        ));
    }
}
