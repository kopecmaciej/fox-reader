use piper_rs::synth::{AudioOutputConfig, PiperSpeechSynthesizer};
use rodio::buffer::SamplesBuffer;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PiperTTS {
    synthesizer: Arc<Mutex<Option<PiperSpeechSynthesizer>>>,
}

impl Default for PiperTTS {
    fn default() -> Self {
        Self {
            synthesizer: Arc::new(Mutex::new(None)),
        }
    }
}

impl PiperTTS {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn initialize(&self, model_path: &str) -> Result<(), Box<dyn Error>> {
        if !model_path.ends_with("json") {
            return Err("Voice model should be loaded as .json file".into());
        }
        let model = piper_rs::from_config_path(Path::new(model_path))
            .map_err(|e| format!("Failed to load Piper model: {}", e))?;

        let synth = PiperSpeechSynthesizer::new(model)
            .map_err(|e| format!("Failed to initialize Piper synthesizer: {}", e))?;

        *self.synthesizer.lock().await = Some(synth);

        Ok(())
    }

    pub async fn synthesize_speech(
        &self,
        text: &str,
        rate: Option<u8>,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error>> {
        let synth_guard = self.synthesizer.lock().await;
        let synth = synth_guard
            .as_ref()
            .ok_or_else(|| "Piper synthesizer not initialized".to_string())?;

        let option = AudioOutputConfig {
            rate,
            volume: None,
            pitch: None,
            appended_silence_ms: Some(100),
        };

        let mut samples: Vec<f32> = Vec::new();
        let audio_results = synth
            .synthesize_parallel(text.to_string(), Some(option))
            .map_err(|e| format!("Failed to synthesize speech: {}", e))?;

        for result in audio_results {
            let chunk = result.map_err(|e| format!("Failed to get audio chunk: {}", e))?;
            samples.append(&mut chunk.into_vec());
        }

        Ok(SamplesBuffer::new(1, 22050, samples))
    }

    pub async fn synthesize_speech_to_wav(
        &self,
        text: &str,
        output_path: &str,
        rate: Option<u8>,
    ) -> Result<(), Box<dyn Error>> {
        let synth_guard = self.synthesizer.lock().await;
        let synth = synth_guard
            .as_ref()
            .ok_or_else(|| "Piper synthesizer not initialized".to_string())?;

        let option = AudioOutputConfig {
            rate,
            volume: None,
            pitch: None,
            appended_silence_ms: None,
        };

        synth
            .synthesize_to_file(Path::new(output_path), text.to_string(), Some(option))
            .map_err(|e| format!("Failed to save speech to file: {}", e))?;

        Ok(())
    }
}
