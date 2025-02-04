use crate::ui::text_reader::TextReader;
use crate::{config::UserConfig, core::file_handler::FileHandler};
use adw::subclass::prelude::*;
use gtk::gdk::RGBA;
use gtk::glib::{self, clone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserSettings {
    pub font: Option<String>,
    pub highlight_color: Option<String>,
}

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/settings.ui")]
    pub struct Settings {
        #[template_child]
        pub font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub highlight_color_button: TemplateChild<gtk::ColorDialogButton>,
        pub user_settings: Rc<RefCell<UserSettings>>,
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

        if let Ok(settings) = FileHandler::load_settings_from_json(&UserConfig::get_config_path()) {
            let imp = obj.imp();

            if let Some(font_str) = settings.font {
                let font_desc = gtk::pango::FontDescription::from_string(&font_str);
                imp.font_button.set_font_desc(&font_desc);
            }

            if let Some(color_str) = settings.highlight_color {
                if let Ok(rgba) = RGBA::parse(&color_str) {
                    imp.highlight_color_button.set_rgba(&rgba);
                }
            }
        }

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

                    let mut user_settings = imp.user_settings.borrow_mut();
                    user_settings.font = Some(font_desc.to_string());
                    Settings::save_settings(&user_settings);
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

                let mut user_settings = imp.user_settings.borrow_mut();
                user_settings.highlight_color = Some(rgba.to_string());
                Settings::save_settings(&user_settings);
            }
        ));
    }

    fn save_settings(settings: &UserSettings) {
        if let Err(e) = FileHandler::update_json(&UserConfig::get_config_path(), settings) {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}
