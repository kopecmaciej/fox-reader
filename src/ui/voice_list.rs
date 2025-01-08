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
        pub language_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub actions_column: TemplateChild<gtk::ColumnViewColumn>,
        pub filter: RefCell<Option<gtk::CustomFilter>>,
        pub filter_criteria: RefCell<FilterCriteria>,
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

#[derive(Debug, Clone)]
pub struct FilterCriteria {
    search_text: String,
    selected_language: String,
    show_downloaded_only: bool,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            search_text: String::new(),
            selected_language: "All".to_string(),
            show_downloaded_only: false,
        }
    }
}

impl VoiceList {
    pub fn init(&self) {
        self.imp()
            .filter_criteria
            .replace(FilterCriteria::default());
        if let Err(e) = SpeechDispatcher::init_config() {
            let err_msg = format!(
                "Error initializing speech dispatcher config. \nDetails: {}",
                e
            );
            dialogs::show_error_dialog(&err_msg, self);
        }
        let voice_list = runtime().block_on(VoiceManager::list_all_available_voices());
        if let Ok(voices) = voice_list {
            self.set_voice_row_model(voices);
        }
    }

    fn set_voice_row_model(&self, voice_list: BTreeMap<String, Voice>) {
        let model = gio::ListStore::new::<VoiceRow>();
        for (_, voice) in voice_list {
            let voice_row = VoiceRow::new(voice);

            voice_row.connect_notify_local(
                Some("is-default"),
                clone!(
                    #[weak(rename_to=this)]
                    self,
                    move |voice_row, _| {
                        if !voice_row.is_default() {
                            return;
                        }
                        let model = this.get_list_model();

                        for i in 0..model.n_items() {
                            if let Some(obj) = model.item(i) {
                                if let Ok(row) = obj.downcast::<VoiceRow>() {
                                    if row.key() != voice_row.key() && row.is_default() {
                                        row.set_is_default(false);
                                    }
                                }
                            }
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

    fn get_list_model(&self) -> gio::ListStore {
        self.imp()
            .column_view
            .model()
            .and_downcast::<gtk::NoSelection>()
            .unwrap()
            .model()
            .and_downcast::<gtk::SortListModel>()
            .unwrap()
            .model()
            .and_downcast::<gtk::FilterListModel>()
            .unwrap()
            .model()
            .and_downcast::<gio::ListStore>()
            .unwrap()
    }

    pub fn filter_by_language(&self, language: impl Into<String>) {
        {
            let mut criteria = self.imp().filter_criteria.borrow_mut();
            criteria.selected_language = language.into();
        }
        self.update_filter();
    }

    pub fn filter_by_search(&self, search_text: impl Into<String>) {
        {
            let mut criteria = self.imp().filter_criteria.borrow_mut();
            criteria.search_text = search_text.into();
        }
        self.update_filter();
    }

    pub fn filter_downloaded_voices(&self) {
        {
            let mut criteria = self.imp().filter_criteria.borrow_mut();
            criteria.show_downloaded_only = true;
        }
        self.update_filter();
    }

    pub fn show_all_voices(&self) {
        {
            let mut criteria = self.imp().filter_criteria.borrow_mut();
            criteria.show_downloaded_only = false;
        }
        self.update_filter();
    }

    fn update_filter(&self) {
        if let Some(filter) = &*self.imp().filter.borrow_mut() {
            let criteria = self.imp().filter_criteria.borrow().clone();
            filter.set_filter_func(move |obj| {
                let voice_row = obj.downcast_ref::<VoiceRow>().unwrap();

                let language_matches = criteria.selected_language == "All"
                    || voice_row.language() == criteria.selected_language;

                let search_matches = criteria.search_text.is_empty()
                    || voice_row
                        .name()
                        .to_lowercase()
                        .contains(&criteria.search_text.to_lowercase());

                let download_matches = !criteria.show_downloaded_only || voice_row.downloaded();

                language_matches && search_matches && download_matches
            });
        };
    }

    pub fn get_language_list(&self) -> Vec<String> {
        let model = self.get_list_model();

        let mut list: Vec<String> = (0..model.n_items())
            .filter_map(|i| model.item(i))
            .filter_map(|obj| obj.downcast::<VoiceRow>().ok())
            .map(|voice_row| voice_row.language())
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
            .language_column
            .set_sorter(self.string_sorter("language").as_ref());
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
    fn setup_play_button(_factory: &gtk::SignalListItemFactory, _: &gtk::ListItem) {}

    #[template_callback]
    fn bind_play_button(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let play_button = VoiceRow::setup_play_button();
        voice_row.handle_play_actions(&play_button);
        list_item.set_child(Some(&play_button));
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
    fn bind_language(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_text(&voice_row.language());
    }

    #[template_callback]
    fn setup_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let grid = gtk::Grid::builder().column_spacing(8).vexpand(true).build();

        list_item.set_child(Some(&grid));
    }

    #[template_callback]
    fn bind_actions(_factory: &gtk::SignalListItemFactory, list_item: &gtk::ListItem) {
        let voice_row = list_item.item().and_downcast::<VoiceRow>().unwrap();
        let grid = list_item.child().and_downcast::<gtk::Grid>().unwrap();

        grid.remove_row(0);

        let (download_button, set_default_button, delete_button) = VoiceRow::setup_action_buttons();

        grid.attach(&download_button, 0, 0, 1, 1);
        grid.attach(&set_default_button, 1, 0, 1, 1);
        grid.attach(&delete_button, 2, 0, 1, 1);

        let download_button = grid.child_at(0, 0).and_downcast::<gtk::Button>().unwrap();
        let set_default_button = grid.child_at(1, 0).and_downcast::<gtk::Button>().unwrap();
        let delete_button = grid.child_at(2, 0).and_downcast::<gtk::Button>().unwrap();

        voice_row.handle_download_actions(&download_button);
        download_button.set_sensitive(!voice_row.downloaded());

        voice_row.handle_delete_actions(&delete_button);
        delete_button.set_sensitive(voice_row.downloaded());

        voice_row.handle_set_default_actions(&set_default_button);
        set_default_button.set_sensitive(voice_row.downloaded());
    }
}
