use super::{voice_events::VoiceEvent, voice_row::VoiceRow};
use gtk::{gio, prelude::*};

pub mod voice_selector {
    use super::*;

    pub fn get_selected_voice(voice_selector: &gtk::DropDown) -> Option<VoiceRow> {
        if let Some(item) = voice_selector.selected_item() {
            if let Some(voice_row) = item.downcast_ref::<VoiceRow>() {
                return Some(voice_row.clone());
            }
        }
        None
    }

    pub fn populate_voice_selector(voice_selector: &gtk::DropDown, all_rows: &[VoiceRow]) {
        let model = gio::ListStore::new::<VoiceRow>();
        model.extend_from_slice(all_rows);

        let filter = gtk::CustomFilter::new(move |obj| {
            if let Some(voice_row) = obj.downcast_ref::<VoiceRow>() {
                return voice_row.downloaded();
            }
            false
        });

        let filtered_model = gtk::FilterListModel::new(Some(model), Some(filter));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                let label = gtk::Label::builder().xalign(0.0).build();
                list_item.set_child(Some(&label));
            }
        });

        factory.connect_bind(|_, list_item| {
            if let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() {
                if let Some(v) = list_item.item().and_downcast::<VoiceRow>() {
                    list_item.set_accessible_label(&v.key());
                    if let Some(label) = list_item.child().and_downcast::<gtk::Label>() {
                        let text = format!("{} ({}) - {}", v.name(), v.quality(), v.language());
                        label.set_text(&text);
                    }
                }
            }
        });
        voice_selector.set_factory(Some(&factory));
        voice_selector.set_model(Some(&filtered_model));
    }

    pub fn refresh_voice_selector(voice_selector: &gtk::DropDown, event: VoiceEvent) {
        if let Some(model) = voice_selector.model() {
            if let Some(filter_model) = model.downcast_ref::<gtk::FilterListModel>() {
                if let Some(base_model) = filter_model.model().and_downcast::<gio::ListStore>() {
                    match event {
                        VoiceEvent::Downloaded(voice_key) => {
                            for i in 0..base_model.n_items() {
                                if let Some(voice_row) =
                                    base_model.item(i).and_downcast::<VoiceRow>()
                                {
                                    if voice_row.key() == voice_key {
                                        voice_row.set_downloaded(true);
                                        base_model.items_changed(i, 1, 1);

                                        if let Some(filter) = filter_model
                                            .filter()
                                            .and_downcast::<gtk::CustomFilter>()
                                        {
                                            filter.changed(gtk::FilterChange::Different);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        VoiceEvent::Deleted(voice_key) => {
                            for i in 0..base_model.n_items() {
                                if let Some(voice_row) =
                                    base_model.item(i).and_downcast::<VoiceRow>()
                                {
                                    if voice_row.key() == voice_key {
                                        base_model.remove(i);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
