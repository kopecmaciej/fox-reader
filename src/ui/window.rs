use adw::prelude::{AdwDialogExt, MessageDialogExt};
use adw::subclass::prelude::*;
use gio::glib::Object;
use gtk::prelude::*;
use gtk::{
    glib::{self, clone},
    StringList,
};

use crate::{
    core::{runtime::spawn_tokio, speech_dispatcher::SpeechDispatcher, voice_manager::VoiceManager},
    utils::espeak_handler::EspeakHandler,
    SETTINGS,
};

use super::{dialogs, settings_dialog::SettingsDialog};

mod imp {

    use crate::ui::{
        ai_chat::AiChat, pdf_reader::PdfReader, text_reader::TextReader, voice_list::VoiceList,
    };

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/window.ui")]
    pub struct FoxReaderAppWindow {
        #[template_child]
        pub text_reader: TemplateChild<TextReader>,
        #[template_child]
        pub pdf_reader: TemplateChild<PdfReader>,
        #[template_child]
        pub ai_chat: TemplateChild<AiChat>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub language_filter: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub voice_list: TemplateChild<VoiceList>,
        #[template_child]
        pub voice_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub all_voices_container: TemplateChild<gtk::Box>,
        pub settings_dialog: SettingsDialog,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FoxReaderAppWindow {
        const NAME: &'static str = "FoxReaderAppWindow";
        type Type = super::FoxReaderAppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl FoxReaderAppWindow {
        #[template_callback]
        fn on_theme_button_clicked(&self, _button: &gtk::Button) {
            let settings = &SETTINGS;
            let is_dark = !settings.is_dark_color_scheme();

            let style_manager = adw::StyleManager::default();
            style_manager.set_color_scheme(if is_dark {
                adw::ColorScheme::ForceDark
            } else {
                adw::ColorScheme::ForceLight
            });

            settings.set_theme(is_dark);
        }

        #[template_callback]
        fn on_settings_button_clicked(&self, button: &gtk::Button) {
            let settings_dialog = &self.settings_dialog;
            settings_dialog.present(Some(button));
        }
    }

    impl ObjectImpl for FoxReaderAppWindow {}
    impl WidgetImpl for FoxReaderAppWindow {}
    impl WindowImpl for FoxReaderAppWindow {}
    impl ApplicationWindowImpl for FoxReaderAppWindow {}
    impl AdwApplicationWindowImpl for FoxReaderAppWindow {}
}

glib::wrapper! {
    pub struct FoxReaderAppWindow(ObjectSubclass<imp::FoxReaderAppWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl FoxReaderAppWindow {
    pub fn new(app: &adw::Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();

        window.load_window_size();
        window.setup_size_saving();

        if let Err(e) = SpeechDispatcher::init() {
            let err_msg = format!(
                "Error initializing speech dispatcher config. \nDetails: {}",
                e
            );
            dialogs::show_error_dialog(&err_msg, &window);
        }

        // Initialize Kokoros TTS
        let window_weak = window.downgrade();
        glib::spawn_future_local(async move {
            if let Some(window) = window_weak.upgrade() {
                // Use spawn_tokio to run Kokoros initialization in tokio runtime context
                match spawn_tokio(async move {
                    VoiceManager::init_kokoros().await
                }).await {
                    Ok(_) => {
                        println!("Kokoros TTS initialized successfully");
                    }
                    Err(e) => {
                        let err_msg = format!(
                            "Error initializing Kokoros TTS. Falling back to dummy audio. \nDetails: {}",
                            e
                        );
                        dialogs::show_error_dialog(&err_msg, &window);
                    }
                }
            }
        });

        let imp = window.imp();
        let settings = &SETTINGS;

        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(settings.get_color_scheme());

        imp.voice_list.init();
        imp.text_reader.init();
        imp.pdf_reader.init();
        imp.ai_chat.init();
        window.setup_stack_switching();
        window.filter_out_by_language();
        window.update_voice_selector_on_click();
        window.setup_search();
        window.ensure_espeak_avaliable();

        window
    }

    fn ensure_espeak_avaliable(&self) {
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                if !EspeakHandler::is_espeak_installed() {
                    let downloading_dialog = adw::MessageDialog::new(
                        Some(&this),
                        Some("Downloading Required Files"),
                        None,
                    );
                    downloading_dialog.set_width_request(400);
                    downloading_dialog.set_body("Downloading espeak-ng data files, please hold");
                    downloading_dialog.add_response("1", "Cancel");
                    downloading_dialog.set_default_response(Some("1"));
                    downloading_dialog.present();

                    let download_result = spawn_tokio(async move {
                        match EspeakHandler::download_espeak_data(None).await {
                            Ok(res) => Ok::<_, Box<dyn std::error::Error + Send + Sync>>(res),
                            Err(e) => Err(format!("Error generating speech: {}", e).into()),
                        }
                    })
                    .await;

                    match download_result {
                        Ok(_) => {
                            downloading_dialog
                                .set_body("Espeak data files have been successfully downloaded.");

                            downloading_dialog.close();
                        }
                        Err(e) => {
                            downloading_dialog.close();
                            dialogs::show_error_dialog(
                                &format!("Failed to download espeak data: {}", e),
                                &this,
                            );
                        }
                    }
                }

                EspeakHandler::set_espeak_environment();
            }
        ));
    }

    fn update_voice_selector_on_click(&self) {
        let imp = self.imp();
        let voice_rows = imp.voice_list.get_all_rows();
        imp.text_reader
            .imp()
            .audio_controls
            .populate_voice_selector(&voice_rows);

        imp.pdf_reader
            .imp()
            .audio_controls
            .populate_voice_selector(&voice_rows);

        imp.ai_chat.populate_voice_selector(&voice_rows);
    }

    fn setup_stack_switching(&self) {
        let voice_list = self.imp().voice_list.downgrade();
        let stack = &self.imp().voice_stack;

        stack.connect_visible_child_notify(clone!(
            #[weak(rename_to=this)]
            self,
            move |stack| {
                if let Some(page_name) = stack.visible_child_name() {
                    if let Some(voice_list) = voice_list.upgrade() {
                        match page_name.as_str() {
                            "all_voices" => {
                                voice_list.show_all_voices();
                                voice_list.unparent();
                                voice_list.set_parent(&this.imp().all_voices_container.get());
                            }
                            _ => (),
                        }
                    }
                }
            }
        ));
    }

    fn setup_search(&self) {
        self.imp().search_entry.connect_search_changed(clone!(
            #[weak(rename_to=voice_list)]
            self.imp().voice_list,
            move |entry| {
                voice_list.filter_by_search(entry.text());
            }
        ));
    }

    fn filter_out_by_language(&self) {
        let imp = self.imp();
        let model = StringList::new(&["All"]);
        imp.voice_list
            .get_language_list()
            .iter()
            .for_each(|lang| model.append(lang));

        imp.language_filter.set_model(Some(&model));
        imp.language_filter.connect_selected_item_notify(clone!(
            #[weak(rename_to=voice_list)]
            self.imp().voice_list,
            move |dropdown| {
                if let Some(lang) = dropdown.selected_item().and_downcast::<gtk::StringObject>() {
                    voice_list.filter_by_language(lang.string());
                }
            }
        ));
    }

    fn load_window_size(&self) {
        let width = SETTINGS.get_window_width();
        let height = SETTINGS.get_window_height();
        let maximized = SETTINGS.get_window_maximized();

        if let Some(display) = gtk::gdk::Display::default() {
            let monitors = display.monitors();

            if monitors.n_items() > 0 {
                if let Some(monitor) = monitors
                    .item(0)
                    .and_then(|obj| obj.downcast::<gtk::gdk::Monitor>().ok())
                {
                    let geometry = monitor.geometry();
                    let screen_width = geometry.width();
                    let screen_height = geometry.height();

                    let width = width.min(screen_width);
                    let height = height.min(screen_height);

                    self.set_default_size(width, height);
                } else {
                    self.set_default_size(width, height);
                }
            } else {
                self.set_default_size(width, height);
            }
        } else {
            self.set_default_size(width, height);
        }

        if maximized {
            self.maximize();
        }
    }

    fn setup_size_saving(&self) {
        let is_maximized = self.is_maximized();
        let default_size = self.default_size();
        self.connect_close_request(move |_| {
            if !is_maximized {
                let (width, height) = default_size;
                SETTINGS.set_window_width(width);
                SETTINGS.set_window_height(height);
            }
            SETTINGS.set_window_maximized(is_maximized);

            glib::Propagation::Proceed
        });

        let window_clone = self.clone();
        self.connect_maximized_notify(move |_| {
            SETTINGS.set_window_maximized(window_clone.is_maximized());
        });
    }
}
