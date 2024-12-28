use std::error::Error;

use crate::{
    config::{dispatcher_config, huggingface_config},
    core::file_handler::FileHandler,
};

const FOX_READER_SCRIPT: &[u8] = include_bytes!("../../scripts/fox-piper.sh");

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn initialize_config() -> Result<(), Box<dyn Error>> {
        let config_file = &dispatcher_config::get_config_file_path();
        // TODO: Check if speechd.conf is default or already adjusted
        let vec_bytes = config_template("en-GB").trim().as_bytes().to_vec();
        let config_exists = FileHandler::does_file_exist(config_file);
        if !config_exists {
            FileHandler::ensure_all_dirs_exists(config_file)?;
            FileHandler::save_bytes(config_file, &vec_bytes)?;
        }

        let module_path = &dispatcher_config::get_module_config_path();
        if !FileHandler::does_file_exist(module_path) {
            FileHandler::save_bytes(
                module_path,
                module_template("piper-tts", &huggingface_config::get_download_path())
                    .trim()
                    .as_bytes(),
            )?;
        }
        let script_path = &dispatcher_config::get_script_path();
        if !FileHandler::does_file_exist(script_path) {
            FileHandler::save_bytes(script_path, FOX_READER_SCRIPT)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(script_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o755); // rwxr-xr-x
                std::fs::set_permissions(script_path, perms)?;
            }
        }

        Ok(())
    }

    pub fn add_new_voice(
        language: &str,
        voice_name: &str,
        voice_key: &str,
    ) -> Result<(), Box<dyn Error>> {
        let new_voice = add_voice_template(language, voice_name, voice_key);

        FileHandler::append_to_file(
            &dispatcher_config::get_module_config_path(),
            new_voice.as_bytes(),
        )
    }

    pub fn delete_voice_from_config(
        language: &str,
        voice_name: &str,
        voice_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let voice_template = add_voice_template(language, voice_name, voice_id);

        FileHandler::delete_line_from_config(
            &dispatcher_config::get_module_config_path(),
            &voice_template,
        )
    }

    pub fn set_default_voice(default_voice: &str) -> Result<(), Box<dyn Error>> {
        FileHandler::upsert_value_in_config(
            &dispatcher_config::get_module_config_path(),
            "DefaultVoice",
            default_voice,
        )
    }
}

fn config_template(default_lang: &str) -> String {
    format!(
        r#"
# Fox Reader 
# Speech Dispatcher Configuration
# Please do not modify this file as it can cause issues with application

# Symbol preprocessing files
SymbolsPreproc "char"
SymbolsPreprocFile "gender-neutral.dic"
SymbolsPreprocFile "font-variants.dic"
SymbolsPreprocFile "symbols.dic"
SymbolsPreprocFile "emojis.dic"
SymbolsPreprocFile "orca.dic"
SymbolsPreprocFile "orca-chars.dic"

AddModule "fox-reader" "sd_generic" "fox-reader.conf"

DefaultLanguage "{}"
DefaultModule "fox-reader" "#,
        default_lang
    )
}

fn module_template(module_path: &str, voices_path: &str) -> String {
    format!(
        r#"
GenericExecuteSynth "export XDATA=\'$DATA\'; echo \"$XDATA\" | sed -z 's/\\n/ /g' | {} -q -m {}/\'$VOICE\' -f - | mpv --speed=\'$RATE\' --volume=100 --no-terminal --keep-open=no -"
    "#,
        module_path, voices_path
    )
}

fn add_voice_template(language: &str, voice_name: &str, voice_relative_path: &str) -> String {
    format!(
        r#" 
AddVoice "{}_{}" "male1" "{}"
"#,
        language, voice_name, voice_relative_path
    )
}
