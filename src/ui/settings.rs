use crate::config::{LLMProvider, ProviderConfig, SharedConfig};
use crate::ui::text_reader::TextReader;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gdk::RGBA;
use gtk::glib::{self, clone};
use std::cell::RefCell;

mod imp {

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/settings.ui")]
    pub struct Settings {
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

        pub user_config: RefCell<SharedConfig>,
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
    pub fn new(user_config: SharedConfig) -> Self {
        let obj = glib::Object::builder::<Settings>().build();

        let imp = obj.imp();

        // Set up text reader settings
        if let Some(font_str) = &user_config.borrow().font {
            let font_desc = gtk::pango::FontDescription::from_string(font_str);
            imp.font_button.set_font_desc(&font_desc);
        }

        let color_str = &user_config.borrow().highlight_color.clone();
        if let Ok(rgba) = RGBA::parse(color_str) {
            imp.highlight_color_button.set_rgba(&rgba);
        }

        // Setup the combo box for providers
        let model = gtk::StringList::new(&[]);
        for provider in LLMProvider::get_all() {
            model.append(&provider.to_string());
        }
        imp.provider_combo.set_model(Some(&model));

        obj.imp().user_config.replace(user_config);
        obj.setup_llm_signals();

        obj
    }

    pub fn setup_signals(&self, text_reader: &TextReader) {
        let imp = self.imp();

        imp.font_button.connect_font_desc_notify(clone!(
            #[weak(rename_to=this)]
            self,
            #[weak]
            text_reader,
            move |button| {
                if let Some(font_desc) = button.font_desc() {
                    text_reader.set_text_font(font_desc.clone());
                    this.get_shared_config().borrow_mut().set_font(&font_desc);
                }
            }
        ));

        imp.highlight_color_button.connect_rgba_notify(clone!(
            #[weak(rename_to=this)]
            self,
            #[weak]
            text_reader,
            move |button| {
                let rgba = button.rgba();
                text_reader.set_highlight_color(rgba);
                this.get_shared_config()
                    .borrow_mut()
                    .set_highlight_color(&rgba);
            }
        ));
    }

    pub fn setup_llm_signals(&self) {
        let imp = self.imp();

        // Initialize UI from the active provider's configuration
        let user_config = self.get_shared_config();
        let active_provider = &user_config.borrow().llm_config.active_provider;
        let provider_config = user_config.borrow().get_provider_config(active_provider);
        self.update_ui_from_provider_config(active_provider, &provider_config);

        imp.provider_combo.connect_selected_notify(clone!(
            #[weak(rename_to=this)]
            self,
            move |combo| {
                if let Some(item) = combo.selected_item() {
                    if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                        let provider_str = string_obj.string().to_string();

                        if let Some(provider) = LLMProvider::from_str(&provider_str) {
                            let config = this.get_shared_config();
                            config
                                .borrow_mut()
                                .set_active_llm_provider(provider.clone());

                            let provider_config =
                                config.borrow_mut().get_provider_config(&provider);
                            drop(config);
                            this.update_ui_from_provider_config(&provider, &provider_config);
                        }
                    }
                }
            }
        ));

        imp.api_key_entry.connect_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |entry| {
                let api_key = entry.text().to_string();
                if !api_key.is_empty() {
                    this.get_shared_config()
                        .borrow_mut()
                        .set_llm_api_key(api_key);
                }
            }
        ));

        imp.base_url_entry.connect_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |entry| {
                let base_url = entry.text().to_string();
                if !base_url.is_empty() {
                    this.get_shared_config()
                        .borrow_mut()
                        .set_llm_base_url(base_url);
                }
            }
        ));

        imp.model_entry.connect_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |entry| {
                let model = entry.text().to_string();
                if !model.is_empty() {
                    this.get_shared_config().borrow_mut().set_llm_model(model);
                }
            }
        ));

        imp.temperature_scale.connect_value_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |scale| {
                let temperature = scale.value() as f32;
                this.get_shared_config()
                    .borrow_mut()
                    .set_llm_temperature(temperature);
            }
        ));

        imp.max_tokens_spin.connect_value_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |spin| {
                let max_tokens = spin.value() as u32;
                this.get_shared_config()
                    .borrow_mut()
                    .set_llm_max_tokens(max_tokens);
            }
        ));
    }

    fn get_shared_config(&self) -> SharedConfig {
        self.imp().user_config.borrow().clone()
    }

    fn update_ui_from_provider_config(&self, provider: &LLMProvider, config: &ProviderConfig) {
        let imp = self.imp();

        let provider_index = match provider {
            LLMProvider::LMStudio => 0,
            LLMProvider::Ollama => 1,
            LLMProvider::OpenAI => 2,
            LLMProvider::Anthropic => 3,
        };
        imp.provider_combo.set_selected(provider_index);

        if let Some(api_key) = &config.api_key {
            imp.api_key_entry.set_text(api_key);
        } else {
            imp.api_key_entry.set_text("");
        }

        imp.base_url_entry.set_text(&config.base_url);

        if let Some(model) = &config.model {
            imp.model_entry.set_text(model);
        } else {
            imp.model_entry.set_text("");
        }

        if let Some(temp) = config.temperature {
            imp.temperature_scale.set_value(temp.into());
        } else {
            imp.temperature_scale.set_value(0.7);
        }

        if let Some(max_tokens) = config.max_tokens {
            imp.max_tokens_spin.set_value(max_tokens.into());
        } else {
            imp.max_tokens_spin.set_value(1024.0);
        }

        self.update_ui_for_provider(provider);
    }

    fn update_ui_for_provider(&self, provider: &LLMProvider) {
        let imp = self.imp();

        match provider {
            LLMProvider::LMStudio => {
                imp.base_url_entry.set_visible(true);
            }
            LLMProvider::OpenAI => {
                imp.base_url_entry.set_visible(false);
            }
            LLMProvider::Anthropic => {
                imp.base_url_entry.set_visible(false);
            }
            LLMProvider::Ollama => {
                imp.base_url_entry.set_visible(true);
            }
        }
    }
}
