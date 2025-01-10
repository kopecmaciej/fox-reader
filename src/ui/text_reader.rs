use adw::subclass::prelude::*;
use gtk::{
    glib::{self},
    prelude::*,
    StringList,
};

use super::voice_row::VoiceRow;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/text_reader.ui")]
    pub struct TextReader {
        #[template_child]
        pub text_input: TemplateChild<gtk::TextView>,
        #[template_child]
        pub voice_selector: TemplateChild<gtk::DropDown>,
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
    pub fn populate_voice_selector(&self, downloaded_rows: Vec<VoiceRow>) {
        let string_list = StringList::new(&[]);
        for v in downloaded_rows {
            let item = format!("{} ({}) - {}", v.name(), v.quality(), v.language());
            string_list.append(&item);
        }

        let voice_selector = &self.imp().voice_selector;
        voice_selector.set_model(Some(&string_list));
        voice_selector.set_expression(Some(&gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<&gtk::Expression>,
            "string",
        )));
    }

    pub fn get_voice_selector(&self) -> &TemplateChild<gtk::DropDown> {
        &self.imp().voice_selector
    }
}
