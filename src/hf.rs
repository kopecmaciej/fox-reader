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
    pub language: Language,
    pub quality: String,
    #[serde(default)]
    pub downloaded: bool,
    pub files: HashMap<String, File>,
}

impl VoiceManager {
    pub fn new() -> Self {
        Self {
            config: HFConfig::new(),
        }
    }

    pub fn list_all_avaliable_voices(&self) -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let voices_url = self.config.get_voices_url();
        let voices_file = FileHandler::download_file(voices_url)?;
        let raw_json = voices_file.text()?;

        let value_data: Value = serde_json::from_str(&raw_json)?;
        let mut voices: BTreeMap<String, Voice> = serde_json::from_value(value_data.clone())?;

        let downloaded_voices = self.list_downloaded_voices()?;
        downloaded_voices.iter().for_each(|f| {
            if let Some(voice) = voices.get_mut(f) {
                voice.downloaded = true;
            }
        });

        Ok(voices)
    }

    pub fn list_downloaded_voices(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let downloaded_voices = FileHandler::get_all_files(self.config.download_path)?;
        let downloaded_voices: Vec<String> = downloaded_voices
            .iter()
            .map(|f| f.split(".").next().unwrap_or(f).to_string())
            .collect();

        Ok(downloaded_voices)
    }

    pub fn download_voice(
        &self,
        voice_files: &HashMap<String, File>,
    ) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".onnx") {
                let voice_url = self.config.get_voice_url(&file_path);
                let res = FileHandler::download_file(voice_url)?;
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                FileHandler::save_file(&self.config.get_voice_file_path(file_name), res)?
            }
        }

        Ok(())
    }

    pub fn delete_voice(&self, voice_files: &HashMap<String, File>) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".onnx") {
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                FileHandler::remove_file(&self.config.get_voice_file_path(file_name))?
            }
        }

        Ok(())
    }
}
