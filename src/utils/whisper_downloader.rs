use std::error::Error;
use std::fs::{self};
use std::path::Path;

use crate::paths::whisper_config::{self};

use super::{file_handler::FileHandler, progress_tracker::ProgressCallback};

pub fn is_model_downloaded(model_name: &str) -> bool {
    let model_path = whisper_config::get_model_path(model_name);
    Path::new(&model_path).exists()
}

pub fn get_downloaded_models() -> Vec<String> {
    let models_dir = whisper_config::get_whisper_models_path();
    let models_path = Path::new(&models_dir);

    if !models_path.exists() {
        return Vec::new();
    }

    let mut downloaded_models = Vec::new();

    if let Ok(entries) = fs::read_dir(models_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.starts_with("ggml-") && file_name.ends_with(".bin") {
                            if let Some(model_name) = file_name
                                .strip_prefix("ggml-")
                                .and_then(|s| s.strip_suffix(".bin"))
                            {
                                downloaded_models.push(model_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    downloaded_models
}

pub async fn download_model(
    model_name: &str,
    progress_callback: Option<ProgressCallback>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = whisper_config::get_model_url(model_name);
    let path = whisper_config::get_model_path(model_name);

    match FileHandler::fetch_file_async_with_progress(url, progress_callback).await {
        Ok(file) => match FileHandler::save_bytes(&path, &file) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("Error while saving whisper model: {}", e).into()),
        },
        Err(e) => Err(format!("Error while downloading whisper model: {}", e).into()),
    }
}

pub fn remove_model(model_name: &str) -> Result<(), Box<dyn Error>> {
    let path = whisper_config::get_model_path(model_name);
    FileHandler::remove_file(&path)
}
