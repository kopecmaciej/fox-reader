use super::{voice_row::VoiceRow};
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
        
        // Sort voices by country and then by name for better organization
        let mut sorted_rows = all_rows.to_vec();
        sorted_rows.sort_by(|a, b| {
            // Extract country from language display (for non-Kokoros voices) or voice name (for Kokoros)
            let get_country = |voice: &VoiceRow| -> String {
                let name = voice.name();
                let language = voice.language();
                
                // For Kokoros voices, extract country from flag emojis in name
                if name.contains("🇺🇸") { return "1_United States".to_string(); }
                if name.contains("🇬🇧") { return "2_United Kingdom".to_string(); }
                if name.contains("🇯🇵") { return "3_Japan".to_string(); }
                if name.contains("🇨🇳") { return "4_China".to_string(); }
                if name.contains("🇪🇸") { return "5_Spain".to_string(); }
                if name.contains("🇫🇷") { return "6_France".to_string(); }
                if name.contains("🇮🇳") { return "7_India".to_string(); }
                if name.contains("🇮🇹") { return "8_Italy".to_string(); }
                if name.contains("🇧🇷") { return "9_Brazil".to_string(); }
                
                // For other voices, try to extract from language
                if language.contains("United States") { return "1_United States".to_string(); }
                if language.contains("United Kingdom") { return "2_United Kingdom".to_string(); }
                
                // Default fallback
                format!("Z_{}", language)
            };
            
            let country_a = get_country(a);
            let country_b = get_country(b);
            
            // First sort by country, then by voice name
            country_a.cmp(&country_b).then_with(|| a.name().cmp(&b.name()))
        });
        
        model.extend_from_slice(&sorted_rows);


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
                        // Show traits separately from name to avoid duplication
                        let text = if v.name().contains("🇺🇸") || v.name().contains("🇬🇧") || 
                                      v.name().contains("🇯🇵") || v.name().contains("🇨🇳") ||
                                      v.name().contains("🇪🇸") || v.name().contains("🇫🇷") ||
                                      v.name().contains("🇮🇳") || v.name().contains("🇮🇹") ||
                                      v.name().contains("🇧🇷") {
                            // For Kokoros voices, show traits + name + quality
                            if !v.traits().is_empty() {
                                format!("{} {} ({})", v.traits(), v.name(), v.quality())
                            } else {
                                format!("{} ({})", v.name(), v.quality())
                            }
                        } else {
                            // For other voices, show traditional format with traits if available
                            if !v.traits().is_empty() {
                                format!("{} {} ({}) - {}", v.traits(), v.name(), v.quality(), v.language())
                            } else {
                                format!("{} ({}) - {}", v.name(), v.quality(), v.language())
                            }
                        };
                        label.set_text(&text);
                    }
                }
            }
        });
        voice_selector.set_factory(Some(&factory));
        voice_selector.set_model(Some(&model));
    }


}
