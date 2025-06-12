use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;

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
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/ai_chat_row.ui")]
    pub struct ChatMessageRow {
        #[template_child]
        pub message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub message_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub copy_button: TemplateChild<gtk::Button>,

        pub message_type: RefCell<MessageType>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ChatMessageRow {
        const NAME: &'static str = "ChatMessageRow";
        type Type = super::ChatMessageRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl ChatMessageRow {
        #[template_callback]
        fn on_copy_button_clicked(&self, _button: &gtk::Button) {
            let text = self.message_label.text();
            let clipboard = self.obj().clipboard();
            clipboard.set_text(&text);

            self.show_copy_feedback();
        }
    }

    impl ChatMessageRow {
        fn show_copy_feedback(&self) {
            let copy_button = &self.copy_button;

            if let Some(image) = copy_button
                .child()
                .and_then(|child| child.downcast::<gtk::Image>().ok())
            {
                image.set_icon_name(Some("object-select-symbolic"));
            }
            copy_button.add_css_class("copy-success");

            glib::spawn_future_local(clone!(
                #[weak]
                copy_button,
                async move {
                    glib::timeout_future_seconds(1).await;
                    if let Some(image) = copy_button
                        .child()
                        .and_then(|child| child.downcast::<gtk::Image>().ok())
                    {
                        image.set_icon_name(Some("edit-copy-symbolic"));
                    }
                    copy_button.remove_css_class("copy-success");
                }
            ));
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

        *imp.message_type.borrow_mut() = message_type;

        match message_type {
            MessageType::User => {
                imp.message_box.add_css_class("user-message");
                imp.message_box.set_halign(gtk::Align::End);
            }
            MessageType::Assistant => {
                imp.message_box.add_css_class("assistant-message");
                imp.message_box.set_halign(gtk::Align::Start);
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
