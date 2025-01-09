use adw::prelude::AdwDialogExt;
use adw::subclass::prelude::*;
use gio::glib::Object;
use gtk::prelude::*;
use gtk::{
    glib::{self, clone},
    StringList,
};

mod imp {
    use crate::ui::voice_list::VoiceList;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/window.ui")]
    pub struct FoxReaderAppWindow {
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
        #[template_child]
        pub downloaded_container: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FoxReaderAppWindow {
        const NAME: &'static str = "FoxReaderAppWindow";
        type Type = super::FoxReaderAppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
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
        use crate::ui::piper_installer::PiperInstaller;

        let window: Self = Object::builder().property("application", app).build();

        window.imp().voice_list.init();
        window.filter_out_by_language();
        window.setup_search();
        window.setup_stack_switching();

        match PiperInstaller::is_paper_available() {
            Ok(ok) => {
                if !ok {
                    let piper_window = PiperInstaller::new();
                    piper_window.present(Some(&window));
                }
            }
            Err(e) => {
                super::dialogs::show_error_dialog(
                    &format!("Failed to check if piper was already added: {}", e),
                    &window,
                );
            }
        }

        window
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
                            "downloaded_voices" => {
                                voice_list.filter_downloaded_voices();
                                voice_list.unparent();
                                voice_list.set_parent(&this.imp().downloaded_container.get());
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
            #[weak(rename_to=this)]
            self,
            move |entry| {
                let search_text = entry.text();
                this.imp().voice_list.filter_by_search(search_text);
            }
        ));
    }

    fn filter_out_by_language(&self) {
        let language_list = self.imp().voice_list.get_language_list();
        let string_list = StringList::new(&["All"]);
        for c in language_list {
            string_list.append(&c);
        }

        let dropdown = &self.imp().language_filter;
        dropdown.set_model(Some(&string_list));
        dropdown.set_expression(Some(&gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<&gtk::Expression>,
            "string",
        )));

        dropdown.connect_selected_item_notify(clone!(
            #[weak(rename_to=this)]
            self,
            move |f| {
                if let Some(selected_item) = f.selected_item() {
                    if let Some(string_obj) = selected_item.downcast_ref::<gtk::StringObject>() {
                        let language = string_obj.string();
                        this.imp().voice_list.filter_by_language(language);
                    };
                }
            },
        ));
    }
}
