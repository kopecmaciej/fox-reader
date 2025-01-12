use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};

use crate::core::{runtime::runtime, voice_manager::VoiceManager};

use super::voice_row::VoiceRow;

mod imp {
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
    const HIGHLIGH_TAG: &str = "highlight";

    pub fn init(&self) {
        self.read_text_by_selected_voice();
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

    fn highlight_text(&self, start_offset: i32, end_offset: i32) {
        let buffer = self.imp().text_input.buffer();

        let tag = if let Some(tag) = buffer.tag_table().lookup(Self::HIGHLIGH_TAG) {
            tag
        } else {
            buffer
                .create_tag(Some(Self::HIGHLIGH_TAG), &[("background", &"yellow")])
                .expect("Failed to create tag")
        };

        buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());

        buffer.apply_tag(
            &tag,
            &buffer.iter_at_offset(start_offset),
            &buffer.iter_at_offset(end_offset),
        );
    }

    fn remove_tag(&self, buffer: gtk::TextBuffer) {
        if let Some(tag) = buffer.tag_table().lookup(Self::HIGHLIGH_TAG) {
            buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
        };
    }

    pub fn read_text_by_selected_voice(&self) {
        let imp = self.imp();
        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);
        let kill_tx = std::sync::Arc::new(kill_tx);
        let kill_rx = std::sync::Arc::new(tokio::sync::Mutex::new(kill_rx));

        self.imp().play_button.connect_clicked(clone!(
            #[weak]
            imp,
            #[weak(rename_to=this)]
            self,
            move |button| {
                let kill_tx = kill_tx.clone();

                if button.label() == Some("Stop".into()) {
                    runtime().block_on(async {
                        let _ = kill_tx.send(()).await;
                    });
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
                            #[weak]
                            button,
                            #[weak]
                            this,
                            #[strong]
                            kill_rx,
                            async move {
                                let sentences: Vec<_> =
                                    text.split(['.']).filter(|s| !s.trim().is_empty()).collect();

                                let mut should_continue = true;
                                let mut current_offset = 0;
                                for sentence in sentences {
                                    if !should_continue {
                                        this.remove_tag(buffer);
                                        break;
                                    }
                                    let end_offset = current_offset + sentence.len() as i32;

                                    this.highlight_text(current_offset, end_offset);

                                    if let Ok(mut process) =
                                        runtime().block_on(VoiceManager::play_text_using_piper(
                                            &(sentence.to_owned() + "."),
                                            &voice,
                                        ))
                                    {
                                        let kill_rx = kill_rx.clone();
                                        if let Ok(true) = runtime()
                                            .spawn(async move {
                                                let mut guard = kill_rx.lock().await;
                                                tokio::select! {
                                                    _ = process.wait() => false,
                                                    _ = guard.recv() => {
                                                        let _ = process.terminate().await;
                                                        true
                                                    }
                                                }
                                            })
                                            .await
                                        {
                                            should_continue = false
                                        }
                                    }

                                    current_offset = end_offset;
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
