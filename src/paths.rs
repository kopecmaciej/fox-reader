use dirs::home_dir;

const FOX_READER_BASE_PATH: &str = "$HOME/.local/share/fox-reader";

const HF_BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices";
const HF_VERSION: &str = "v1.0.0/";
const HF_VOICES_JSON: &str = "voices.json";

const WHISPER_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp";

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

pub fn get_pdfium_path() -> String {
    build_path(FOX_READER_BASE_PATH, "pdfium")
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
        build_path(&format!("{}/voices", FOX_READER_BASE_PATH), voice_file)
    }

    pub fn get_download_path() -> String {
        resolve_home(&format!("{}/voices", FOX_READER_BASE_PATH))
    }
}

pub mod whisper_config {
    use super::*;

    pub fn get_whisper_models_path() -> String {
        resolve_home(&format!("{}/whisper", FOX_READER_BASE_PATH))
    }

    pub fn get_model_path(model_name: &str) -> String {
        let whisper_path = get_whisper_models_path();
        build_path(&whisper_path, &format!("ggml-{}.bin", model_name))
    }

    pub fn get_model_url(model_name: &str) -> String {
        let base_url = format!("{}/resolve/main/ggml", WHISPER_BASE_URL);

        format!("{}-{}.bin", base_url, model_name)
    }

    pub fn get_whisper_models() -> Vec<&'static str> {
        vec![
            "base-q5_1",
            "base-q8_0",
            "base",
            "base.en-q5_1",
            "base.en-q8_0",
            "base.en",
            "large-v1",
            "large-v2-q5_0",
            "large-v2-q8_0",
            "large-v2",
            "large-v3-q5_0",
            "large-v3-turbo-q5_0",
            "large-v3-turbo-q8_0",
            "large-v3-turbo",
            "large-v3",
            "medium-q5_0",
            "medium-q8_0",
            "medium",
            "medium.en-q5_0",
            "medium.en-q8_0",
            "medium.en",
            "small-q5_1",
            "small-q8_0",
            "small",
            "small.en-q5_1",
            "small.en-q8_0",
            "small.en",
            "tiny-q5_1",
            "tiny-q8_0",
            "tiny",
            "tiny.en-q5_1",
            "tiny.en-q8_0",
            "tiny.en",
        ]
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
