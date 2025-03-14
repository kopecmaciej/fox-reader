use adw::subclass::prelude::*;
use gtk::{
    glib::{self},
    prelude::*,
};
use std::cell::RefCell;

// This enum will represent the different types of messages in the chat
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum MessageType {
    #[default]
    User,
    Assistant,
}

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/ai_chat_row.ui")]
    pub struct ChatMessageRow {
        #[template_child]
        pub message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub message_box: TemplateChild<gtk::Box>,

        pub message_type: RefCell<MessageType>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ChatMessageRow {
        const NAME: &'static str = "ChatMessageRow";
        type Type = super::ChatMessageRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ChatMessageRow {}
    impl WidgetImpl for ChatMessageRow {}
    impl ListBoxRowImpl for ChatMessageRow {}
}

glib::wrapper! {
    pub struct ChatMessageRow(ObjectSubclass<imp::ChatMessageRow>)
        @extends gtk::Widget, gtk::ListBoxRow;
}

impl ChatMessageRow {
    pub fn new(text: &str, message_type: MessageType) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        obj.add_css_class("message");

        imp.message_label.set_text(text);
        imp.message_label.set_wrap(true);
        imp.message_label
            .set_wrap_mode(gtk::pango::WrapMode::WordChar);
        imp.message_label.set_max_width_chars(50);

        // Set the message type
        *imp.message_type.borrow_mut() = message_type;

        // Apply appropriate styling based on message type
        match message_type {
            MessageType::User => {
                imp.message_box.set_halign(gtk::Align::Start);
                imp.message_box.add_css_class("user-message");
            }
            MessageType::Assistant => {
                imp.message_box.set_halign(gtk::Align::End);
                imp.message_box.add_css_class("assistant-message");
            }
        }

        obj
    }

    pub fn message_type(&self) -> MessageType {
        *self.imp().message_type.borrow()
    }

    pub fn set_text(&self, text: &str) {
        self.imp().message_label.set_text(text);
    }
}
