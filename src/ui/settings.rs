use crate::ui::text_reader::TextReader;
use adw::subclass::prelude::*;
use gtk::glib::{self, clone};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/settings.ui")]
    pub struct Settings {
        #[template_child]
        pub font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub highlight_color_button: TemplateChild<gtk::ColorDialogButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Settings {
        const NAME: &'static str = "Settings";
        type Type = super::Settings;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Settings {}
    impl WidgetImpl for Settings {}
    impl AdwDialogImpl for Settings {}
    impl PreferencesDialogImpl for Settings {}
}

glib::wrapper! {
    pub struct Settings(ObjectSubclass<imp::Settings>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Settings {
    fn default() -> Self {
        Settings::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        let obj = glib::Object::builder::<Settings>().build();
        obj
    }
    pub fn setup_signals(&self, text_reader: &TextReader) {
        let imp = self.imp();

        imp.font_button.connect_font_desc_notify(clone!(
            #[weak]
            text_reader,
            move |button| {
                if let Some(font_desc) = button.font_desc() {
                    text_reader.set_text_font(font_desc);
                }
            }
        ));

        imp.highlight_color_button.connect_rgba_notify(clone!(
            #[weak]
            text_reader,
            move |button| {
                let rgba = button.rgba();
                text_reader.set_highlight_color(rgba);
            }
        ));
    }
}
