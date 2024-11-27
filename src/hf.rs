use crate::config::HFConfig;
use crate::downloader::Downloader;

pub struct HuggingFace {
    config: HFConfig,
}

impl HuggingFace {
    pub fn new() -> Self {
        Self {
            config: HFConfig::new(),
        }
    }

    pub fn get_avaliable_voices(&self) {
        let voices_url = self.config.get_voices_url();

        match Downloader::download_file(voices_url) {
            Ok(res) => {
                let save_path = "voices.json".to_string();
                if let Err(e) = Downloader::save_file(save_path, res) {
                    eprintln!("Failed to save file: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to download file: {}", e),
        }
    }
}
