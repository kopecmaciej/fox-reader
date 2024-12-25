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
        pub country_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub voice_list: TemplateChild<VoiceList>,
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
        let window: Self = Object::builder().property("application", app).build();

        window.imp().voice_list.initialize();
        window.populate_country_dropdown();

        window
    }

    fn populate_country_dropdown(&self) {
        let country_list = self.imp().voice_list.get_country_list();
        let string_list = StringList::new(&[]);
        string_list.append("All");
        for c in country_list {
            string_list.append(&c);
        }
        self.imp().country_dropdown.set_model(Some(&string_list));
        self.imp()
            .country_dropdown
            .connect_selected_item_notify(clone!(
                #[weak(rename_to=this)]
                self,
                move |f| {
                    if let Some(selected_item) = f.selected_item() {
                        if let Some(string_obj) = selected_item.downcast_ref::<gtk::StringObject>()
                        {
                            let country = string_obj.string();
                            this.imp().voice_list.filter_by_country(&country);
                        };
                    }
                },
            ));
    }
}
