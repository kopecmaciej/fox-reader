use crate::config::{dispatcher_config, huggingface_config};
use crate::core::file_handler::FileHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::Path;

use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub struct VoiceManager {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Language {
    pub code: String,
    pub name_english: String,
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
                voice.key = format!("{}.onnx", voice.key);
                key = format!("{}.onnx", key);
                voice
                    .files
                    .retain(|f, _| f.ends_with("json") || f.ends_with("onnx"));

                // Mark as downloaded if in the list of downloaded voices
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

    pub fn download_voice_samples(file_paths: Vec<String>) -> Result<Vec<u8>, Box<dyn Error>> {
        if let Some(file_name) = file_paths.first() {
            let (base_path, _) = file_name.rsplit_once("/").unwrap();
            let sample_path = format!("{}/samples/speaker_0.mp3", base_path);

            let voice_sample_url = huggingface_config::get_voice_url(&sample_path);
            let file = FileHandler::fetch_file(voice_sample_url)?;
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

    pub fn play_audio_data(audio_data: Vec<u8>) -> Result<(), String> {
        let cursor = Cursor::new(audio_data);

        // Create output stream
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to setup audio output: {}", e))?;

        // Create a Sink
        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Decode and append to sink
        let source = Decoder::new(cursor).map_err(|e| format!("Failed to decode audio: {}", e))?;

        sink.append(source);

        // Wait for playback to complete
        sink.sleep_until_end();

        // Keep stream in scope
        drop(stream);

        Ok(())
    }
}
