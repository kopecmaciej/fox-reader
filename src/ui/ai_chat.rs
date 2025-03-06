use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::cell::RefCell;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/ai_chat.ui")]
    pub struct AiChat {
        #[template_child]
        pub mic_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,

        pub is_recording: RefCell<bool>,
        pub audio_data: RefCell<Option<Vec<u8>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChat {
        const NAME: &'static str = "AiChat";
        type Type = super::AiChat;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl AiChat {
        #[template_callback]
        fn on_mic_button_clicked(&self, button: &gtk::Button) {
            let obj = self.obj();
            if *self.is_recording.borrow() {
                obj.stop_recording();
            } else {
                obj.start_recording();
            }
        }
    }

    impl ObjectImpl for AiChat {}
    impl WidgetImpl for AiChat {}
    impl BinImpl for AiChat {}
}

glib::wrapper! {
    pub struct AiChat(ObjectSubclass<imp::AiChat>)
        @extends gtk::Widget;
}

impl AiChat {
    pub fn init(&self) {
        let imp = self.imp();

        // Set up initial UI state
        imp.status_label.set_text("Ready to chat");
    }

    fn start_recording(&self) {
        let imp = self.imp();

        // Update UI to show recording in progress
        *imp.is_recording.borrow_mut() = true;
        imp.status_label.set_text("Listening...");

        // Change button appearance to indicate recording
        imp.mic_button.add_css_class("destructive-action");
        imp.mic_button.remove_css_class("suggested-action");

        // TODO: Start actual audio recording
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                // Placeholder for audio recording logic
                // For now, simulate recording with a delay
                glib::timeout_future(std::time::Duration::from_secs(1)).await;

                // Keep recording state until user stops or timeout
                if *this.imp().is_recording.borrow() {
                    // Continue recording
                }
            }
        ));
    }

    fn stop_recording(&self) {
        let imp = self.imp();

        // Update UI to show processing
        *imp.is_recording.borrow_mut() = false;
        imp.status_label.set_text("Processing...");

        // Restore button appearance
        imp.mic_button.remove_css_class("destructive-action");
        imp.mic_button.add_css_class("suggested-action");

        // Placeholder for stopping recording and processing audio
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                // Simulate processing with a delay
                glib::timeout_future(std::time::Duration::from_secs(2)).await;

                // Here you would send the audio to speech-to-text service
                // Then send the text to LLM
                // For now just simulate a response
                this.handle_ai_response("This is a simulated AI response.");
            }
        ));
    }

    fn handle_ai_response(&self, response: &str) {
        let imp = self.imp();

        // Update UI to show response received
        imp.status_label.set_text("Ready");

        // In future, this would trigger text-to-speech for the AI response
        // For now, just show a dialog with the response
    }
}
