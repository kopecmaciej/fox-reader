use crate::config::HFConfig;
use crate::downloader::FileHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::Path;

pub struct VoiceManager {
    config: HFConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct Language {
    code: String,
    region: String,
    name_native: String,
    name_english: String,
    country_english: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Voice {
    pub key: String,
    pub name: String,
    pub language: Language,
    pub quality: String,
    pub files: HashMap<String, File>,
}

impl VoiceManager {
    pub fn new() -> Self {
        Self {
            config: HFConfig::new(),
        }
    }

    pub fn parse_avaliable_voices(&self) -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let raw_json = self.fetch_avaliable_voices()?;
        let value_data: Value = serde_json::from_str(&raw_json)?;
        let voices: BTreeMap<String, Voice> = serde_json::from_value(value_data.clone())?;
        Ok(voices)
    }

    pub fn fetch_avaliable_voices(&self) -> Result<String, Box<dyn Error>> {
        let voices_url = self.config.get_voices_url();

        let voices_file = FileHandler::download_file(voices_url)?;

        Ok(voices_file.text()?)
    }

    pub fn download_voice(
        &self,
        voice_files: &HashMap<String, File>,
    ) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".onnx") {
                let voice_url = self.config.get_voice_url(&file_path);
                let res = FileHandler::download_file(voice_url)?;
                if let Some(file_name) = Path::new(file_path).file_name().and_then(|f| f.to_str()) {
                    FileHandler::save_file(&self.config.get_save_path(file_name), res)?
                } else {
                    return Err(format!("Failed to properly extract file name from path").into());
                }
            }
        }

        Ok(())
    }

    pub fn delete_voice(&self, voice_files: &HashMap<String, File>) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".onnx") {
                if let Some(file_name) = Path::new(file_path).file_name().and_then(|f| f.to_str()) {
                    FileHandler::remove_file(&self.config.get_save_path(file_name))?
                } else {
                    return Err(format!("Failed to properly delete file from path").into());
                }
            }
        }

        Ok(())
    }
}
