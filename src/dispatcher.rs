use std::error::Error;
use std::fs;

use crate::{config::dispatcher_config, file_handler::FileHandler};

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn set_default_voice(default_voice: &str) -> Result<(), Box<dyn Error>> {
        FileHandler::upsert_value_in_config(
            &dispatcher_config::get_module_config_path(),
            "DefaultVoiceType",
            default_voice,
        )
    }

    pub fn initialize_config() -> Result<(), Box<dyn Error>> {
        Self::create_config_dir()?;

        FileHandler::save_file(
            &dispatcher_config::get_config_file_path(),
            &mut config_template("en-GB").trim().as_bytes(),
        )?;
        Self::initialize_module_config()
    }

    pub fn add_new_voice(
        language: &str,
        voice_name: &str,
        voice_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let new_voice = add_voice_template(language, voice_name, voice_id);
        println!("{}", new_voice);

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
            &mut module_template("piper-tts", "downloads").trim().as_bytes(),
        )
    }

    fn create_config_dir() -> Result<(), std::io::Error> {
        let path = &dispatcher_config::get_dispatcher_config_path();
        let exist = fs::metadata(&path).is_ok();
        if !exist {
            fs::create_dir(path)
        } else {
            Ok(())
        }
    }
}

fn config_template(default_lang: &str) -> String {
    format!(
        r#"
###
### Custom Speech Dispatcher Configuration
### Please do not modify this file as it can cause issues with application
###

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
DefaultVoiceType  ""
DefaultModule "piper-reader" "#,
        default_lang
    )
}

fn module_template(piper_path: &str, module_path: &str) -> String {
    let module_path_with_voice = format!("{}/${{VOICE}}", module_path);
    format!(
        r#"
GenericExecuteSynth "export XDATA=\'$DATA\'; echo \"$XDATA\" | sed -z 's/\\n/ /g' | \"{}\" -q -m \"{}\" -s 21 -f - | mpv --volume=100 --no-terminal --keep-open=no -"
    "#,
        piper_path, module_path_with_voice
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
