use dirs::home_dir;

const FOX_READER_BASE_PATH: &str = "$HOME/.local/share/fox-reader";

const WHISPER_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp";

const SCHEMAS_DIR_PATH: &str = "$HOME/.local/share/glib-2.0/schemas";
const SCHEMA_URL: &str = "https://raw.githubusercontent.com/kopecmaciej/fox-reader/refs/heads/master/resources/com.github.kopecmaciej.Settings.gschema.xml";

const DISPATCHER_CONFIG_PATH: &str = "$HOME/.config/speech-dispatcher";
const DISPATCHER_CONFIG_FILE: &str = "speechd.conf";
const DISPATCHER_MODULE_FILE: &str = "modules/fox-reader.conf";
const DISPATCHER_SCRIPT_FILE: &str = "fox-reader.sh";

pub fn resolve_home(path: &str) -> String {
    let home = home_dir().expect("Failed to get home directory");
    path.replace("$HOME", &home.to_string_lossy())
}

fn build_path(base_path: &str, relative_path: &str) -> String {
    resolve_home(base_path).to_string() + "/" + relative_path
}

pub fn get_pdfium_path() -> String {
    build_path(FOX_READER_BASE_PATH, "pdfium")
}

pub fn get_espeak_path() -> String {
    build_path(FOX_READER_BASE_PATH, "espeak-ng-data")
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

    pub fn get_whisper_models_names() -> Vec<&'static str> {
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

pub mod voice_config {
    use super::*;

    pub fn get_kokoros_model_path() -> String {
        build_path(FOX_READER_BASE_PATH, "kokoros/kokoro-v1.0.onnx")
    }

    pub fn get_kokoros_voices_path() -> String {
        build_path(FOX_READER_BASE_PATH, "kokoros/voices-v1.0.bin")
    }
}

pub mod schema_config {
    use super::*;

    pub fn get_schemas_dir() -> String {
        resolve_home(SCHEMAS_DIR_PATH)
    }

    pub fn get_schema_url() -> String {
        SCHEMA_URL.to_string()
    }
}
