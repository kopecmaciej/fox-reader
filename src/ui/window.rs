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
        pub language_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub voice_list: TemplateChild<VoiceList>,
        #[template_child]
        pub downloaded_filter: TemplateChild<gtk::CheckButton>,
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
        use crate::ui::piper::PiperWindow;

        let window: Self = Object::builder().property("application", app).build();

        window.imp().voice_list.init();
        window.filter_out_by_language();
        window.filter_out_downloaded_voices();
        window.setup_search();

        match PiperWindow::is_paper_available() {
            Ok(ok) => {
                if !ok {
                    let piper_window = PiperWindow::new();
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

    fn filter_out_downloaded_voices(&self) {
        self.imp().downloaded_filter.connect_toggled(clone!(
            #[weak(rename_to=this)]
            self,
            move |btn| {
                let is_checked = btn.is_active();
                let voice_list = &this.imp().voice_list;
                if is_checked {
                    voice_list.filter_downloaded_voices();
                } else {
                    voice_list.show_all_voices();
                }
            },
        ));
    }

    fn filter_out_by_language(&self) {
        let language_list = self.imp().voice_list.get_language_list();
        let string_list = StringList::new(&["All"]);
        for c in language_list {
            string_list.append(&c);
        }

        let dropdown = &self.imp().language_dropdown;
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
