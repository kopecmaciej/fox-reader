use crate::paths::{dispatcher_config, huggingface_config};
use crate::utils::file_handler::FileHandler;
use crate::core::kokoros_manager::KokorosTTS;
use rodio::buffer::SamplesBuffer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::OnceCell;

// Global Kokoros TTS instance
static KOKOROS_TTS: OnceCell<Arc<KokorosTTS>> = OnceCell::const_new();

pub struct VoiceManager {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Language {
    pub code: String,
    pub name_english: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    size_bytes: u64,
    md5_digest: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Voice {
    pub name: String,
    pub key: String,
    pub language: Language,
    pub quality: String,
    #[serde(default)]
    pub downloaded: bool,
    pub is_default: Option<bool>,
    pub files: HashMap<String, File>,
}

impl VoiceManager {
    pub async fn list_all_available_voices() -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let voices_url = huggingface_config::get_voices_url();
        let voices_file = FileHandler::fetch_file_async(voices_url).await?;
        let raw_json = String::from_utf8(voices_file)?;

        let value_data: Value = serde_json::from_str(&raw_json)?;
        let voices: BTreeMap<String, Voice> = serde_json::from_value(value_data)?;

        let downloaded_voices = Self::list_downloaded_voices()?;
        let default_voice = Self::get_default_voice()?;

        let voices = voices
            .into_iter()
            .map(|(mut key, mut voice)| {
                voice.language.code = voice.language.code.replace("_", "-");
                // we want key to be as voice.onnx for dispatcher config
                voice.key = format!("{}.onnx.json", voice.key);
                key = format!("{}.onnx.json", key);
                voice
                    .files
                    .retain(|f, _| f.ends_with("json") || f.ends_with("onnx"));

                voice.downloaded = downloaded_voices.contains(&voice.key);
                if let Some(ref default_voice) = default_voice {
                    voice.is_default = Some(default_voice == &voice.key);
                }

                (key, voice)
            })
            .collect();

        Ok(voices)
    }

    pub fn list_downloaded_voices() -> Result<Vec<String>, Box<dyn Error>> {
        FileHandler::get_all_file_names(&huggingface_config::get_download_path())
    }

    pub fn get_default_voice() -> Result<Option<String>, Box<dyn Error>> {
        FileHandler::get_default_voice_from_config(&dispatcher_config::get_module_config_path())
    }

    pub async fn download_voice(file_paths: Vec<String>) -> Result<(), Box<dyn Error>> {
        for file_path in file_paths {
            let voice_config_url = huggingface_config::get_voice_url(&file_path);
            let file = FileHandler::fetch_file_async(voice_config_url).await?;
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or("Failed to properly extract file name from path")?;

            FileHandler::save_bytes(&huggingface_config::get_voice_file_path(file_name), &file)?;
        }
        Ok(())
    }

    pub async fn download_voice_samples(
        file_paths: Vec<String>,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        if let Some(file_name) = file_paths.first() {
            let (base_path, _) = file_name.rsplit_once("/").unwrap();
            let sample_path = format!("{}/samples/speaker_0.mp3", base_path);

            let voice_sample_url = huggingface_config::get_voice_url(&sample_path);
            let file = FileHandler::fetch_file_async(voice_sample_url).await?;
            Ok(file)
        } else {
            Err("Invalid file path structure".into())
        }
    }

    pub fn delete_voice(file_paths: Vec<String>) -> Result<(), Box<dyn Error>> {
        for file_path in file_paths {
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or("Failed to properly extract file name from path")?;

            FileHandler::remove_file(&huggingface_config::get_voice_file_path(file_name))?;
        }

        Ok(())
    }

    pub async fn generate_piper_raw_speech(
        text: &str,
        voice_path: &str,
        rate: Option<u8>,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error>> {
        let voice_full_path = format!("{}/{}", huggingface_config::get_download_path(), voice_path);

        return Ok(SamplesBuffer::new(1, 24000, vec![0.0; 24000]));

        // let piper_tts = PiperTTS::new();
        // piper_tts.initialize(&voice_full_path).await?;
        //
        // piper_tts.synthesize_speech(text, rate).await
    }

    // Initialize Kokoros TTS (call this once at app startup)
    pub async fn init_kokoros() -> Result<(), Box<dyn Error + Send + Sync>> {
        let kokoros = KokorosTTS::new().await?;
        KOKOROS_TTS.set(Arc::new(kokoros)).map_err(|_| "Failed to initialize Kokoros TTS")?;
        Ok(())
    }

    // New Kokoros TTS method
    pub async fn generate_kokoros_speech(
        text: &str,
        voice_style: &str,
        speed: f32,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error + Send + Sync>> {
        let kokoros = KOKOROS_TTS.get()
            .ok_or("Kokoros TTS not initialized")?;
        
        kokoros.generate_speech(text, voice_style, speed).await
    }

    // Get available Kokoros voice styles
    pub fn get_kokoros_voices() -> Vec<String> {
        KokorosTTS::get_available_voices()
    }

    // Create Kokoros voice entries compatible with the existing voice system
    pub fn get_kokoros_voice_rows() -> Vec<Voice> {
        let kokoros_voices = KokorosTTS::get_available_voices();
        
        kokoros_voices.into_iter().map(|voice_style| {
            // Create a compatible Voice struct for Kokoros voices
            Voice {
                name: format!("Kokoros {}", voice_style),
                key: format!("kokoros_{}", voice_style),
                language: Language {
                    code: "en-US".to_string(),
                    name_english: "English".to_string(),
                    region: "United States".to_string(),
                },
                quality: "high".to_string(),
                downloaded: true, // Kokoros voices are always "available" once model is downloaded
                is_default: Some(voice_style == "af_sky"), // Make af_sky default
                files: std::collections::HashMap::new(), // No files for Kokoros
            }
        }).collect()
    }

    // Updated method to list all voices (both Piper and Kokoros)
    pub async fn list_all_available_voices_with_kokoros() -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let mut all_voices = BTreeMap::new();
        
        // Add Kokoros voices first (they're always available)
        let kokoros_voices = Self::get_kokoros_voice_rows();
        for voice in kokoros_voices {
            all_voices.insert(voice.key.clone(), voice);
        }
        
        // Add Piper voices (for backward compatibility)
        match Self::list_all_available_voices().await {
            Ok(piper_voices) => {
                all_voices.extend(piper_voices);
            }
            Err(e) => {
                println!("Warning: Could not load Piper voices: {}", e);
                // Continue with just Kokoros voices
            }
        }
        
        Ok(all_voices)
    }

    // Helper method to check if a voice key is Kokoros
    pub fn is_kokoros_voice(voice_key: &str) -> bool {
        voice_key.starts_with("kokoros_")
    }

    // Get Kokoros style name from voice key
    pub fn get_kokoros_style_from_key(voice_key: &str) -> String {
        if voice_key.starts_with("kokoros_") {
            voice_key.strip_prefix("kokoros_").unwrap_or("af_sky").to_string()
        } else {
            "af_sky".to_string() // fallback
        }
    }
}
