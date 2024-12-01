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
struct Voices {
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

    pub fn parse_avaliable_voices(&self) -> Result<Value, Box<dyn Error>> {
        let raw_json = self.get_avaliable_voices()?;
        let json_data: Value = serde_json::from_str(&raw_json)?;
        let flatten_json: HashMap<String, Voices> = serde_json::from_value(json_data.clone())?;

        for voices in flatten_json.values() {
            for (f, _) in voices.files.iter() {
                let voice_url = self.config.get_voice_url(f);
                println!("{}", voice_url);

                let file_name = std::path::Path::new(&f)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| format!("Invalid file name: {}", f))?;

                if let Err(e) = Downloader::download_file(voice_url.clone())
                    .and_then(|res| Downloader::save_file(&file_name, res))
                {
                    return Err(format!("Failed to download  to file: {}", e).into());
                }
            }
        }

        Ok(json_data)
    }
}
