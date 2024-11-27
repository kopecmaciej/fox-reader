pub struct HFConfig {
    base_url: String,
    version: String,
    voices_json: String,
}

impl HFConfig {
    pub fn new() -> Self {
        Self {
            base_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            version: "v1.0.0/".to_string(),
            voices_json: "voices.json".to_string(),
        }
    }

    pub fn get_voices_url(&self) -> String {
        format!(
            "{}/blob/{}{}",
            self.base_url, self.version, self.voices_json
        )
    }
}
