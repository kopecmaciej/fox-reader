use crate::{
    core::runtime::spawn_tokio,
    paths::whisper_config::get_whisper_models_names,
    settings::LLMProvider,
    utils::{
        progress_tracker::ProgressTracker,
        whisper_downloader::{download_model, get_downloaded_models, remove_model},
    },
    SETTINGS,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib::{self, clone};

use super::dialogs::show_error_dialog;

mod imp {
    use std::cell::RefCell;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/settings_dialog.ui")]
    pub struct SettingsDialog {
        // Font/highlight settings
        #[template_child]
        pub font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub highlight_color_button: TemplateChild<gtk::ColorDialogButton>,

        // LLM Settings
        #[template_child]
        pub provider_list: TemplateChild<adw::ComboRow>,
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

        // Whisper settings
        #[template_child]
        pub whisper_models: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub whisper_download_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub whisper_download_progress: TemplateChild<gtk::ProgressBar>,
        pub whisper_downloaded_models: RefCell<Vec<String>>,
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

        let font_desc = settings.get_font_description();
        imp.font_button.set_font_desc(&font_desc);

        let rgba = settings.get_highlight_rgba();
        imp.highlight_color_button.set_rgba(&rgba);

        *imp.whisper_downloaded_models.borrow_mut() = get_downloaded_models();

        obj.setup_signals();
        obj.update_ui_from_provider();
        obj.setup_whisper_model_list();
        obj
    }
}

impl SettingsDialog {
    pub fn setup_signals(&self) {
        let imp = self.imp();
        let settings = &SETTINGS;

        imp.font_button.connect_font_desc_notify(|button| {
            if let Some(font_desc) = button.font_desc() {
                settings.set_font(&font_desc);
            }
        });

        imp.highlight_color_button.connect_rgba_notify(|button| {
            let rgba = button.rgba();
            settings.set_highlight_color(&rgba);
        });

        imp.provider_list.connect_selected_notify(clone!(
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

        imp.base_url_entry.connect_changed(|entry| {
            let base_url = entry.text();
            settings.set_base_url(&base_url);
        });

        imp.model_entry.connect_changed(|entry| {
            let model = entry.text();
            settings.set_model(&model);
        });

        imp.api_key_entry.connect_changed(|entry| {
            let api_key = entry.text();
            settings.set_api_key(&api_key);
        });

        imp.temperature_scale.connect_value_changed(|scale| {
            let temperature = scale.value();
            settings.set_temperature(temperature);
        });

        imp.max_tokens_spin.connect_value_changed(|spin| {
            let max_tokens = spin.value() as u32;
            settings.set_max_tokens(max_tokens);
        });

        imp.whisper_models.connect_selected_notify(clone!(
            #[weak(rename_to=this)]
            self,
            move |combo| {
                if let Some(item) = combo.selected_item() {
                    if let Ok(model) = item.downcast::<gtk::StringObject>() {
                        if this.is_model_downloaded(model.string().to_string()) {
                            SETTINGS.set_whisper_model(model.string().as_ref());
                        }
                        this.update_whisper_button_state(model.string().to_string());
                    }
                }
            }
        ));

        imp.whisper_models.connect_realize(clone!(
            #[weak(rename_to=this)]
            self,
            move |_| {
                if let Some(item) = this.imp().whisper_models.selected_item() {
                    if let Ok(model) = item.downcast::<gtk::StringObject>() {
                        this.update_whisper_button_state(model.string().to_string());
                    }
                }
            }
        ));

        imp.whisper_download_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                if let Some(item) = this.imp().whisper_models.selected_item() {
                    if let Ok(model) = item.downcast::<gtk::StringObject>() {
                        let model_name = model.string().to_string();

                        if button.has_css_class("destructive-action") {
                            if let Err(e) = remove_model(&model_name) {
                                show_error_dialog(&format!("Failed to remove file, {}", e), button);
                            }
                            {
                                let mut models = this.imp().whisper_downloaded_models.borrow_mut();
                                if let Some(index) = models.iter().position(|m| m == &model_name) {
                                    models.remove(index);
                                }
                            }
                            this.update_whisper_button_state(model_name);
                            this.refresh_whisper_model_list();
                            return;
                        }

                        button.set_sensitive(false);

                        glib::spawn_future_local(clone!(
                            #[weak]
                            button,
                            async move {
                                let progress_tracker = ProgressTracker::default();
                                let progress_callback = progress_tracker.get_progress_callback();

                                let (on_complete, on_cancel) = progress_tracker
                                    .track_with_progress_bar(&this.imp().whisper_download_progress);

                                let model_name_clone = model_name.clone();
                                let result = spawn_tokio(async move {
                                    download_model(&model_name_clone, Some(progress_callback)).await
                                })
                                .await;

                                button.set_sensitive(true);

                                match result {
                                    Ok(_) => {
                                        on_complete();
                                        SETTINGS.set_whisper_model(&model_name);
                                        {
                                            this.imp()
                                                .whisper_downloaded_models
                                                .borrow_mut()
                                                .push(model_name.clone());
                                        }
                                        this.update_whisper_button_state(model_name);
                                        this.refresh_whisper_model_list();
                                    }
                                    Err(e) => {
                                        on_cancel();
                                        show_error_dialog(&format!("{}", e), &button);
                                    }
                                };
                            }
                        ));
                    }
                }
            }
        ));
    }

    fn setup_whisper_model_list(&self) {
        let imp = self.imp();

        let models = get_whisper_models_names();

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                let label = gtk::Label::builder().xalign(0.0).build();
                list_item.set_child(Some(&label));
            }
        });

        let downloaded_models = imp.whisper_downloaded_models.borrow().to_vec();
        factory.connect_bind(clone!(move |_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                if let Some(item) = list_item.item() {
                    if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                        let model_name = string_obj.string();
                        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();

                        if downloaded_models.contains(&model_name.to_string()) {
                            label.add_css_class("whisper-downloaded-model");
                            label.set_text(&format!("{} ✓", model_name));
                        } else {
                            label.remove_css_class("whisper-downloaded-model");
                            label.set_text(model_name.as_ref());
                        }
                    }
                }
            }
        }));

        imp.whisper_models.set_factory(Some(&factory));

        let whisper_model = gtk::StringList::new(&models);
        imp.whisper_models.set_model(Some(&whisper_model));

        let provider_index = SETTINGS.get_active_model_index();
        imp.whisper_models.set_selected(provider_index as u32);
    }

    pub fn refresh_whisper_model_list(&self) {
        let imp = self.imp();

        let selected_model = if let Some(item) = imp.whisper_models.selected_item() {
            if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                Some(string_obj.string().to_string())
            } else {
                None
            }
        } else {
            None
        };

        self.setup_whisper_model_list();

        if let Some(model) = selected_model {
            let models = get_whisper_models_names();
            for (i, m) in models.iter().enumerate() {
                if m == &model {
                    imp.whisper_models.set_selected(i as u32);
                    break;
                }
            }
        }
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

    fn update_whisper_button_state(&self, model_name: String) {
        let imp = self.imp();
        if self.is_model_downloaded(model_name) {
            imp.whisper_download_button.set_label("Remove");
            imp.whisper_download_button
                .set_css_classes(&["destructive-action"]);
        } else {
            imp.whisper_download_button.set_label("Download");
            imp.whisper_download_button
                .set_css_classes(&["suggested-action"]);
        }
    }

    fn is_model_downloaded(&self, model_name: String) -> bool {
        self.imp()
            .whisper_downloaded_models
            .borrow()
            .contains(&model_name)
    }
}
