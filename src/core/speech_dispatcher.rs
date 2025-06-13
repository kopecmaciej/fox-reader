use std::error::Error;

use crate::{paths::dispatcher_config, utils::file_handler::FileHandler};
use crate::core::voice_manager::VoiceManager;

const FOX_READER_SCRIPT: &[u8] = include_bytes!("../../scripts/fox-piper.sh");

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn init() -> Result<(), Box<dyn Error>> {
        Self::init_speechd_config()?;
        Self::init_module_config()?;
        Self::init_script()?;
        Ok(())
    }

    fn init_speechd_config() -> Result<(), Box<dyn Error>> {
        let config_file = &dispatcher_config::get_config_file_path();
        if !FileHandler::does_file_exist(config_file) {
            FileHandler::ensure_all_paths_exists(config_file)?;
            FileHandler::save_bytes(config_file, config_template("en-GB").trim().as_bytes())?;
        }
        Ok(())
    }

    fn init_module_config() -> Result<(), Box<dyn Error>> {
        let module_path = &dispatcher_config::get_module_config_path();
        if !FileHandler::does_file_exist(module_path) {
            FileHandler::save_bytes(module_path, module_template().trim().as_bytes())?;
        }
        Ok(())
    }

    pub fn init_script() -> Result<(), Box<dyn Error>> {
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

    pub fn set_default_voice(default_voice: &str) -> Result<(), Box<dyn Error>> {
        FileHandler::upsert_value_in_module_config(
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
DefaultModule "fox-reader""#,
        default_lang
    )
}

fn module_template() -> String {
    let voices = VoiceManager::get_kokoros_voice_rows();
    let add_voice_lines: String = voices.iter().map(|voice| {
        let gender = if voice.traits.contains("♂️") { "male1" } else { "female1" };
        format!(r#"AddVoice "{}" "{}" "{}""#, format!("{}_{}", voice.language.code, voice.key), gender, voice.key)
    }).collect::<Vec<_>>().join("\n");
    let default_voice = voices.first().unwrap().key.clone();
    
    format!(
        r#"
GenericExecuteSynth "export DATA='$DATA';export RATE='$RATE';export VOICE='$VOICE';{}"
DefaultVoice "{}"

{}"#,
        dispatcher_config::get_script_path(),
        default_voice,
        add_voice_lines,
    )
}
