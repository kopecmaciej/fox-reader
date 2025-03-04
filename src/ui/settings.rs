use crate::config::UserConfig;
use crate::ui::text_reader::TextReader;
use adw::subclass::prelude::*;
use gtk::gdk::RGBA;
use gtk::glib::{self, clone};
use std::{cell::RefCell, rc::Rc};

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
        pub user_config: RefCell<Rc<RefCell<UserConfig>>>,
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

impl Settings {
    pub fn new(user_config: Rc<RefCell<UserConfig>>) -> Self {
        let obj = glib::Object::builder::<Settings>().build();

        let imp = obj.imp();

        if let Some(font_str) = &user_config.borrow().font {
            let font_desc = gtk::pango::FontDescription::from_string(font_str);
            imp.font_button.set_font_desc(&font_desc);
        }

        let color_str = &user_config.borrow().highlight_color.clone();
        if let Ok(rgba) = RGBA::parse(color_str) {
            imp.highlight_color_button.set_rgba(&rgba);
        }

        obj.imp().user_config.replace(user_config);

        obj
    }

    pub fn setup_signals(&self, text_reader: &TextReader) {
        let imp = self.imp();

        imp.font_button.connect_font_desc_notify(clone!(
            #[weak]
            imp,
            #[weak]
            text_reader,
            move |button| {
                if let Some(font_desc) = button.font_desc() {
                    text_reader.set_text_font(font_desc.clone());
                    imp.user_config.borrow().borrow_mut().set_font(&font_desc);
                }
            }
        ));

        imp.highlight_color_button.connect_rgba_notify(clone!(
            #[weak]
            imp,
            #[weak]
            text_reader,
            move |button| {
                let rgba = button.rgba();
                text_reader.set_highlight_color(rgba);
                imp.user_config
                    .borrow()
                    .borrow_mut()
                    .set_highlight_color(&rgba);
            }
        ));
    }
}
