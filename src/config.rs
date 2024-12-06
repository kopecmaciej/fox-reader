pub struct Config {
    pub hf: HuggingFace,
    pub dispatcher: Dispatcher,
}

pub struct Dispatcher {
    pub config_path: &'static str,
    pub config_file: &'static str,
    pub module_file: &'static str,
}

pub struct HuggingFace {
    base_url: &'static str,
    version: &'static str,
    voices_json: &'static str,
    pub download_path: &'static str,
}

impl Config {
    pub fn new() -> Self {
        let hf = HuggingFace {
            base_url: "https://huggingface.co/rhasspy/piper-voices",
            version: "v1.0.0/",
            voices_json: "voices.json",
            download_path: "downloads",
        };
        let dispatcher = Dispatcher {
            config_path: "/home/cieju/.config/speech-dispatcher",
            config_file: "speechd.conf",
            module_file: "modules/piper-reader.conf",
        };
        Self { hf, dispatcher }
    }

    pub fn get_voices_url(&self) -> String {
        format!(
            "{}/resolve/{}{}",
            self.hf.base_url, self.hf.version, self.hf.voices_json
        )
    }

    pub fn get_voice_url(&self, path: &str) -> String {
        format!("{}/resolve/main/{}", self.hf.base_url, path)
    }

    pub fn get_voice_file_path(&self, voice_file: &str) -> String {
        format!("{}/{}", self.hf.download_path, voice_file)
    }

    pub fn get_config_file_path(&self) -> String {
        format!(
            "{}/{}",
            self.dispatcher.config_path, self.dispatcher.config_file
        )
    }

    pub fn get_module_config_path(&self) -> String {
        format!(
            "{}/{}",
            self.dispatcher.config_path, self.dispatcher.module_file
        )
    }
}
