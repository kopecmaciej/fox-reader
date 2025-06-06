use kokoros::tts::koko::{TTSKoko, InitConfig};
use rodio::buffer::SamplesBuffer;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::paths::voice_config;

pub struct KokorosTTS {
    tts_engine: Arc<Mutex<TTSKoko>>,
    sample_rate: u32,
}

impl KokorosTTS {
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let model_path = voice_config::get_kokoros_model_path();
        let voices_path = voice_config::get_kokoros_voices_path();
        
        // Ensure parent directories exist
        if let Some(parent) = Path::new(&model_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        if let Some(parent) = Path::new(&voices_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let config = InitConfig::default();
        let sample_rate = config.sample_rate;
        
        let tts_engine = TTSKoko::from_config(&model_path, &voices_path, config).await;
        
        Ok(Self {
            tts_engine: Arc::new(Mutex::new(tts_engine)),
            sample_rate,
        })
    }

    pub async fn generate_speech(
        &self,
        text: &str,
        voice_style: &str,
        speed: f32,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error + Send + Sync>> {
        let tts_engine = self.tts_engine.lock().await;
        
        // Use English as default language for now
        let audio_data = tts_engine.tts_raw_audio(
            text,
            "en",
            voice_style,
            speed,
            Some(0), // No initial silence
        ).map_err(|e| format!("TTS generation failed: {}", e))?;

        // Convert Vec<f32> to SamplesBuffer<f32> for compatibility with rodio
        let samples_buffer = SamplesBuffer::new(1, self.sample_rate, audio_data);
        
        Ok(samples_buffer)
    }

    pub fn get_available_voices() -> Vec<String> {
        // Based on Kokoros documentation, these are the available voice styles
        vec![
            "af_sky".to_string(),
            "af_bella".to_string(),
            "af_sarah".to_string(),
            "af_nicole".to_string(),
        ]
    }

    pub fn supports_voice_mixing() -> bool {
        true
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
} 