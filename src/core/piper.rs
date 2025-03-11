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

    pub async fn initialize(
        &self,
        model_path: &str,
        speaker_id: Option<i64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let model = piper_rs::from_config_path(Path::new(model_path))
            .map_err(|e| format!("Failed to load Piper model: {}", e))?;

        if let Some(sid) = speaker_id {
            model.set_speaker(sid);
        }

        let synth = PiperSpeechSynthesizer::new(model)
            .map_err(|e| format!("Failed to initialize Piper synthesizer: {}", e))?;

        *self.synthesizer.lock().await = Some(synth);

        Ok(())
    }

    // Rate should be from 5-50 as after some testing >50 start to be too fast for
    // humand to process
    pub async fn synthesize_speech(
        &self,
        text: &str,
        rate: Option<u8>,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error + Send + Sync>> {
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
}
