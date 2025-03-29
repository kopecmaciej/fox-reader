# Fox Reader

Fox Reader is a simple text-to-speech application built in Rust and GTK4 that
converts text to speech using all voices from
[piper voices](https://huggingface.co/rhasspy/piper-voices).

## Why I've built it?

While other applications offer Piper voice integration with Speech Dispatcher, I
wanted a little bit more, also non of them seems to work with firefox
`Read aloud` functionallty, so I decided to build this small GTK application.

## Key Features

1. PdfReader with highlighting system
2. Text-to-speech with highlighting system
3. AI Chat with LLM's via api key or locally (Ollama/LM Studio)
4. Speech Dispatcher compatibility
5. Firefox Read Aloud integration

## Current UI:

![Text to Speech Interface](assets/test_to_speach.png)

![Voice List](assets/voice_list.png)

## Prerequisites
- GTK4 and its development libraries
- Rust toolchain (building from source)
- Speech Dispatcher for reading via spd-say or browser (optional)
- Pdfium for Pdf Reader (optional - if missing will be installed)
- Whisper model for AI Chat (optional - if missing will be installed)

## Installation

### From Release

1. Download the latest release:

```bash
wget https://github.com/kopecmaciej/fox-reader/releases/download/v0.1.0/fox-reader-v0.1.0.tar.gz
tar -xzf fox-reader-v0.1.0.tar.gz
mv fox-reader ~/.local/bin/
rm fox-reader-v0.1.0.tar.gz
```

2. Run the application:

```bash
fox-reader
```

### Building from Source

1. Clone the repository:

```bash
# Clone repository
git clone https://github.com/kopecmaciej/fox-reader.git
cd fox-reader

# Build
cargo build --release

# Run
./target/release/fox-reader
```

## Usage

### Voice Management

1. Open the `Voice List` tab
2. Browse available voices from the Piper voices repository
3. Download desired voices, every downloaded voice is avaliable in `Text Reader`
   and `Speech Dispatcher`
4. Set favorite voice as default for better `Speech Dispatcher` experience

### Speech Dispatcher Integration

Fox Reader integrates with Speech Dispatcher through a custom output module
script that processes audio using various players (mpv, ffplay, sox with aplay
or paplay). The script handles:

- Dynamic audio player selection based on system availability
- Speech rate adjustments for Firefox compatibility
- Volume control and audio processing
- Raw audio stream handling from Piper TTS

### Cli

## Development Status

- Improve performance for better user experience on slow PC's
- (To consider) Experiment with better-quality voices
- (To consider) Move to other GUI rust library for better support on Macos/Windows (probably
  as separate project)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Piper Voices](https://huggingface.co/rhasspy/piper-voices) for providing the
  TTS voices
- [GTK4 team for the UI framework](https://www.gtk.org/)
- [Speech Dispatcher project](https://freebsoft.org/speechd)
- [Pdfium-render](https://github.com/ajrcarey/pdfium-render) After using multiple crates that works with PDF's this one seems the best
- [Piper-rs](https://github.com/thewh1teagle/piper-rs) for simple integration with Piper voices
- [Whisper-rs](https://github.com/tazz4843/whisper-rs) seamless integration with Whisper models.










