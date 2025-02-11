# Fox Reader

Fox Reader is a powerful text-to-speech application built in Rust and GTK4 that
converts text to speech using all voices from
[piper voices](https://huggingface.co/rhasspy/piper-voices).

## Why I've built it?

While other applications offer Piper voice integration with Speech Dispatcher, I
wanted a little bit more, also non of them seems to work with firefox
`Read aloud` functionallty, so I decided to build this small GTK application.

## Key Features

Easy voice installation and management Firefox Read Aloud integration Adjustable
speech parameters Speech Dispatcher compatibility

## Development Status

- Audio processor improvements for better voice quality at higher speeds
- PDF file support (experimental)
- Move to other GUI rust library for better support on Macos/Windows (probably
  as separate project)

## Current UI:

![Text to Speech Interface](assets/test_to_speach.png)

![Voice List](assets/voice_list.png)

## Prerequisites

- GTK4 and its development libraries
- Speech Dispatcher (optional)
- Rust toolchain (for building from source)

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

1. Open the voice management interface
2. Browse available voices from the Piper voices repository
3. Install desired voices for use with the application
4. Set as default given voice to be used in speech-dispatcher

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Piper Voices](https://huggingface.co/rhasspy/piper-voices) for providing the
  TTS voices
- [GTK4 team for the UI framework](https://www.gtk.org/)
- [Speech Dispatcher project](https://freebsoft.org/speechd)
