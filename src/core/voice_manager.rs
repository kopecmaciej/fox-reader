use crate::config::huggingface_config;
use crate::core::file_handler::FileHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

pub struct VoiceManager {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Language {
    pub code: String,
    pub name_english: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Voice {
    pub name: String,
    pub key: String,
    pub language: Language,
    pub quality: String,
    #[serde(default)]
    pub downloaded: bool,
    pub files: HashMap<String, File>,
}

impl VoiceManager {
    pub async fn list_all_available_voices() -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let voices_url = huggingface_config::get_voices_url();
        let voices_file = FileHandler::fetch_file(voices_url).await?;
        let raw_json = String::from_utf8(voices_file)?;

        let value_data: Value = serde_json::from_str(&raw_json)?;
        let voices: BTreeMap<String, Voice> = serde_json::from_value(value_data)?;

        let downloaded_voices = Self::list_downloaded_voices()?;

        let voices = voices
            .into_iter()
            .map(|(key, mut voice)| {
                voice.language.code = voice.language.code.replace("_", "-");
                // we want key to be as voice.onnx for dispatcher config
                voice.key = format!("{}.onnx", voice.key);
                // Mark as downloaded if in the list of downloaded voices
                voice.downloaded = downloaded_voices.contains(&voice.key);

                (key, voice)
            })
            .collect();

        Ok(voices)
    }

    pub fn list_downloaded_voices() -> Result<Vec<String>, Box<dyn Error>> {
        let downloaded_voices =
            FileHandler::get_all_file_names(&huggingface_config::get_download_path())?;
        let downloaded_voices: Vec<String> =
            downloaded_voices.iter().map(|f| f.to_string()).collect();

        Ok(downloaded_voices)
    }

    pub async fn download_voice(voice_files: Vec<String>) -> Result<(), Box<dyn Error>> {
        for file_path in voice_files {
            let voice_config_url = huggingface_config::get_voice_url(&file_path);
            let file = FileHandler::fetch_file(voice_config_url).await?;
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or("Failed to properly extract file name from path")?;

            FileHandler::save_bytes(&huggingface_config::get_voice_file_path(file_name), &file)?;
        }
        Ok(())
    }

    pub fn delete_voice(voice_files: Vec<String>) -> Result<(), Box<dyn Error>> {
        for file_path in voice_files {
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or("Failed to properly extract file name from path")?;

            FileHandler::remove_file(&huggingface_config::get_voice_file_path(file_name))?;
        }

        Ok(())
    }
}
