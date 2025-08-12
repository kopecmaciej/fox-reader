use clap::{Arg, Command};
use std::error::Error;

use crate::core::voice_manager::VoiceManager;
use crate::utils::audio_player::AudioPlayer;
use crate::utils::espeak_handler::EspeakHandler;
use crate::utils::file_handler::FileHandler;

pub async fn run_cli() -> Result<bool, Box<dyn Error>> {
    if !std::env::args().any(|arg| &arg == "--cli") {
        return Ok(false);
    }

    let matches = Command::new("fox-reader")
        .about("A text-to-speech application with GUI and CLI modes")
        .long_about(
            "Fox Reader can synthesize speech from text using various voice styles. \
                    Run without --cli for GUI mode, or use --cli for command-line operation.",
        )
        .arg(
            Arg::new("cli")
                .long("cli")
                .help("Run in CLI mode without GUI")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("voice")
                .short('v')
                .long("voice")
                .help("Voice style to use for speech synthesis (use --list-voices to see options)")
                .value_name("VOICE_STYLE")
                .default_value("af_heart"),
        )
        .arg(
            Arg::new("text")
                .short('t')
                .long("text")
                .help("Text to synthesize")
                .value_name("TEXT"),
        )
        .arg(
            Arg::new("speed")
                .short('s')
                .long("speed")
                .help("Speech speed (0.5-2.0)")
                .value_name("SPEED")
                .default_value("1.0")
                .value_parser(clap::value_parser!(f32)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Path to save audio output as WAV file (if not specified, plays directly)")
                .value_name("OUTPUT_PATH"),
        )
        .arg(
            Arg::new("list-voices")
                .long("list-voices")
                .help("List all available voice styles")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("list-voices") {
        println!("Available voice styles:");
        let voices = VoiceManager::get_kokoros_voices();
        for voice in voices {
            println!("  {}", voice);
        }
        return Ok(true);
    }

    let text = match matches.get_one::<String>("text") {
        Some(text) => text,
        None => {
            return Err("Error: flag --cli cannot be empty".into());
        }
    };

    if !EspeakHandler::is_espeak_installed() {
        EspeakHandler::download_with_progress_cli().await?;
    }

    EspeakHandler::set_espeak_environment();

    let voice_style = matches.get_one::<String>("voice").unwrap();
    let speed = matches.get_one::<f32>("speed").unwrap();
    let output_path = matches.get_one::<String>("output");

    let available_voices = VoiceManager::get_kokoros_voices();
    if !available_voices.contains(voice_style) {
        let err_msg = format!(
            "Error: Invalid voice style '{}'. Use --list-voices to see available options.",
            voice_style
        );
        return Err(err_msg.into());
    }

    if *speed < 0.5 || *speed > 2.0 {
        let err_msg = "Error: Speed must be between 0.5 and 2.0";
        return Err(err_msg.into());
    }

    VoiceManager::init_kokoros()
        .await
        .map_err(|e| format!("Failed to initialize Kokoros TTS: {}", e))?;

    // Check if we're being called from speech dispatcher (via environment or other indicators)
    let is_speech_dispatcher = std::env::var("MOZ_CRASHREPORTER_DATA_DIRECTORY").is_ok()
        || std::env::var("SPEECHD_PORT").is_ok()
        || std::env::var("SPEECHD_HOST").is_ok();

    if let Some(output_path) = output_path {
        if let Err(e) = FileHandler::ensure_all_paths_exists(output_path) {
            let err_msg = format!("Error: Failed to create output directory: {}", e);
            return Err(err_msg.into());
        }

        if !is_speech_dispatcher {
            println!("Generating and saving speech to file...");
        }
        match VoiceManager::save_kokoros_speech_to_file(text, voice_style, *speed, output_path)
            .await
        {
            Ok(_) => {
                if !is_speech_dispatcher {
                    println!("Successfully saved audio to: {}", output_path);
                }
            }
            Err(e) => {
                let err_msg = format!("Error: Failed to save audio to file: {}", e);
                return Err(err_msg.into());
            }
        }
    } else {
        if !is_speech_dispatcher {
            println!("Generating speech...");
        }
        let audio_buffer = VoiceManager::generate_kokoros_speech(text, voice_style, *speed)
            .await
            .map_err(|e| format!("Failed to generate speech: {}", e))?;

        if !is_speech_dispatcher {
            println!("Playing audio...");
        }
        let player = AudioPlayer::default();
        match player.play_audio(audio_buffer) {
            Ok(_) => {
                if !is_speech_dispatcher {
                    println!("Audio playback completed.");
                }
            }
            Err(e) => {
                let err_msg = format!("Error: Failed to play audio: {}", e);
                return Err(err_msg.into());
            }
        }
    }

    Ok(true)
}
