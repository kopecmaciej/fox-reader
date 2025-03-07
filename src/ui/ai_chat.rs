use adw::subclass::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::{
    core::{runtime::runtime, voice_manager::VoiceManager},
    utils::audio_player::AudioPlayer,
};

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

        pub is_recording: RefCell<bool>,
        pub is_speaking: RefCell<bool>,
        pub audio_data: RefCell<Option<Vec<f32>>>,
        pub http_client: RefCell<Client>,
        pub recording_stream: RefCell<Option<cpal::Stream>>,
        pub shared_audio_buffer: RefCell<Option<Arc<Mutex<Vec<f32>>>>>,
        pub stop_speaking: RefCell<bool>,
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

            if *self.is_speaking.borrow() {
                // If speaking, stop the speech
                obj.stop_speaking();
            } else if *self.is_recording.borrow() {
                // If recording, stop recording
                obj.stop_recording();
            } else {
                // Otherwise, start recording
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

        // Initialize HTTP client for LMstudio API
        *imp.http_client.borrow_mut() = Client::new();

        // Select default language (Auto-detect)
        imp.language_dropdown.set_selected(0);

        // Initialize state flags
        *imp.is_recording.borrow_mut() = false;
        *imp.is_speaking.borrow_mut() = false;
        *imp.stop_speaking.borrow_mut() = false;
    }

    // Method to stop speech playback
    fn stop_speaking(&self) {
        let imp = self.imp();

        println!("Stopping speech playback");
        *imp.stop_speaking.borrow_mut() = true;
        *imp.is_speaking.borrow_mut() = false;

        // Reset button to microphone
        self.set_mic_button_recording_state(false);

        imp.status_label.set_text("Ready");
    }

    // Get the selected language code for Whisper
    fn get_selected_language_code(&self) -> Option<&'static str> {
        let imp = self.imp();
        let selected_index = imp.language_dropdown.selected();

        match selected_index {
            0 => None, // Auto-detect
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
            _ => None, // Default to auto-detect for unexpected values
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

        // Update UI to show recording in progress
        *imp.is_recording.borrow_mut() = true;
        *imp.stop_speaking.borrow_mut() = false;
        imp.status_label.set_text("Listening...");

        // Change button appearance to indicate recording
        self.set_mic_button_recording_state(true);

        // Initialize the audio data container - important to start with an empty vec
        let shared_audio_data = Arc::new(Mutex::new(Vec::<f32>::new()));
        let audio_data_clone = Arc::clone(&shared_audio_data);

        // We'll store this Arc in our struct to access it later
        *imp.audio_data.borrow_mut() = Some(Vec::new());

        // Try to use PipeWire explicitly if available
        let host = cpal::host_from_id(
            cpal::available_hosts()
                .into_iter()
                .find(|id| *id == cpal::HostId::Alsa)
                .unwrap_or_else(|| cpal::default_host().id()),
        )
        .expect("Failed to initialize audio host");

        println!("Using audio host: {:?}", host.id());

        // List available input devices
        println!("Available input devices:");
        let mut input_devices = host
            .input_devices()
            .unwrap_or_else(|_| panic!("Error getting input devices"));

        // Try to find the pipewire device specifically
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

        println!("Using config: {:?}", config);

        // Create an input stream for recording
        let err_fn = move |err| {
            eprintln!("An error occurred on the input audio stream: {}", err);
        };

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    // Store the audio data in our shared buffer
                    let mut buffer = audio_data_clone.lock().unwrap();
                    buffer.extend_from_slice(data);
                },
                err_fn,
                None,
            )
            .expect("Failed to build input stream");

        // Start the recording
        stream.play().expect("Failed to start recording stream");
        *imp.recording_stream.borrow_mut() = Some(stream);

        // Store the shared audio buffer in a place where stop_recording can access it
        imp.shared_audio_buffer.replace(Some(shared_audio_data));

        println!("Recording started successfully");
    }

    // And update the stop_recording function to retrieve the data
    fn stop_recording(&self) {
        let imp = self.imp();

        // Update UI to show processing
        *imp.is_recording.borrow_mut() = false;
        imp.status_label.set_text("Processing speech...");

        // Change button appearance to normal state
        self.set_mic_button_recording_state(false);

        if let Some(stream) = imp.recording_stream.borrow_mut().take() {
            println!("Recording stream stopped");
        }

        // Retrieve the recorded audio data from our shared buffer
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
                    // If we couldn't get exclusive ownership, clone the data
                    let buffer = arc.lock().unwrap().clone();
                    println!("Cloned {} audio samples", buffer.len());
                    buffer
                }
            }
        } else {
            println!("No audio data found");
            imp.status_label.set_text("Error: No audio recorded");
            return;
        };

        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                match this.process_audio(audio_data) {
                    Ok(text) => {
                        println!("Transcribed text: {}", text);
                        this.imp().status_label.set_text("Sending to LLM...");

                        match this.send_to_lm_studio(&text).await {
                            Ok(response) => {
                                println!("LLM Response: {}", response);
                                this.handle_ai_response(&response).await
                            }
                            Err(e) => {
                                println!("LLM API error: {:?}", e);
                                this.imp()
                                    .status_label
                                    .set_text("Error: Failed to get LLM response");
                            }
                        }
                    }
                    Err(e) => {
                        println!("Speech recognition error: {:?}", e);
                        this.imp()
                            .status_label
                            .set_text("Error: Failed to recognize speech");
                    }
                }
            }
        ));
    }

    fn process_audio(&self, audio_data: Vec<f32>) -> Result<String, Box<dyn std::error::Error>> {
        let whisper_ctx = WhisperContext::new_with_params(
            "/home/cieju/projects/rust/fox-reader/whisper.cpp/models/ggml-base.bin",
            WhisperContextParameters::default(),
        )
        .expect("failed to load model");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Use the selected language, or auto if none selected
        let language_code = self.get_selected_language_code();
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

        Ok(text.trim().to_string())
    }

    async fn send_to_lm_studio(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let imp = self.imp();

        // Improved system prompt for voice conversations
        let system_prompt = "You are a helpful voice assistant. Respond in a conversational, natural way. Use short, clear sentences and avoid complex formatting, lists, or code. Keep responses concise and easy to listen to. Speak as if you're having a casual conversation. Use simple language that's easy to follow when heard rather than read.";

        let request_body = json!({
            "model": "model-identifier",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 300,  // Limit response length for voice
        });

        let request = {
            let client = imp.http_client.borrow();
            client
                .post("http://localhost:1234/v1/chat/completions")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer lm-studio")
                .json(&request_body)
        };

        let response = runtime().block_on(request.send())?;

        let response_json: Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        Ok(content)
    }

    async fn handle_ai_response(&self, response: &str) {
        let imp = self.imp();

        // Set UI to speaking state
        imp.status_label.set_text("Speaking...");
        *imp.is_speaking.borrow_mut() = true;
        *imp.stop_speaking.borrow_mut() = false;
        self.set_mic_button_speaking_state(true);

        // Split by sentences for more natural TTS playback
        let sentences = self.split_into_sentences(response);

        for sentence in sentences {
            // Check if speaking has been stopped
            if *imp.stop_speaking.borrow() {
                println!("Speech playback canceled by user");
                break;
            }

            // Skip empty sentences
            if sentence.trim().is_empty() {
                continue;
            }

            let voice = "en_US-ryan-high.onnx";

            // Show current sentence in status label (truncated if too long)
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

            let audio_player = AudioPlayer::new();
            let source_audio = AudioPlayer::generate_source(raw_audio, 1.2);
            let play_result = audio_player.play_audio(source_audio);

            if play_result.is_err() {
                println!("Error playing audio: {:?}", play_result.err());
            }

            // Check again if speaking has been stopped
            if *imp.stop_speaking.borrow() {
                println!("Speech playback canceled by user");
                break;
            }
        }

        // Reset state
        *imp.is_speaking.borrow_mut() = false;
        *imp.stop_speaking.borrow_mut() = false;
        self.set_mic_button_speaking_state(false);
        imp.status_label.set_text("Ready");
    }

    // Helper function to split text into more natural sentences for TTS
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
