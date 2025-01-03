use crate::core::speech_dispatcher::SpeechDispatcher;
use crate::core::voice_manager::Voice;
use crate::core::{runtime::runtime, voice_manager::VoiceManager};
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::collections::HashSet;
use std::{cell::RefCell, collections::BTreeMap};

use super::dialogs;
use super::voice_row::{VoiceRow, DEFAULT_VOICE_ICON};

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
        pub filter: RefCell<Option<gtk::CustomFilter>>,
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
        if let Err(e) = SpeechDispatcher::initialize_config() {
            let err_msg = format!(
                "Error initializing speech dispatcher config. \nDetails: {}",
                e
            );
            dialogs::show_error_dialog(&err_msg, self);
        }
    }

    fn set_voice_row_model(&self, voice_list: BTreeMap<String, Voice>) {
        let model = gio::ListStore::new::<VoiceRow>();
        for (_, voice) in voice_list {
            let voice_row = VoiceRow::new(voice);

            voice_row.connect_notify_local(
                Some("downloaded"),
                clone!(
                    #[strong(rename_to=this)]
                    self,
                    move |voice_row, _| {
                        if let Ok(mut voices) = this.imp().voice_list.try_borrow_mut() {
                            if let Some(voice) = voices.get_mut(&voice_row.key()) {
                                voice.downloaded = voice_row.downloaded();
                            }
                        }
                    }
                ),
            );

            voice_row.connect_notify_local(
                Some("is_default"),
                clone!(
                    #[strong(rename_to=this)]
                    self,
                    move |voice_row, _| {
                        if let Ok(mut voices) = this.imp().voice_list.try_borrow_mut() {
                            voices.values_mut().for_each(|v| {
                                if v.key == voice_row.key() {
                                    v.is_default = Some(true);
                                } else {
                                    v.is_default = None;
                                }
                            });
                        }
                    }
                ),
            );

            model.append(&voice_row);
        }

        self.set_sorters();

        let filter = gtk::CustomFilter::new(|_| true);
        let filter_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        let sort_model = gtk::SortListModel::builder()
            .model(&filter_model)
            .sorter(&self.imp().column_view.sorter().unwrap())
            .build();

        self.imp()
            .column_view
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));

        self.imp().filter.replace(Some(filter));
    }

    pub fn filter_by_country(&self, search_text: glib::GString) {
        if let Some(filter) = &*self.imp().filter.borrow() {
            filter.set_filter_func(move |obj| {
                if search_text == "All" {
                    return true;
                }
                let voice_row = obj.downcast_ref::<VoiceRow>().unwrap();
                voice_row.country() == search_text
            })
        }
    }

    pub fn filter_downloaded_voices(&self) {
        if let Some(filter) = &*self.imp().filter.borrow() {
            filter.set_filter_func(move |obj| {
                let voice_row = obj.downcast_ref::<VoiceRow>().unwrap();
                voice_row.downloaded()
            });
        }
    }

    pub fn show_all_voices(&self) {
        if let Some(filter) = &*self.imp().filter.borrow() {
            filter.set_filter_func(move |_| true)
        };
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
        list_item.set_child(Some(&VoiceRow::setup_play_button()));
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
        let grid = gtk::Grid::builder().column_spacing(8).vexpand(true).build();

        let (download_button, set_default_button, delete_button) = VoiceRow::setup_action_buttons();

        grid.attach(&download_button, 0, 0, 1, 1);
        grid.attach(&set_default_button, 1, 0, 1, 1);
        grid.attach(&delete_button, 2, 0, 1, 1);
        list_item.set_child(Some(&grid));
    }

    #[template_callback]
    fn bind_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let grid = list_item.child().and_downcast::<gtk::Grid>().unwrap();

        let download_button = grid.child_at(0, 0).and_downcast::<gtk::Button>().unwrap();
        let set_default_button = grid.child_at(1, 0).and_downcast::<gtk::Button>().unwrap();
        let delete_button = grid.child_at(2, 0).and_downcast::<gtk::Button>().unwrap();
        if voice_row.is_default() {
            set_default_button.set_icon_name(DEFAULT_VOICE_ICON);
        }

        voice_row.handle_download_click(&download_button);
        voice_row.handle_set_default_click(&set_default_button);
        voice_row.handle_delete_click(&delete_button);

        download_button.set_sensitive(!voice_row.downloaded());
        set_default_button.set_sensitive(voice_row.downloaded());
        delete_button.set_sensitive(voice_row.downloaded());
    }
}
