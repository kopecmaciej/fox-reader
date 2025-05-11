use flate2::read::GzDecoder;
use reqwest;
use std::env;
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::Path;
use tar::Archive;

use crate::paths;
use crate::utils::progress_tracker::ProgressTracker;

use super::file_handler::FileHandler;
use super::progress_tracker::ProgressCallback;

pub struct EspeakHandler {}

impl EspeakHandler {
    pub fn get_espeak_data_path() -> String {
        paths::get_espeak_path()
    }

    pub fn get_espeak_parent_dir() -> String {
        let espeak_data_path = Self::get_espeak_data_path();
        Path::new(&espeak_data_path)
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    pub fn is_espeak_installed() -> bool {
        let espeak_data_path = Self::get_espeak_data_path();
        FileHandler::does_file_exist(&espeak_data_path) && Path::new(&espeak_data_path).is_dir()
    }

    pub async fn download_espeak_data(
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), Box<dyn Error>> {
        let espeak_parent_dir = Self::get_espeak_parent_dir();

        create_dir_all(&espeak_parent_dir)?;

        let espeak_download_url = "https://github.com/thewh1teagle/piper-rs/releases/download/espeak-ng-files/espeak-ng-data.tar.gz";

        let temp_path = Path::new("/tmp/espeak-ng-temp");
        if temp_path.exists() {
            fs::remove_dir_all(temp_path)?;
        }
        create_dir_all(temp_path)?;

        let compressed_data = if let Some(progress_cb) = progress_callback {
            FileHandler::fetch_file_async_with_progress(
                espeak_download_url.to_string(),
                Some(progress_cb),
            )
            .await?
        } else {
            reqwest::get(espeak_download_url)
                .await?
                .bytes()
                .await?
                .to_vec()
        };

        let gz = GzDecoder::new(&compressed_data[..]);
        let mut archive = Archive::new(gz);
        archive.unpack(&espeak_parent_dir)?;

        if !Path::new(&Self::get_espeak_data_path()).exists() {
            return Err("Failed to extract espeak-ng-data properly".into());
        }

        Ok(())
    }

    pub fn set_espeak_environment() {
        let espeak_data_path = Self::get_espeak_data_path();
        env::set_var(
            "PIPER_ESPEAKNG_DATA_DIRECTORY",
            Path::new(&espeak_data_path).parent().unwrap(),
        );
    }

    pub async fn download_with_progress_cli() -> Result<(), Box<dyn Error>> {
        if !EspeakHandler::is_espeak_installed() {
            println!("Downloading espeak-ng data files required for TTS...");

            let progress_tracker = ProgressTracker::default();
            let callback = progress_tracker.get_terminal_progress_callback();

            EspeakHandler::download_espeak_data(Some(callback)).await?;

            println!("Espeak data downloaded successfully.");
        }

        EspeakHandler::set_espeak_environment();

        Ok(())
    }

    pub async fn download_with_progress_ui() -> Result<(), Box<dyn Error>> {
        if !EspeakHandler::is_espeak_installed() {}
        EspeakHandler::set_espeak_environment();
        Ok(())
    }
}
