const HF_BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices";
const HF_VERSION: &str = "v1.0.0/";
const HF_VOICES_JSON: &str = "voices.json";
const HF_DOWNLOAD_PATH: &str = "downloads";

const DISPATCHER_CONFIG_PATH: &str = "/home/cieju/.config/speech-dispatcher";
const DISPATCHER_CONFIG_FILE: &str = "speechd.conf";
const DISPATCHER_MODULE_FILE: &str = "modules/piper-reader.conf";

pub mod huggingface_config {
    use super::*;

    pub fn get_voices_url() -> String {
        format!("{}/resolve/{}{}", HF_BASE_URL, HF_VERSION, HF_VOICES_JSON)
    }

    pub fn get_voice_url(path: &str) -> String {
        format!("{}/resolve/main/{}", HF_BASE_URL, path)
    }

    pub fn get_voice_file_path(voice_file: &str) -> String {
        format!("{}/{}", HF_DOWNLOAD_PATH, voice_file)
    }

    pub fn get_download_path() -> String {
        HF_DOWNLOAD_PATH.to_string()
    }
}

pub mod dispatcher_config {
    use super::*;

    pub fn get_config_file_path() -> String {
        format!("{}/{}", DISPATCHER_CONFIG_PATH, DISPATCHER_CONFIG_FILE)
    }

    pub fn get_module_config_path() -> String {
        format!("{}/{}", DISPATCHER_CONFIG_PATH, DISPATCHER_MODULE_FILE)
    }

    pub fn get_dispatcher_config_path() -> String {
        DISPATCHER_CONFIG_PATH.to_string()
    }
}
