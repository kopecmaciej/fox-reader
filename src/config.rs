pub struct HFConfig {
    base_url: &'static str,
    version: &'static str,
    voices_json: &'static str,
    download_path: &'static str,
}

impl HFConfig {
    pub fn new() -> Self {
        Self {
            base_url: "https://huggingface.co/rhasspy/piper-voices",
            version: "v1.0.0/",
            voices_json: "voices.json",
            download_path: "downloads",
        }
    }

    pub fn get_voices_url(&self) -> String {
        format!(
            "{}/resolve/{}{}",
            self.base_url, self.version, self.voices_json
        )
    }

    pub fn get_voice_url(&self, path: &str) -> String {
        format!("{}/resolve/main/{}", self.base_url, path)
    }

    pub fn get_save_path(&self, voice_file: &str) -> String {
        format!("{}/{}", self.download_path, voice_file)
    }
}
