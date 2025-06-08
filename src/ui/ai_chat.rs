use adw::subclass::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::{
    core::{llm_manager::LLMManager, runtime::spawn_tokio, voice_manager::VoiceManager},
    paths::whisper_config::get_model_path,
    settings::Settings,
    ui::dialogs::show_error_dialog,
    utils::{audio_player, markdown, text},
    SETTINGS,
};

use super::{
    ai_chat_row::{ChatMessageRow, MessageType},
    helpers::voice_selector,
    voice_row::VoiceRow,
};

const WELCOME_MESSAGE: &str = "Hello! I'm your AI voice assistant. Click the microphone button and start speaking to chat with me.";

#[derive(Default, PartialEq)]
pub enum State {
    #[default]
    Idle,
    Recording,
    Speaking,
    Stopped,
}

mod imp {

    use crate::utils::whisper_downloader;

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/ai_chat.ui")]
    pub struct AiChat {
        #[template_child]
        pub mic_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub voice_selector: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub button_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub chat_history_list: TemplateChild<gtk::ListBox>,

        pub show_history: RefCell<bool>,
        #[template_child]
        pub history_toggle_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub history_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub main_container: TemplateChild<gtk::Box>,

        pub state: RefCell<State>,
        pub recording_stream: RefCell<Option<cpal::Stream>>,
        pub shared_audio_buffer: RefCell<Option<Arc<Mutex<Vec<f32>>>>>,
        pub llm_manager: Arc<LLMManager>,
        pub audio_player: Arc<audio_player::AudioPlayer>,
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
        fn on_mic_button_clicked(&self, _button: &gtk::Button) {
            let obj = self.obj();
            let mut state = self.state.borrow_mut();

            if whisper_downloader::get_downloaded_models().is_empty() {
                glib::spawn_future_local(clone!(
                    #[weak]
                    obj,
                    async move {
                        obj.imp()
                            .status_label
                            .set_text("No whisper model available, go to settings to download");
                        obj.imp().mic_button.add_css_class("destructive-action");
                        glib::timeout_future_seconds(2).await;
                        obj.imp().status_label.set_text("Ready to chat");
                        obj.imp().mic_button.remove_css_class("destructive-action");
                    }
                ));
                return;
            }

            match *state {
                State::Idle => {
                    *state = State::Recording;
                    obj.start_recording()
                }
                State::Recording => obj.stop_recording(),
                State::Speaking => {
                    *state = State::Stopped;
                    obj.stop_speaking()
                }
                _ => {}
            }
        }

        #[template_callback]
        fn on_reset_button_clicked(&self, _button: &gtk::Button) {
            self.obj().reset_conversation();
        }

        #[template_callback]
        fn on_history_toggle_clicked(&self, _button: &gtk::Button) {
            self.obj().toggle_history_visibility();
        }
    }

    impl ObjectImpl for AiChat {}
    impl WidgetImpl for AiChat {}
    impl BinImpl for AiChat {}
}

glib::wrapper! {
    pub struct AiChat(ObjectSubclass<imp::AiChat>)
        @extends gtk::Widget, adw::Bin;
}

impl AiChat {
    pub fn init(&self) {
        self.setup_chat_history();
        self.setup_history_toggle();
    }

    fn setup_history_toggle(&self) {
        let imp = self.imp();
        imp.history_revealer.set_reveal_child(false);
        imp.history_toggle_button.set_label("Show History");
        imp.main_container.add_css_class("history-hidden");
    }

    fn toggle_history_visibility(&self) {
        let imp = self.imp();
        let mut show_history = imp.show_history.borrow_mut();
        *show_history = !*show_history;

        imp.history_revealer.set_reveal_child(*show_history);

        if *show_history {
            imp.main_container.remove_css_class("history-hidden");
            imp.history_toggle_button.set_label("Hide History");
        } else {
            imp.main_container.add_css_class("history-hidden");
            imp.history_toggle_button.set_label("Show History");
        }
    }

    fn setup_chat_history(&self) {
        let imp = self.imp();
        imp.chat_history_list
            .set_selection_mode(gtk::SelectionMode::None);

        self.add_message_to_chat(WELCOME_MESSAGE, MessageType::Assistant);
    }

    pub fn add_message_to_chat(&self, message: &str, message_type: MessageType) {
        let imp = self.imp();

        let row = ChatMessageRow::new(message, message_type);
        imp.chat_history_list.append(&row);

        // TODO: fix Auto-scroll to the bottom
        if let Some(scrolled_window) = imp
            .chat_history_list
            .ancestor(gtk::ScrolledWindow::static_type())
        {
            let adj = scrolled_window
                .downcast_ref::<gtk::ScrolledWindow>()
                .unwrap()
                .vadjustment();
            adj.set_value(adj.upper() - adj.page_size());
        }
    }

    pub fn populate_voice_selector(&self, voices: &[VoiceRow]) {
        voice_selector::populate_voice_selector(&self.imp().voice_selector, voices);

        self.set_default_voice_from_settings();
    }

    pub fn set_default_voice_from_settings(&self) {
        let settings = Settings::default();
        let default_voice_key = settings.get_default_voice();

        if !default_voice_key.is_empty() {
            voice_selector::set_selected_voice_by_key(
                &self.imp().voice_selector,
                &default_voice_key,
            );
        } else {
            println!("No default voice set in settings for AI chat");
        }
    }

    fn reset_conversation(&self) {
        let imp = self.imp();

        let llm_manager = &*imp.llm_manager.clone();
        llm_manager.reset_conversation();

        while let Some(child) = imp.chat_history_list.first_child() {
            imp.chat_history_list.remove(&child);
        }

        self.add_message_to_chat(WELCOME_MESSAGE, MessageType::Assistant);

        glib::spawn_future_local(clone!(
            #[weak]
            imp,
            async move {
                imp.status_label.set_text("Conversation reset");
                glib::timeout_future_seconds(1).await;
                imp.status_label.set_text("Ready to chat");
            }
        ));
    }

    fn stop_speaking(&self) {
        let imp = self.imp();

        imp.audio_player.stop();
        imp.status_label.set_text("Ready");
    }

    pub fn get_selected_language_code(&self) -> Option<String> {
        if let Some(voice_row) = voice_selector::get_selected_voice(&self.imp().voice_selector) {
            return Some(voice_row.language_code().split("-").collect::<Vec<_>>()[0].to_string());
        }
        None
    }

    fn start_recording(&self) {
        let imp = self.imp();

        imp.status_label.set_text("Listening...");
        imp.button_icon
            .set_icon_name(Some("media-playback-stop-symbolic"));

        let shared_audio_data = Arc::new(Mutex::new(Vec::<f32>::new()));
        let audio_data_clone = Arc::clone(&shared_audio_data);

        let host = cpal::host_from_id(
            cpal::available_hosts()
                .into_iter()
                .find(|id| *id == cpal::HostId::Alsa)
                .unwrap_or_else(|| cpal::default_host().id()),
        )
        .expect("Failed to initialize audio host");

        let mut input_devices = host
            .input_devices()
            .unwrap_or_else(|_| panic!("Error getting input devices"));

        let device = input_devices
            .find(|d| {
                if let Ok(name) = d.name() {
                    name.contains("pipewire") || name.contains("pulse")
                } else {
                    false
                }
            })
            .or_else(|| host.default_input_device())
            .expect("No working input device found");

        // 16kHz mono is what Whisper expects
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        };

        let err_fn = move |err| {
            eprintln!("An error occurred on the input audio stream: {}", err);
        };

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    let mut buffer = audio_data_clone.lock().unwrap();
                    buffer.extend_from_slice(data);
                },
                err_fn,
                None,
            )
            .expect("Failed to build input stream");

        stream.play().expect("Failed to start recording stream");
        *imp.recording_stream.borrow_mut() = Some(stream);

        *imp.shared_audio_buffer.borrow_mut() = Some(shared_audio_data);
    }

    fn stop_recording(&self) {
        let imp = self.imp();

        imp.button_icon.set_icon_name(Some("system-run"));
        imp.status_label.set_text("Processing speech...");

        if let Some(stream) = imp.recording_stream.borrow_mut().take() {
            drop(stream);
        }

        let audio_data = if let Some(shared_buffer) = imp.shared_audio_buffer.take() {
            match Arc::try_unwrap(shared_buffer) {
                Ok(mutex) => {
                    let buffer = mutex.into_inner().unwrap();
                    if buffer.is_empty() {
                        println!("Warning: Recorded audio buffer is empty!");
                    } else {
                        println!("Retrieved {} audio samples", buffer.len());
                    }
                    buffer
                }
                Err(arc) => {
                    let buffer = arc.lock().unwrap().clone();
                    buffer
                }
            }
        } else {
            imp.status_label.set_text("Error: No audio recorded");
            return;
        };

        let language = self.get_selected_language_code();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                this.process_audio_and_get_response(audio_data, language)
                    .await;
            }
        ));
    }

    async fn process_audio_and_get_response(&self, audio_data: Vec<f32>, language: Option<String>) {
        let imp = self.imp();

        let transcription_result =
            spawn_tokio(async move { Self::process_audio(audio_data, language) }).await;
        match transcription_result {
            Ok(text) => {
                self.add_message_to_chat(&text, MessageType::User);

                imp.status_label.set_text("Sending to LLM...");

                let llm_manager = imp.llm_manager.clone();
                let response =
                    spawn_tokio(async move { llm_manager.send_to_llm(&text.clone()).await }).await;
                match response {
                    Ok(response) => {
                        self.handle_ai_response(&response).await;
                    }
                    Err(e) => {
                        show_error_dialog(&format!("LLM response error: {}", e), self);
                        imp.status_label.set_text("Error: LLM response failed");
                    }
                }
            }
            Err(e) => {
                show_error_dialog(&format!("Process audio error: {}", e), self);
                imp.status_label.set_text("Error: Audio processing failed");
            }
        }
        *imp.state.borrow_mut() = State::Idle;
        imp.status_label.set_text("Ready");
        imp.button_icon
            .set_icon_name(Some("microphone-sensitivity-high-symbolic"));
    }

    fn process_audio(
        audio_data: Vec<f32>,
        language_code: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let model = get_model_path(&SETTINGS.get_whisper_model());
        let whisper_ctx =
            WhisperContext::new_with_params(&model, WhisperContextParameters::default())
                .expect("failed to load model");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_language(language_code.as_deref());

        params.set_print_progress(false);
        params.set_print_special(false);

        let mut state = whisper_ctx.create_state()?;
        state.full(params, &audio_data)?;

        let num_segments = state.full_n_segments()?;
        let mut text = String::new();

        for i in 0..num_segments {
            text.push_str(&state.full_get_segment_text(i)?);
            text.push(' ');
        }

        let lang_id = state.full_lang_id_from_state()?;
        let _lang = whisper_rs::get_lang_str(lang_id);

        Ok(text.trim().to_string())
    }

    async fn handle_ai_response(&self, response: &str) {
        let imp = self.imp();

        self.add_message_to_chat(response, MessageType::Assistant);

        imp.status_label.set_text("Speaking...");
        {
            *imp.state.borrow_mut() = State::Speaking;
        }
        imp.button_icon
            .set_icon_name(Some("media-playback-stop-symbolic"));

        let tts_text = markdown::strip_markdown_for_tts(response);
        let sentences: Vec<String> = text::split_text_into_sentences(&tts_text)
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect();

        if sentences.is_empty() {
            return;
        }

        let voice = if let Some(v) = voice_selector::get_selected_voice(&self.imp().voice_selector)
        {
            v.key()
        } else {
            return;
        };

        let speed = 1.0;
        let mut current_audio = None;

        if let Some(first_sentence) = sentences.first() {
            let voice_clone = voice.clone();
            let sentence_clone = first_sentence.clone();

            match spawn_tokio(async move {
                VoiceManager::generate_kokoros_speech(&sentence_clone, &voice_clone, speed).await
            })
            .await
            {
                Ok(audio) => current_audio = Some(audio),
                Err(e) => {
                    show_error_dialog(&format!("Error generating TTS: {}", e), self);
                    return;
                }
            }
        }

        for i in 0..sentences.len() {
            {
                let mut state = imp.state.borrow_mut();
                if *state == State::Stopped {
                    *state = State::Idle;
                    break;
                }
            }

            let audio_to_play = if let Some(audio) = current_audio.take() {
                audio
            } else {
                continue;
            };

            let next_index = i + 1;
            if next_index < sentences.len() {
                let next_sentence = sentences[next_index].clone();
                let voice_clone = voice.clone();

                let audio_player = self.imp().audio_player.clone();
                let play_future =
                    spawn_tokio(async move { audio_player.play_audio(audio_to_play) });

                let generate_future = spawn_tokio(async move {
                    VoiceManager::generate_kokoros_speech(&next_sentence, &voice_clone, speed).await
                });

                let (play_result, generate_result) = tokio::join!(play_future, generate_future);

                if let Err(e) = play_result {
                    show_error_dialog(&format!("Error playing audio: {}", e), self);
                }

                match generate_result {
                    Ok(next_audio) => current_audio = Some(next_audio),
                    Err(e) => {
                        show_error_dialog(&format!("Error generating TTS: {}", e), self);
                        break;
                    }
                }
            } else {
                let audio_player = self.imp().audio_player.clone();
                if let Err(e) =
                    spawn_tokio(async move { audio_player.play_audio(audio_to_play) }).await
                {
                    show_error_dialog(&format!("Error playing audio: {}", e), self);
                }
            }
        }

        {
            *imp.state.borrow_mut() = State::Idle;
        }
        imp.status_label.set_text("Ready");
        imp.button_icon
            .set_icon_name(Some("audio-input-microphone-symbolic"));
    }
}
