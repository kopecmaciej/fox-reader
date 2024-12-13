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

#[derive(Debug, Serialize, Deserialize)]
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
    pub fn list_all_avaliable_voices(
    ) -> Result<BTreeMap<String, Rc<RefCell<Voice>>>, Box<dyn Error>> {
        let voices_url = huggingface_config::get_voices_url();
        let voices_file = FileHandler::download_file(voices_url)?;
        let raw_json = voices_file.text()?;

        let value_data: Value = serde_json::from_str(&raw_json)?;
        let mut voices: BTreeMap<String, Voice> = serde_json::from_value(value_data)?;

        voices.iter_mut().for_each(|(_, voice)| {
            // we want language in format of en-GB not en_GB
            voice.language.code = voice.language.code.replace("_", "-");
            // we want key to be as voice.onnx for dispatcher config
            voice.key = format!("{}.onnx", voice.key);
        });

        let downloaded_voices = Self::list_downloaded_voices()?;
        downloaded_voices.iter().for_each(|f| {
            if let Some(voice) = voices.get_mut(f) {
                voice.downloaded = true;
            }
        });

        let voices = voices
            .into_iter()
            .map(|(key, voice)| {
                let voice = Rc::new(RefCell::new(voice));
                (key, voice)
            })
            .collect();

        Ok(voices)
    }

    pub fn list_downloaded_voices() -> Result<Vec<String>, Box<dyn Error>> {
        let downloaded_voices =
            FileHandler::get_all_file_names(&huggingface_config::get_download_path())?;
        let downloaded_voices: Vec<String> = downloaded_voices
            .iter()
            .map(|f| f.split(".").next().unwrap_or(f).to_string())
            .collect();

        Ok(downloaded_voices)
    }

    pub fn download_voice(voice_files: &HashMap<String, File>) -> Result<(), Box<dyn Error>> {
        for (file_path, _) in voice_files {
            if file_path.ends_with(".json") {
                // Download the voice json config
                let voice_config_url = huggingface_config::get_voice_url(&file_path);
                let mut res = FileHandler::download_file(voice_config_url)?;
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                FileHandler::save_file(
                    &huggingface_config::get_voice_file_path(file_name),
                    &mut res,
                )?;
            }
            if file_path.ends_with(".onnx") {
                // Download the voice file
                let voice_url = huggingface_config::get_voice_url(&file_path);
                let mut res = FileHandler::download_file(voice_url)?;
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or("Failed to properly extract file name from path")?;

                FileHandler::save_file(
                    &huggingface_config::get_voice_file_path(file_name),
                    &mut res,
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
