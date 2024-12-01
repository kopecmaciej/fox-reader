use crate::config::HFConfig;
use crate::downloader::Downloader;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

pub struct HuggingFace {
    config: HFConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct File {
    size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Voice {
    key: String,
    name: String,
    quality: String,
    files: HashMap<String, File>,
}

impl HuggingFace {
    pub fn new() -> Self {
        Self {
            config: HFConfig::new(),
        }
    }

    pub fn get_avaliable_voices(&self) -> Result<String, Box<dyn Error>> {
        let voices_url = self.config.get_voices_url();

        let voices_file = Downloader::download_file(voices_url)?;

        Ok(voices_file.text()?)
    }

    pub fn parse_avaliable_voices(&self) -> Result<Vec<Voice>, Box<dyn Error>> {
        let raw_json = self.get_avaliable_voices()?;
        let value_data: Value = serde_json::from_str(&raw_json)?;
        let voice_map: HashMap<String, Voice> = serde_json::from_value(value_data.clone())?;
        let voices: Vec<Voice> = voice_map.into_iter().map(|(_, v)| v).collect();
        Ok(voices)
    }
}
