use std::error::Error;
use std::fs;

use crate::{config::DispatcherConfig, downloader::FileHandler};

pub struct SpeechDispatcher {}

impl SpeechDispatcher {
    pub fn initialize_config() -> Result<(), Box<dyn Error>> {
        Self::create_config_dir()?;

        FileHandler::save_file(
            &DispatcherConfig::get_config_file_path(),
            &mut config_template().trim().as_bytes(),
        )?;

        FileHandler::save_file(
            &DispatcherConfig::get_module_config_path(),
            &mut module_template("piper-tts", "downloads/de_DE-karlsson-low.onnx")
                .trim()
                .as_bytes(),
        )?;

        Ok(())
    }

    fn create_config_dir() -> Result<(), std::io::Error> {
        let path = &DispatcherConfig::get_dispatcher_config_path();
        let exist = fs::metadata(&path).is_ok();
        if !exist {
            fs::create_dir(path)
        } else {
            Ok(())
        }
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
DefaultVoiceType  "male1"
DefaultModule "piper-reader" "#
    )
}

fn module_template(piper_path: &str, module_path: &str) -> String {
    format!(
        r#"
GenericExecuteSynth "export XDATA=\'$DATA\'; echo \"$XDATA\" | sed -z 's/\\n/ /g' | \"{}\" -q -m \"{}\" -s 21 -f - | mpv --volume=100 --no-terminal --keep-open=no -"

AddVoice "en-GB" "male1" "en_GB-northern_english_male-medium"
    "#,
        piper_path, module_path
    )
}
