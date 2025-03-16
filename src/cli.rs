use clap::{Arg, Command};
use std::error::Error;

use crate::core::piper::PiperTTS;
use crate::utils::audio_player::AudioPlayer;

pub async fn run_cli() -> Result<bool, Box<dyn Error>> {
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
                .help("Path to the Piper model directory")
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
                .help("Speech rate (5-55)")
                .value_name("RATE")
                .value_parser(clap::value_parser!(u8)),
        )
        .get_matches();

    if !matches.get_flag("cli") {
        return Ok(false);
    }

    let model_path = matches.get_one::<String>("model").unwrap();
    let text = matches.get_one::<String>("text").unwrap();
    let rate = matches.get_one::<u8>("rate").copied();

    let piper = PiperTTS::new();
    piper.initialize(model_path).await?;

    let audio_buffer = piper.synthesize_speech(text, rate).await?;

    let player = AudioPlayer::default();
    player
        .play_audio(audio_buffer)
        .expect("Error while playing audio");

    Ok(true)
}
