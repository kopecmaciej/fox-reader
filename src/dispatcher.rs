use std::error::Error;

use crate::{
    config::{dispatcher_config, huggingface_config},
    file_handler::FileHandler,
};

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn set_default_voice(default_voice: &str) -> Result<(), Box<dyn Error>> {
        FileHandler::upsert_value_in_config(
            &dispatcher_config::get_module_config_path(),
            "DefaultVoice",
            default_voice,
        )
    }

    pub fn initialize_config() -> Result<(), Box<dyn Error>> {
        let path = &dispatcher_config::get_dispatcher_config_path();
        FileHandler::ensure_path_exists(path)?;

        FileHandler::save_file(
            &dispatcher_config::get_config_file_path(),
            &mut config_template("en-GB").trim().as_bytes(),
        )?;
        Self::initialize_module_config()
    }

    pub fn add_new_voice(
        language: &str,
        voice_name: &str,
        voice_key: &str,
    ) -> Result<(), Box<dyn Error>> {
        // speechd want's language in format of en-GB not en_GB
        let language = language.replace("_", "-");
        let voice_file = format!("{}.onnx", voice_key);
        let new_voice = add_voice_template(&language, voice_name, &voice_file);

        FileHandler::append_to_file(
            &dispatcher_config::get_module_config_path(),
            new_voice.as_bytes(),
        )
    }

    pub fn remove_voice(
        language: &str,
        voice_name: &str,
        voice_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let voice_template = add_voice_template(language, voice_name, voice_id);

        FileHandler::remove_line_from_config(
            &dispatcher_config::get_module_config_path(),
            &voice_template,
        )
    }

    fn initialize_module_config() -> Result<(), Box<dyn Error>> {
        FileHandler::save_file(
            &dispatcher_config::get_module_config_path(),
            &mut module_template("piper-tts", &huggingface_config::get_download_path())
                .trim()
                .as_bytes(),
        )
    }
}

fn config_template(default_lang: &str) -> String {
    format!(
        r#"
#
# Piper Reader Speech Dispatcher Configuration
# Please do not modify this file as it can cause issues with application
#

# Symbol preprocessing files
SymbolsPreproc "char"
SymbolsPreprocFile "gender-neutral.dic"
SymbolsPreprocFile "font-variants.dic"
SymbolsPreprocFile "symbols.dic"
SymbolsPreprocFile "emojis.dic"
SymbolsPreprocFile "orca.dic"
SymbolsPreprocFile "orca-chars.dic"

AddModule "piper-reader" "sd_generic" "piper-reader.conf"

DefaultLanguage "{}"
DefaultModule "piper-reader" "#,
        default_lang
    )
}

fn module_template(piper_path: &str, voices_path: &str) -> String {
    format!(
        r#"
GenericExecuteSynth "export XDATA=\'$DATA\'; echo \"$XDATA\" | sed -z 's/\\n/ /g' | {} -q -m {}\'$VOICE\' -f - | mpv --speed=\'$RATE\' --volume=100 --no-terminal --keep-open=no -"
    "#,
        piper_path, voices_path
    )
}

fn add_voice_template(language: &str, voice_name: &str, voice_relative_path: &str) -> String {
    format!(
        r#" 
AddVoice "{}" "{}" "{}"
"#,
        language, voice_name, voice_relative_path
    )
}
