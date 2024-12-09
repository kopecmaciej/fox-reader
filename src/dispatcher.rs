use std::error::Error;
use std::fs;

use crate::{config::dispatcher_config, file_handler::FileHandler};

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn set_default_voice(default_voice: &str) -> Result<(), Box<dyn Error>> {
        Self::update_module_config(&set_default_voice_template(default_voice))
    }
    pub fn initialize_config() -> Result<(), Box<dyn Error>> {
        Self::create_config_dir()?;

        FileHandler::save_file(
            &dispatcher_config::get_config_file_path(),
            &mut config_template().trim().as_bytes(),
        )?;

        FileHandler::save_file(
            &dispatcher_config::get_module_config_path(),
            &mut module_template("piper-tts", "downloads/de_DE-karlsson-low.onnx")
                .trim()
                .as_bytes(),
        )?;

        Ok(())
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

    fn update_module_config(config_line: &str) -> Result<(), Box<dyn Error>> {
        let module_config_path = dispatcher_config::get_module_config_path();
        let mut file_content = String::new();

        if fs::metadata(&module_config_path).is_ok() {
            file_content = fs::read_to_string(&module_config_path)?;
        }

        if !file_content.contains(config_line) {
            FileHandler::append_to_file(&module_config_path, &mut config_line.trim().as_bytes())?;
        } else {
            let lines: Vec<&str> = file_content.lines().collect();
            let updated_lines: Vec<String> = lines
                .iter()
                .map(|&line| {
                    if line.starts_with("DefaultVoiceType") {
                        config_line.trim().to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect();

            FileHandler::save_file(
                &module_config_path,
                &mut updated_lines.join("\n").as_bytes(),
            )?;
        }

        Ok(())
    }
}

fn config_template() -> String {
    format!(
        r#"
###
### Custom Speech Dispatcher Configuration
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

DefaultLanguage "en-GB"
DefaultVoiceType  ""
DefaultModule "piper-reader" "#
    )
}

fn module_template(piper_path: &str, module_path: &str) -> String {
    format!(
        r#"
GenericExecuteSynth "export XDATA=\'$DATA\'; echo \"$XDATA\" | sed -z 's/\\n/ /g' | \"{}\" -q -m \"{}\" -s 21 -f - | mpv --volume=100 --no-terminal --keep-open=no -"

    "#,
        piper_path, module_path
    )
}

fn add_voice(language: &str, voice_name: &str, voice_id: &str) -> String {
    format!(
        r#" AddVoice "{}" "{}" "{}"
"#,
        language, voice_name, voice_id
    )
}

fn set_default_voice_template(voice: &str) -> String {
    format!(
        r#" DefaultVoiceType "{}"
"#,
        voice
    )
}
