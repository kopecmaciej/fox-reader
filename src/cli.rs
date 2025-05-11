use clap::{Arg, Command};
use std::error::Error;
use std::path::Path;

use crate::core::piper::PiperTTS;
use crate::utils::audio_player::AudioPlayer;
use crate::utils::espeak_handler::EspeakHandler;
use crate::utils::file_handler::FileHandler;

pub async fn run_cli() -> Result<bool, Box<dyn Error>> {
    if !std::env::args().any(|arg| &arg == "--cli") {
        return Ok(false);
    }

    let matches = Command::new("fox-reader")
        .about("A text-to-speech application")
        .arg(
            Arg::new("cli")
                .long("cli")
                .help("Run in CLI mode without GUI")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("model")
                .short('m')
                .long("model")
                .help("Path to the Piper model directory (.onnx.json file)")
                .value_name("MODEL_PATH")
                .required(true),
        )
        .arg(
            Arg::new("text")
                .short('t')
                .long("text")
                .help("Text to synthesize")
                .value_name("TEXT")
                .required(true),
        )
        .arg(
            Arg::new("rate")
                .short('r')
                .long("rate")
                .help("Speech rate (-100:100)")
                .value_name("RATE")
                .default_value("0")
                .allow_negative_numbers(true)
                .value_parser(clap::value_parser!(f32)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Path to save audio output (WAV format)")
                .value_name("OUTPUT_PATH"),
        )
        .get_matches();

    let model_path = matches.get_one::<String>("model").unwrap();
    let text = matches.get_one::<String>("text").unwrap();
    let rate = matches.get_one::<f32>("rate").unwrap();
    let output_path = matches.get_one::<String>("output");

    let calculated_rate = speech_dispatcher_to_piper_percentage(rate);

    if !Path::new(model_path).exists() {
        let err_msg = format!("Error: Model path does not exist: {}", model_path);
        return Err(err_msg.into());
    }
    EspeakHandler::download_with_progress_cli().await?;

    let piper = PiperTTS::new();
    match piper.initialize(model_path).await {
        Ok(_) => {}
        Err(e) => {
            let err_msg = "Error: Failed to initialize Piper TTS engine";
            let err_msg = format!("{}\nModel path: {}", err_msg, model_path);
            let err_msg = format!("{}\nDetails: {}", err_msg, e);
            return Err(err_msg.into());
        }
    }

    if let Some(output_path) = output_path {
        if let Err(e) = FileHandler::ensure_all_paths_exists(output_path) {
            let err_msg = format!("Error: Failed to create output directory: {}", e);
            return Err(err_msg.into());
        }

        match piper
            .synthesize_speech_to_wav(text, output_path, Some(calculated_rate))
            .await
        {
            Ok(_) => {
                println!("Successfully saved audio to: {}", output_path);
            }
            Err(e) => {
                let err_msg = "Error: Failed to save audio to file";
                let err_msg = format!("{}\nOutput path: {}", err_msg, output_path);
                let err_msg = format!("{}\nDetails: {}", err_msg, e);
                return Err(err_msg.into());
            }
        }
    } else {
        let audio_buffer = match piper.synthesize_speech(text, Some(calculated_rate)).await {
            Ok(buffer) => buffer,
            Err(e) => {
                let err_msg = "Error: Failed to synthesize speech";
                let err_msg = format!("{}\nDetails: {}", err_msg, e);
                return Err(err_msg.into());
            }
        };

        let player = AudioPlayer::default();
        match player.play_audio(audio_buffer) {
            Ok(_) => {}
            Err(e) => {
                let err_msg = "Error: Failed to play audio";
                let err_msg = format!("{}\nDetails: {}", err_msg, e);
                return Err(err_msg.into());
            }
        }
    }

    Ok(true)
}

fn speech_dispatcher_to_piper_percentage(sd_rate: &f32) -> u8 {
    if *sd_rate < 0.0 {
        (sd_rate + 100.0 / 10.0).round() as u8
    } else {
        (10.0 + (sd_rate * 0.4)).round() as u8
    }
}
