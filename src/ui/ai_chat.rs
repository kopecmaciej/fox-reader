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

        pub is_recording: RefCell<bool>,
        pub audio_data: RefCell<Option<Vec<f32>>>,
        pub http_client: RefCell<Client>,
        pub recording_stream: RefCell<Option<cpal::Stream>>,
        pub shared_audio_buffer: RefCell<Option<Arc<Mutex<Vec<f32>>>>>,
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

        // Initialize HTTP client for LMstudio API
        *imp.http_client.borrow_mut() = Client::new();
    }

    fn start_recording(&self) {
        let imp = self.imp();

        // Update UI to show recording in progress
        *imp.is_recording.borrow_mut() = true;
        imp.status_label.set_text("Listening...");

        // Change button appearance to indicate recording
        imp.mic_button.add_css_class("destructive-action");
        imp.mic_button.remove_css_class("suggested-action");

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

        // Restore button appearance
        imp.mic_button.remove_css_class("destructive-action");
        imp.mic_button.add_css_class("suggested-action");

        // Stop the recording by dropping the stream
        if let Some(stream) = imp.recording_stream.borrow_mut().take() {
            // The stream is dropped here, which stops recording
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

        // Now we have the audio data, continue with processing...

        // Process the recording in a background task
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                // Process the audio data with Whisper to get text
                match this.process_audio(audio_data) {
                    Ok(text) => {
                        println!("Transcribed text: {}", text);
                        this.imp().status_label.set_text("Sending to LLM...");

                        // Send to LLM and get response
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
            "/home/cieju/projects/rust/fox-reader/whisper.cpp/models/ggml-base.en.bin",
            WhisperContextParameters::default(),
        )
        .expect("failed to load model");

        // Resample to 16kHz if needed (assuming the data is already in the right format here)
        // In a real implementation, you might need to convert sample rate

        // Set up parameters for Whisper
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en")); // Set language to English
        params.set_print_progress(false);
        params.set_print_special(false);

        // Process the audio with Whisper
        let mut state = whisper_ctx.create_state()?;
        state.full(params, &audio_data)?;

        // Extract the text from the result
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

        // Prepare the request to LMstudio API
        let request_body = json!({
            "model": "model-identifier",  // Replace with your actual model identifier
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
        });

        let request = {
            let client = imp.http_client.borrow();
            client
                .post("http://localhost:1234/v1/chat/completions")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer lm-studio")
                .json(&request_body)
        }; // client is dropped here when the scope ends

        let response = runtime().block_on(request.send())?;

        // Process the response
        let response_json: Value = response.json().await?;

        // Extract the message content
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to parse response")
            .to_string();

        Ok(content)
    }

    async fn handle_ai_response(&self, response: &str) {
        let imp = self.imp();

        imp.status_label.set_text("Ready");

        response.split(".").for_each(|text| {
            let voice = "en_US-ryan-high.onnx";
            let raw_audio = runtime()
                .block_on(VoiceManager::generate_piper_raw_speech(text, voice))
                .unwrap();

            let audio_player = AudioPlayer::new();
            audio_player.play_wav(raw_audio, 1.2).unwrap();
        });
    }
}
