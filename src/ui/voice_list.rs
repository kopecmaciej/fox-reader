use adw::subclass::prelude::*;
use gtk::{glib, prelude::ListItemExt};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/voice_list.ui")]
    pub struct VoiceList {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
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

impl VoiceList {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl VoiceList {
    #[template_callback]
    fn setup_play_button(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        let button = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .action_name("voice.play")
            .build();
        item.set_child(Some(&button));
    }

    #[template_callback]
    fn bind_play_button(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Additional binding logic if needed
    }

    #[template_callback]
    fn setup_label(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        let label = gtk::Label::new(None);
        item.set_child(Some(&label));
    }

    #[template_callback]
    fn bind_accent(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Implement accent binding logic
    }

    #[template_callback]
    fn bind_gender(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Implement gender binding logic
    }

    #[template_callback]
    fn bind_country(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Implement country binding logic
    }

    #[template_callback]
    fn setup_actions(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Implement actions setup
    }

    #[template_callback]
    fn bind_actions(_factory: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
        // Implement actions binding
    }
}
