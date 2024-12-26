use crate::core::voice_manager::Voice;
use crate::core::{runtime::runtime, voice_manager::VoiceManager};
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::collections::HashSet;
use std::{cell::RefCell, collections::BTreeMap};

use super::voice_row::VoiceRow;

mod imp {

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
        pub voice_list: RefCell<BTreeMap<String, Voice>>,
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
    pub fn init(&self) {
        let voice_list = runtime().block_on(VoiceManager::list_all_available_voices());
        if let Ok(voices) = voice_list {
            self.imp().voice_list.replace(voices.clone());
            self.set_voice_row_model(voices);
        }
    }

    fn set_voice_row_model(&self, voice_list: BTreeMap<String, Voice>) {
        let model = gio::ListStore::new::<VoiceRow>();
        for (_, voice) in voice_list {
            let voice_row = VoiceRow::new(voice);
            model.append(&voice_row);
        }
        self.set_sorters();

        let sort_model = gtk::SortListModel::builder()
            .model(&model)
            .sorter(&self.imp().column_view.sorter().unwrap())
            .build();

        self.imp()
            .column_view
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    pub fn filter_by_country(&self, search_text: &str) {
        let countries = self.get_country_list();

        if countries.contains(&search_text.to_string()) {
            let filtered_voices: BTreeMap<String, Voice> = self
                .imp()
                .voice_list
                .borrow()
                .iter()
                .filter(|(_, v)| v.language.name_english == search_text)
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect();

            self.set_voice_row_model(filtered_voices);
        } else {
            self.set_voice_row_model(self.imp().voice_list.borrow().clone());
        }
    }

    pub fn get_country_list(&self) -> Vec<String> {
        let mut list: Vec<String> = self
            .imp()
            .voice_list
            .borrow()
            .iter()
            .map(|(_, v)| v.language.name_english.to_owned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        list.sort();
        list
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
        let button = list_item.child().and_downcast::<gtk::Button>().unwrap();
        button.set_sensitive(voice_row.downloaded());
    }

    #[template_callback]
    fn setup_label(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let label = gtk::Label::builder().xalign(0.0).build();
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn bind_accent(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_text(&voice_row.name());
    }

    #[template_callback]
    fn bind_quality(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_text(&voice_row.quality());
    }

    #[template_callback]
    fn bind_country(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_text(&voice_row.country());
    }

    #[template_callback]
    fn setup_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let box_ = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let download_button = gtk::Button::builder()
            .icon_name("folder-download-symbolic")
            .build();

        let set_default_button = gtk::Button::builder().icon_name("emblem-default").build();

        let delete_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .build();

        box_.append(&download_button);
        box_.append(&set_default_button);
        box_.append(&delete_button);
        list_item.set_child(Some(&box_));
    }

    #[template_callback]
    fn bind_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let box_ = list_item.child().and_downcast::<gtk::Box>().unwrap();

        let mut child = box_.first_child();

        let download_button = child.take().and_downcast::<gtk::Button>().unwrap();
        child = download_button.next_sibling();

        let set_default_button = child.take().and_downcast::<gtk::Button>().unwrap();
        child = download_button.next_sibling();

        let delete_button = child.take().and_downcast::<gtk::Button>().unwrap();

        let files = voice_row.files();
        download_button.connect_clicked(clone!(
            #[strong]
            files,
            move |button| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    button,
                    #[strong]
                    files,
                    async move {
                        let _ = runtime()
                            .spawn(clone!(async move {
                                if let Err(e) = VoiceManager::download_voice(files).await {
                                    eprintln!("Failed to download voice: {}", e);
                                }
                            }))
                            .await;

                        button.set_sensitive(false);
                    }
                ));
            }
        ));

        download_button.set_sensitive(!voice_row.downloaded());
        set_default_button.set_sensitive(voice_row.downloaded());
        delete_button.set_sensitive(voice_row.downloaded());
    }
}
