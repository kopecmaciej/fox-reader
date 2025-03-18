use crate::settings::{LLMProvider, SETTINGS};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib::{self, clone};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/settings.ui")]
    pub struct SettingsDialog {
        // Font/highlight settings
        #[template_child]
        pub font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub highlight_color_button: TemplateChild<gtk::ColorDialogButton>,

        // LLM Settings
        #[template_child]
        pub provider_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub api_key_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub base_url_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub model_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub temperature_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub max_tokens_spin: TemplateChild<gtk::SpinButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsDialog {
        const NAME: &'static str = "Settings";
        type Type = super::SettingsDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SettingsDialog {}
    impl WidgetImpl for SettingsDialog {}
    impl AdwDialogImpl for SettingsDialog {}
    impl PreferencesDialogImpl for SettingsDialog {}
}

glib::wrapper! {
    pub struct SettingsDialog(ObjectSubclass<imp::SettingsDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SettingsDialog {
    fn default() -> Self {
        let obj = glib::Object::builder::<SettingsDialog>().build();

        let imp = obj.imp();
        let settings = &SETTINGS;

        // Set up text reader settings
        let font_desc = settings.get_font_description();
        imp.font_button.set_font_desc(&font_desc);

        let rgba = settings.get_highlight_rgba();
        imp.highlight_color_button.set_rgba(&rgba);

        let model = gtk::StringList::new(
            &LLMProvider::get_all_str()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        imp.provider_combo.set_model(Some(&model));

        let provider_index = settings.get_active_provider_index();
        imp.provider_combo.set_selected(provider_index as u32);

        obj.setup_llm_signals();
        obj
    }
}

impl SettingsDialog {
    pub fn setup_reader_signals(&self) {
        let imp = self.imp();
        let settings = &SETTINGS;

        imp.font_button
            .connect_font_desc_notify(clone!(move |button| {
                if let Some(font_desc) = button.font_desc() {
                    settings.set_font(&font_desc);
                }
            }));

        imp.highlight_color_button
            .connect_rgba_notify(clone!(move |button| {
                let rgba = button.rgba();
                settings.set_highlight_color(&rgba);
            }));
    }

    pub fn setup_llm_signals(&self) {
        let imp = self.imp();
        let settings = &SETTINGS;

        // Initialize UI from the active provider's configuration
        self.update_ui_from_provider();

        imp.provider_combo.connect_selected_notify(clone!(
            #[weak(rename_to=this)]
            self,
            move |combo| {
                if let Some(item) = combo.selected_item() {
                    if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                        let provider = string_obj.string().to_string();
                        settings.set_active_provider(&provider);
                        this.update_ui_from_provider();
                    }
                }
            }
        ));

        imp.base_url_entry.connect_changed(clone!(move |entry| {
            let base_url = entry.text();
            settings.set_base_url(&base_url);
        }));

        imp.model_entry.connect_changed(clone!(move |entry| {
            let model = entry.text();
            settings.set_model(&model);
        }));

        imp.api_key_entry.connect_changed(clone!(move |entry| {
            let api_key = entry.text();
            settings.set_api_key(&api_key);
        }));

        imp.temperature_scale
            .connect_value_changed(clone!(move |scale| {
                let temperature = scale.value();
                settings.set_temperature(temperature);
            }));

        imp.max_tokens_spin
            .connect_value_changed(clone!(move |spin| {
                let max_tokens = spin.value() as u32;
                settings.set_max_tokens(max_tokens);
            }));
    }

    fn update_ui_from_provider(&self) {
        let imp = self.imp();
        let settings = &SETTINGS;

        let provider = settings.get_active_provider();

        imp.base_url_entry.set_text(&settings.get_base_url());
        imp.api_key_entry.set_text(&settings.get_api_key());
        imp.model_entry.set_text(&settings.get_model());
        imp.temperature_scale.set_value(settings.get_temperature());
        imp.max_tokens_spin
            .set_value(settings.get_max_tokens() as f64);
        // Update base URL
        match provider {
            LLMProvider::LMStudio => {
                imp.base_url_entry.set_visible(true);
            }
            LLMProvider::Ollama => {
                imp.base_url_entry.set_visible(true);
            }
            LLMProvider::OpenAI => {
                imp.base_url_entry.set_visible(false);
            }
            LLMProvider::Anthropic => {
                imp.base_url_entry.set_visible(false);
            }
        }
    }
}
