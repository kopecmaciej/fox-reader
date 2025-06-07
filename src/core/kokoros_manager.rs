use kokoros::tts::koko::{InitConfig, TTSKoko};
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

        if let Some(parent) = Path::new(&model_path).parent() {
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

        let audio_data = tts_engine
            .tts_raw_audio(text, "en", voice_style, speed, Some(0))
            .map_err(|e| format!("TTS generation failed: {}", e))?;

        let samples_buffer = SamplesBuffer::new(1, self.sample_rate, audio_data);

        Ok(samples_buffer)
    }

    pub fn get_available_voices() -> Vec<String> {
        vec![
            // American English (🇺🇸)
            "af_heart".to_string(),
            "af_alloy".to_string(),
            "af_aoede".to_string(),
            "af_bella".to_string(),
            "af_jessica".to_string(),
            "af_kore".to_string(),
            "af_nicole".to_string(),
            "af_nova".to_string(),
            "af_river".to_string(),
            "af_sarah".to_string(),
            "af_sky".to_string(),
            "am_adam".to_string(),
            "am_echo".to_string(),
            "am_eric".to_string(),
            "am_fenrir".to_string(),
            "am_liam".to_string(),
            "am_michael".to_string(),
            "am_onyx".to_string(),
            "am_puck".to_string(),
            "am_santa".to_string(),
            // British English (🇬🇧)
            "bf_alice".to_string(),
            "bf_emma".to_string(),
            "bf_isabella".to_string(),
            "bf_lily".to_string(),
            "bm_daniel".to_string(),
            "bm_fable".to_string(),
            "bm_george".to_string(),
            "bm_lewis".to_string(),
            // Japanese (🇯🇵)
            "jf_alpha".to_string(),
            "jf_gongitsune".to_string(),
            "jf_nezumi".to_string(),
            "jf_tebukuro".to_string(),
            "jm_kumo".to_string(),
            // Mandarin Chinese (🇨🇳)
            "zf_xiaobei".to_string(),
            "zf_xiaoni".to_string(),
            "zf_xiaoxiao".to_string(),
            "zf_xiaoyi".to_string(),
            "zm_yunjian".to_string(),
            "zm_yunxi".to_string(),
            "zm_yunxia".to_string(),
            "zm_yunyang".to_string(),
            // Spanish (🇪🇸)
            "ef_dora".to_string(),
            "em_alex".to_string(),
            "em_santa".to_string(),
            // French (🇫🇷)
            "ff_siwis".to_string(),
            // Hindi (🇮🇳)
            "hf_alpha".to_string(),
            "hf_beta".to_string(),
            "hm_omega".to_string(),
            "hm_psi".to_string(),
            // Italian (🇮🇹)
            "if_sara".to_string(),
            "im_nicola".to_string(),
            // Brazilian Portuguese (🇧🇷)
            "pf_dora".to_string(),
            "pm_alex".to_string(),
            "pm_santa".to_string(),
        ]
    }

    pub fn supports_voice_mixing() -> bool {
        true
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

