use crate::config::huggingface_config;
use crate::file_handler::FileHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::Path;

use std::cell::RefCell;
use std::rc::Rc;

pub struct VoiceManager {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Language {
    pub code: String,
    region: String,
    name_native: String,
    pub name_english: String,
    country_english: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub async fn list_all_available_voices(
    ) -> Result<BTreeMap<String, Rc<RefCell<Voice>>>, Box<dyn Error>> {
        let voices_url = huggingface_config::get_voices_url();
        let voices_file = FileHandler::download_file(voices_url).await?;
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

                let voice = Rc::new(RefCell::new(voice));
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

    pub async fn download_voice(voice_files: &HashMap<String, File>) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".json") || file_path.ends_with(".onnx") {
                // Download the voice json config
                let voice_config_url = huggingface_config::get_voice_url(&file_path);
                let file = FileHandler::download_file(voice_config_url).await?;
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                FileHandler::save_bytes(
                    &huggingface_config::get_voice_file_path(file_name),
                    &file,
                )?;
            }
        }
        Ok(())
    }

    pub fn delete_voice(voice_files: &HashMap<String, File>) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".onnx") {
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                // Remove the voice file
                FileHandler::remove_file(&huggingface_config::get_voice_file_path(file_name))?;

                // Remove the voice json config
                let config_file_name = format!("{}.json", file_name);
                FileHandler::remove_file(&huggingface_config::get_voice_file_path(
                    &config_file_name,
                ))?;
            }
        }

        Ok(())
    }
}
