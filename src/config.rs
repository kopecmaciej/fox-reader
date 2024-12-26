use dirs::home_dir;

const HF_BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices";
const HF_VERSION: &str = "v1.0.0/";
const HF_VOICES_JSON: &str = "voices.json";
const HF_DOWNLOAD_PATH: &str = "$HOME/.local/share/fox-reader/voices";

const DISPATCHER_CONFIG_PATH: &str = "$HOME/.config/speech-dispatcher";
const DISPATCHER_CONFIG_FILE: &str = "speechd.conf";
const DISPATCHER_MODULE_FILE: &str = "modules/fox-reader.conf";
const DISPATCHER_SCRIPT_FILE: &str = "fox-reader.sh";

fn resolve_home(path: &str) -> String {
    let home = home_dir().expect("Failed to get home directory");
    path.replace("$HOME", &home.to_string_lossy())
}

fn build_path(base_path: &str, relative_path: &str) -> String {
    resolve_home(base_path).to_string() + "/" + relative_path
}

pub mod huggingface_config {
    use super::*;

    pub fn get_voices_url() -> String {
        format!("{}/resolve/{}{}", HF_BASE_URL, HF_VERSION, HF_VOICES_JSON)
    }

    pub fn get_voice_url(path: &str) -> String {
        format!("{}/resolve/main/{}", HF_BASE_URL, path)
    }

    pub fn get_voice_file_path(voice_file: &str) -> String {
        build_path(HF_DOWNLOAD_PATH, voice_file)
    }

    pub fn get_download_path() -> String {
        resolve_home(HF_DOWNLOAD_PATH)
    }
}

pub mod dispatcher_config {
    use super::*;

    pub fn get_config_file_path() -> String {
        build_path(DISPATCHER_CONFIG_PATH, DISPATCHER_CONFIG_FILE)
    }

    pub fn get_module_config_path() -> String {
        build_path(DISPATCHER_CONFIG_PATH, DISPATCHER_MODULE_FILE)
    }

    pub fn get_script_path() -> String {
        build_path(DISPATCHER_CONFIG_PATH, DISPATCHER_SCRIPT_FILE)
    }
}
