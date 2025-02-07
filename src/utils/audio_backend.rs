use std::error::Error;
use which::which;

#[derive(Debug)]
pub struct AudioBackend {
    command: String,
}

impl AudioBackend {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        for cmd in ["ffplay", "sox", "aplay", "paplay", "mpv"] {
            if which(cmd).is_ok() {
                return Ok(Self::create_backend(cmd));
            }
        }
        Err("No supported audio processor found. Please install one of: mpv, aplay, paplay, sox, or ffplay".into())
    }

    fn create_backend(processor: &str) -> Self {
        let command = match processor {
            "mpv" => r#"mpv --speed=${RATE} --volume=$${VOLUME} --no-terminal --keep-open=no -"#,
            "aplay" => r#"aplay -q -t raw -c 1 -r $${SAMPLE_RATE} -f S16_LE -"#,
            "paplay" => {
                r#"paplay --volume=$(awk "BEGIN {printf \"%.0f\", $${VOLUME} * 655.35}") --channels=1 --rate=22050 --raw -"#
            }
            "sox" => {
                r#"sox -q -r 22050 -b 16 -c 1 -e signed-integer -t raw - -t wav - tempo $${RATE} norm vol $(awk "BEGIN {printf \"%.2f\", $${VOLUME} / 100}") | paplay --format=s16le --rate=22050 --channels=1"#
            }
            "ffplay" => r#"ffplay -nodisp -autoexit -volume $${VOLUME} -"#,
            _ => unreachable!(),
        }.to_string();

        Self { command }
    }

    pub fn get_command(&self) -> &str {
        &self.command
    }
}
