use crate::paths::voice_config;
use crate::utils::file_handler::FileHandler;
use crate::utils::progress_tracker::{ProgressCallback, ProgressTracker};
use kokoros::tts::koko::InitConfig;
use std::error::Error;
use std::path::Path;

pub struct KokorosDownloader {
    progress_tracker: ProgressTracker,
}

impl KokorosDownloader {
    pub fn new(progress_tracker: ProgressTracker) -> Self {
        Self { progress_tracker }
    }

    pub fn are_files_available() -> bool {
        let model_path = voice_config::get_kokoros_model_path();
        let voices_path = voice_config::get_kokoros_voices_path();

        Path::new(&model_path).exists() && Path::new(&voices_path).exists()
    }

    pub async fn download_required_files(
        &self,
        mut progress_callback: Option<ProgressCallback>,
    ) -> Result<(), Box<dyn Error>> {
        let config = InitConfig::default();
        let model_url = config.model_url;
        let voices_url = config.voices_url;

        let model_path = voice_config::get_kokoros_model_path();
        let voices_path = voice_config::get_kokoros_voices_path();

        if let Some(parent) = Path::new(&model_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.progress_tracker.set_progress(0.0);

        let callback_clone = progress_callback.clone();

        FileHandler::download_file_with_progress(
            &model_url,
            &model_path,
            Some(Box::new(move |progress| {
                let adjusted_progress = progress * 0.5;
                if let Some(ref callback) = callback_clone {
                    callback.lock().unwrap()(adjusted_progress);
                }
            })),
        )
        .await?;

        if let Some(ref mut callback) = progress_callback {
            callback.lock().unwrap()(0.5);
        }

        let callback_clone = progress_callback.clone();

        FileHandler::download_file_with_progress(
            &voices_url,
            &voices_path,
            Some(Box::new(move |progress| {
                let adjusted_progress = 0.5 + (progress * 0.5);
                if let Some(ref callback) = callback_clone {
                    callback.lock().unwrap()(adjusted_progress);
                }
            })),
        )
        .await?;

        if let Some(ref mut callback) = progress_callback {
            callback.lock().unwrap()(1.0);
        }

        self.progress_tracker.set_progress(1.0);
        if let Some(ref mut callback) = progress_callback {
            callback.lock().unwrap()(1.0);
        }

        Ok(())
    }
}
