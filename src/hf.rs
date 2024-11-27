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

    fn get_avaliable_voices(&self) {
        let voices_url = self.config.get_voices_url();

        let res = Downloader::download_file(voices_url);
    }
}
