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
    core::{llm_manager::LLMManager, runtime::runtime, voice_manager::VoiceManager},
    utils::audio_player::AudioPlayer,
};

#[derive(Default)]
pub enum State {
    #[default]
    Idle,
    Recording,
    Speaking,
}

mod imp {

    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/ai_chat.ui")]
    pub struct AiChat {
        #[template_child]
        pub mic_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub language_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub button_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub button_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub reset_button: TemplateChild<gtk::Button>,

        pub state: RefCell<State>,
        pub audio_data: RefCell<Option<Vec<f32>>>,
        pub recording_stream: RefCell<Option<cpal::Stream>>,
        pub shared_audio_buffer: RefCell<Option<Arc<Mutex<Vec<f32>>>>>,
        pub llm_manager: Arc<LLMManager>,
        pub audio_player: Arc<AudioPlayer>,
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

            if matches!(*self.state.borrow(), State::Speaking) {
                obj.stop_speaking();
            } else if matches!(*self.state.borrow(), State::Recording) {
                obj.stop_recording();
            } else {
                obj.start_recording();
            }
        }

        #[template_callback]
        fn on_reset_button_clicked(&self, _button: &gtk::Button) {
            let obj = self.obj();
            obj.reset_conversation();
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

        imp.status_label.set_text("Ready to chat");

        imp.language_dropdown.set_selected(0);
    }

    // Method to reset the conversation history
    fn reset_conversation(&self) {
        let imp = self.imp();

        let llm_manager = &*imp.llm_manager.clone();
        llm_manager.reset_conversation();
        imp.status_label.set_text("Conversation reset");

        // Show a temporary notification that fades out
        glib::timeout_add_seconds_local(
            2,
            clone!(
                #[weak(rename_to=this)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Continue,
                move || {
                    if this.imp().status_label.text() == "Conversation reset" {
                        this.imp().status_label.set_text("Ready to chat");
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn stop_speaking(&self) {
        let imp = self.imp();

        imp.audio_player.stop();
        imp.state.replace(State::Idle);

        self.set_mic_button_recording_state(false);

        imp.status_label.set_text("Ready");
    }

    fn get_selected_language_code(&self) -> Option<&'static str> {
        let imp = self.imp();
        let selected_index = imp.language_dropdown.selected();

        match selected_index {
            0 => None,
            1 => Some("en"),
            2 => Some("es"),
            3 => Some("fr"),
            4 => Some("de"),
            5 => Some("it"),
            6 => Some("ja"),
            7 => Some("zh"),
            8 => Some("ru"),
            9 => Some("pt"),
            10 => Some("pl"),
            _ => None,
        }
    }

    // Helper function to set the mic button state
    fn set_mic_button_recording_state(&self, is_recording: bool) {
        let imp = self.imp();

        if is_recording {
            // Change to stop recording state
            imp.button_icon
                .set_icon_name(Some("media-playback-stop-symbolic"));
            imp.button_label.set_text("Stop");
            imp.mic_button.add_css_class("destructive-action");
            imp.mic_button.remove_css_class("suggested-action");
        } else {
            // Change to start recording state
            imp.button_icon.set_icon_name(Some("microphone-symbolic"));
            imp.button_label.set_text("Talk");
            imp.mic_button.remove_css_class("destructive-action");
            imp.mic_button.add_css_class("suggested-action");
        }
    }

    // Helper function to set the mic button to speaking state
    fn set_mic_button_speaking_state(&self, is_speaking: bool) {
        let imp = self.imp();

        if is_speaking {
            // Change to stop speaking state
            imp.button_icon
                .set_icon_name(Some("media-playback-pause-symbolic"));
            imp.button_label.set_text("Stop");
            imp.mic_button.add_css_class("warning");
            imp.mic_button.remove_css_class("suggested-action");
            imp.mic_button.remove_css_class("destructive-action");
        } else {
            // Change to start recording state
            self.set_mic_button_recording_state(false);
            imp.mic_button.remove_css_class("warning");
        }
    }

    fn start_recording(&self) {
        let imp = self.imp();

        imp.state.replace(State::Recording);
        imp.status_label.set_text("Listening...");

        self.set_mic_button_recording_state(true);

        let shared_audio_data = Arc::new(Mutex::new(Vec::<f32>::new()));
        let audio_data_clone = Arc::clone(&shared_audio_data);

        // We'll store this Arc in our struct to access it later
        *imp.audio_data.borrow_mut() = Some(Vec::new());

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
                    println!("Found device: {}", name);
                    name.contains("pipewire") || name.contains("pulse")
                } else {
                    false
                }
            })
            .or_else(|| host.default_input_device())
            .expect("No working input device found");

        println!("Selected device: {:?}", device.name());

        // Configure for 16kHz mono which is what Whisper expects
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

        imp.shared_audio_buffer.replace(Some(shared_audio_data));
    }

    fn stop_recording(&self) {
        let imp = self.imp();

        imp.status_label.set_text("Processing speech...");
        self.set_mic_button_recording_state(false);

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

        // Process the audio data asynchronously
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

    async fn process_audio_and_get_response(
        &self,
        audio_data: Vec<f32>,
        language: Option<&'static str>,
    ) {
        let imp = self.imp();

        // Process the audio to get transcribed text
        let transcription_result = runtime()
            .spawn(async move { Self::process_audio(audio_data, language) })
            .await;

        match transcription_result {
            Ok(Ok(text)) => {
                println!("Transcribed text: {}", text);
                imp.status_label.set_text("Sending to LLM...");

                let llm_manager = imp.llm_manager.clone();

                let response = runtime()
                    .spawn(async move { llm_manager.send_to_lm_studio(&text.clone()).await })
                    .await;

                match response {
                    Ok(Ok(response)) => {
                        self.handle_ai_response(&response).await;
                    }
                    Ok(Err(err)) => {
                        println!("LLM response error: {:?}", err);
                        imp.status_label.set_text("Error: LLM response failed");
                    }
                    Err(err) => {
                        println!("Task join error: {:?}", err);
                        imp.status_label.set_text("Error: Task join failed");
                    }
                }
            }
            Ok(Err(err)) => {
                eprintln!("process_audio error: {}", err);
                imp.status_label.set_text("Error: Audio processing failed");
            }
            Err(join_err) => {
                eprintln!("Tokio task failed: {}", join_err);
                imp.status_label.set_text("Error: Task execution failed");
            }
        }
    }

    fn process_audio(
        audio_data: Vec<f32>,
        language_code: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let whisper_ctx = WhisperContext::new_with_params(
            "/home/cieju/projects/rust/fox-reader/whisper.cpp/models/ggml-base.bin",
            WhisperContextParameters::default(),
        )
        .expect("failed to load model");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_language(language_code);

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

        imp.status_label.set_text("Speaking...");

        imp.state.replace(State::Speaking);
        self.set_mic_button_speaking_state(true);

        let sentences = self.split_into_sentences(response);

        for sentence in sentences {
            if sentence.trim().is_empty() {
                continue;
            }

            //let voice = "pl_PL-gosia-medium.onnx";
            let voice = "en_GB-northern_english_male-medium.onnx";

            let display_sentence = if sentence.len() > 50 {
                format!("{}...", &sentence[0..47])
            } else {
                sentence.clone()
            };
            imp.status_label
                .set_text(&format!("Speaking: {}", display_sentence));

            let raw_audio = runtime()
                .block_on(VoiceManager::generate_piper_raw_speech(&sentence, voice))
                .unwrap();

            let audio_player = self.imp().audio_player.clone();
            let source_audio = AudioPlayer::generate_source(raw_audio, 1.2);
            let play_result = audio_player.play_audio(source_audio);

            if play_result.is_err() {
                println!("Error playing audio: {:?}", play_result.err());
            }
        }

        imp.state.replace(State::Idle);
        self.set_mic_button_speaking_state(false);
        imp.status_label.set_text("Ready");
    }

    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        // Split by common sentence terminators but keep the terminator with the sentence
        let sentence_regex = regex::Regex::new(r"[^.!?]+[.!?]").unwrap();

        let mut sentences: Vec<String> = sentence_regex
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect();

        // If there's any remaining text without a terminator, add it as a final sentence
        let total_matched_len: usize = sentences.iter().map(|s| s.len()).sum();
        if total_matched_len < text.len() {
            let remaining = text[total_matched_len..].trim();
            if !remaining.is_empty() {
                sentences.push(remaining.to_string());
            }
        }

        sentences
    }
}
