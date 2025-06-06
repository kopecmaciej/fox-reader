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
        let mut kokoros_voices = KokorosTTS::get_available_voices();
        
        // Sort voices by country first, then by voice name for better organization
        kokoros_voices.sort_by(|a, b| {
            let (_, _, country_a, _) = Self::get_language_info_from_voice_style(a);
            let (_, _, country_b, _) = Self::get_language_info_from_voice_style(b);
            
            // First sort by country, then by voice name
            country_a.cmp(&country_b).then_with(|| a.cmp(b))
        });
        
        kokoros_voices.into_iter().map(|voice_style| {
            let (language_code, language_name, region, _flag) = Self::get_language_info_from_voice_style(&voice_style);
            let friendly_name = Self::get_friendly_voice_name(&voice_style);
            
            // Create a compatible Voice struct for Kokoros voices
            Voice {
                name: friendly_name,
                key: format!("kokoros_{}", voice_style),
                language: Language {
                    code: language_code,
                    name_english: language_name,
                    region,
                },
                quality: "high".to_string(),
                downloaded: true, // Kokoros voices are always "available" once model is downloaded
                is_default: Some(voice_style == "af_heart"), // Make af_heart default instead of af_sky
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

    // Helper function to get language info from voice style
    fn get_language_info_from_voice_style(voice_style: &str) -> (String, String, String, String) {
        let prefix = voice_style.get(0..2).unwrap_or("");
        match prefix {
            // American English
            "af" | "am" => (
                "en-US".to_string(),
                "English".to_string(),
                "United States".to_string(),
                "🇺🇸".to_string(),
            ),
            // British English
            "bf" | "bm" => (
                "en-GB".to_string(),
                "English".to_string(),
                "United Kingdom".to_string(),
                "🇬🇧".to_string(),
            ),
            // Japanese
            "jf" | "jm" => (
                "ja".to_string(),
                "Japanese".to_string(),
                "Japan".to_string(),
                "🇯🇵".to_string(),
            ),
            // Mandarin Chinese
            "zf" | "zm" => (
                "zh-CN".to_string(),
                "Chinese".to_string(),
                "China".to_string(),
                "🇨🇳".to_string(),
            ),
            // Spanish
            "ef" | "em" => (
                "es".to_string(),
                "Spanish".to_string(),
                "Spain".to_string(),
                "🇪🇸".to_string(),
            ),
            // French
            "ff" | "fm" => (
                "fr".to_string(),
                "French".to_string(),
                "France".to_string(),
                "🇫🇷".to_string(),
            ),
            // Hindi
            "hf" | "hm" => (
                "hi".to_string(),
                "Hindi".to_string(),
                "India".to_string(),
                "🇮🇳".to_string(),
            ),
            // Italian
            "if" | "im" => (
                "it".to_string(),
                "Italian".to_string(),
                "Italy".to_string(),
                "🇮🇹".to_string(),
            ),
            // Brazilian Portuguese
            "pf" | "pm" => (
                "pt-BR".to_string(),
                "Portuguese".to_string(),
                "Brazil".to_string(),
                "🇧🇷".to_string(),
            ),
            // Default to American English for unknown prefixes
            _ => (
                "en-US".to_string(),
                "English".to_string(),
                "United States".to_string(),
                "🇺🇸".to_string(),
            ),
        }
    }

    // Helper function to get a friendly display name from voice style
    fn get_friendly_voice_name(voice_style: &str) -> String {
        let (_, _, country, flag) = Self::get_language_info_from_voice_style(voice_style);
        
        // Extract gender and name from voice style
        let gender_char = voice_style.chars().nth(1).unwrap_or('u');
        let gender = match gender_char {
            'f' => "Female",
            'm' => "Male",
            _ => "Unknown",
        };
        
        let name_part = voice_style.get(3..).unwrap_or(voice_style);
        let formatted_name = name_part
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        
        format!("{} {} - {} {}", flag, formatted_name, gender, country)
    }

    // Get voices grouped by country for better organization
    pub fn get_voices_grouped_by_country() -> BTreeMap<String, Vec<Voice>> {
        let voices = Self::get_kokoros_voice_rows();
        let mut grouped: BTreeMap<String, Vec<Voice>> = BTreeMap::new();
        
        for voice in voices {
            let country = voice.language.region.clone();
            grouped.entry(country).or_insert_with(Vec::new).push(voice);
        }
        
        // Sort voices within each country by name
        for voices_in_country in grouped.values_mut() {
            voices_in_country.sort_by(|a, b| a.name.cmp(&b.name));
        }
        
        grouped
    }

    // Get country order for consistent sorting
    pub fn get_country_sort_order() -> Vec<String> {
        vec![
            "United States".to_string(),
            "United Kingdom".to_string(),
            "Japan".to_string(),
            "China".to_string(),
            "Spain".to_string(),
            "France".to_string(),
            "India".to_string(),
            "Italy".to_string(),
            "Brazil".to_string(),
        ]
    }
}
