use std::fs;

use crate::config::Config;

struct SpeechDispatcher { 
    config: Config
}

impl SpeechDispatcher {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
        }
    }

    pub fn create_config(&self) -> Result<(), std::io::Error> {
        let path = self.config.get_config_path();
        let exist = fs::metadata(&path).is_ok();
        if !exist {
            fs::create_dir(path)
        } else {
            Ok(())
        }
    }
}
