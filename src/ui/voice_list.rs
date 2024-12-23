use crate::core::{runtime::spawn_tokio_future, voice_manager::VoiceManager};
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::voice_row::VoiceRow;

mod imp {

    use crate::core::voice_manager::Voice;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/voice_list.ui")]
    pub struct VoiceList {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub play_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub country_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub actions_column: TemplateChild<gtk::ColumnViewColumn>,
        pub voice_list: Vec<Rc<RefCell<Voice>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VoiceList {
        const NAME: &'static str = "VoiceList";
        type Type = super::VoiceList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VoiceList {}
    impl WidgetImpl for VoiceList {}
    impl BinImpl for VoiceList {}
}

glib::wrapper! {
    pub struct VoiceList(ObjectSubclass<imp::VoiceList>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for VoiceList {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl VoiceList {
    pub fn initialize(&self) {
        let model = gio::ListStore::new::<VoiceRow>();

        spawn_tokio_future(clone!(
            #[strong]
            model,
            async move {
                if let Ok(voices) = VoiceManager::list_all_available_voices().await {
                    for (_, voice) in voices {
                        let voice_obj = VoiceRow::new(voice);
                        model.append(&voice_obj);
                    }
                }
            }
        ));

        self.set_sorters();

        let sort_model = gtk::SortListModel::builder()
            .model(&model)
            .sorter(&self.imp().column_view.sorter().unwrap())
            .build();

        self.imp()
            .column_view
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    pub fn get_country_list(&self) -> Vec<String> {
        self.imp()
            .voice_list
            .into_iter()
            .map(|v| v.borrow().language.name_english)
            .collect()
    }

    fn set_sorters(&self) {
        self.imp()
            .name_column
            .set_sorter(self.string_sorter("name").as_ref());

        self.imp()
            .country_column
            .set_sorter(self.string_sorter("country").as_ref());
    }

    fn string_sorter(&self, prop_name: &str) -> Option<gtk::StringSorter> {
        Some(gtk::StringSorter::new(Some(gtk::PropertyExpression::new(
            VoiceRow::static_type(),
            None::<&gtk::Expression>,
            prop_name,
        ))))
    }
}

#[gtk::template_callbacks]
impl VoiceList {
    #[template_callback]
    fn setup_play_button(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let button = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .action_name("voice.play")
            .build();
        list_item.set_child(Some(&button));
    }

    #[template_callback]
    fn bind_play_button(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        if let Some(voice) = voice_row.get_voice().borrow().as_ref() {
            let button = list_item.child().and_downcast::<gtk::Button>().unwrap();
            button.set_sensitive(voice.downloaded);
        }
    }

    #[template_callback]
    fn setup_label(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let label = gtk::Label::builder().xalign(0.0).build();
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn bind_accent(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_obj = list_item.item().and_downcast::<VoiceRow>().unwrap();
        if let Some(voice) = voice_obj.get_voice().borrow().as_ref() {
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            label.set_text(&voice.name);
        }
    }

    #[template_callback]
    fn bind_quality(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        if let Some(voice) = voice_row.get_voice().borrow().as_ref() {
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            label.set_text(&voice.quality);
        }
    }

    #[template_callback]
    fn bind_country(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        if let Some(voice) = voice_row.get_voice().borrow().as_ref() {
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            label.set_text(&voice.language.name_english);
        }
    }

    #[template_callback]
    fn setup_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let box_ = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let download_button = gtk::Button::builder()
            .icon_name("folder-download-symbolic")
            .action_name("voice.download")
            .build();

        let set_default_button = gtk::Button::builder()
            .icon_name("emblem-default")
            .action_name("voice.set_default")
            .build();

        let delete_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .action_name("voice.delete")
            .build();

        box_.append(&download_button);
        box_.append(&set_default_button);
        box_.append(&delete_button);
        list_item.set_child(Some(&box_));
    }

    #[template_callback]
    fn bind_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        if let Some(voice) = voice_row.get_voice().borrow().as_ref() {
            let box_ = list_item.child().and_downcast::<gtk::Box>().unwrap();

            let mut child = box_.first_child();

            let download_button = child
                .take()
                .and_then(|c| c.downcast::<gtk::Button>().ok())
                .expect("Failed to get download button");

            child = download_button.next_sibling();

            let set_default_button = child
                .take()
                .and_then(|c| c.downcast::<gtk::Button>().ok())
                .expect("Failed to get set default button");

            child = set_default_button.next_sibling();

            let delete_button = child
                .take()
                .and_then(|c| c.downcast::<gtk::Button>().ok())
                .expect("Failed to get delete button");

            let downloaded = voice.downloaded;
            download_button.set_sensitive(!downloaded);
            set_default_button.set_sensitive(downloaded);
            delete_button.set_sensitive(downloaded);
        }
    }
}
