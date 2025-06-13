# 🦊 Fox Reader

Fox Reader is a simple text-to-speech application built in Rust and GTK4 that
converts text to speech using high-quality voices from
[Kokoros TTS](https://github.com/lucasjinreal/Kokoros).

## Current UI:

![AI Chat](assets/ai_chat.png)

![PDF Reader](assets/pdf_reader.png)

![Voice List](assets/voice_list.png)

![Settings](assets/settings.png)

## Why I've built it?

While other applications offer TTS integration with Speech Dispatcher, I
wanted a little bit more, also none of them seem to work with Firefox
`Read aloud` functionality, so I decided to build this small GTK application.

## Key Features

1. **PDF Reader with Highlighting System**
   - Read PDF documents with real-time text highlighting
   - Choose from where to start reading

2. **AI Chat with LLM Integration**
   - Connect to AI models via API keys (OpenAI, etc.)
   - Use local LLM solutions (Ollama/LM Studio)
   - Voice-to-text capability using Whisper models

3. **Text-to-Speech with Highlighting System**
   - Convert any text to natural-sounding speech using Kokoros voices

4. **Speech Dispatcher Compatibility**
   - Seamless integration with Linux accessibility tools
   - Works with system-wide speech services

5. **Firefox Read Aloud Integration**
   - Works directly with Firefox's built-in reading feature
   - Use voices downloaded via app

## Installation

### Prerequisites

- **Operating Systems**: Primarily Linux-based distributions, tested on:
   - Ubuntu 24.04
   - Fedora 42
   - Arch Linux (GNOME & Hyprland)

- **GTK4** ≥ 4.12, Check version with: 
```bash
pkg-config --modversion gtk4
```
- **Adwaita** >= 1.5, check version via 
```bash
pkg-config --modversion libadwaita-1
```
- **Speech Dispatcher** for reading via spd-say or browser (optional), example install on Arch Linux:

```bash
sudo pacman -S speech-dispatcher

```
- **Rust toolchain** (for building from source)
[Install rust](https://www.rust-lang.org/tools/install)

### Environment Notes

- On Ubuntu 24.04, Fedora 42, and Arch Linux with GNOME, no extra dependencies are needed—required GTK4 and Adwaita libraries are already included.

- On Hyprland (Arch Linux), libadwaita is not installed by default. You must install it manually:

```bash
sudo pacman -S libadwaita
```
- Older versions of Ubuntu, Debian, Fedora etc. can have problem with running this as updating gtk4 to newer version can be a bit difficult.

### Install From Release

1. Download the latest release:

```bash
curl -sL $(curl -s https://api.github.com/repos/kopecmaciej/fox-reader/releases/latest \
  | grep browser_download_url \
  | grep .tar.gz \
  | cut -d '"' -f 4) -o fox-reader.tar.gz

tar -xzf fox-reader.tar.gz
## Or other desired path
mv fox-reader ~/.local/bin/
rm fox-reader.tar.gz
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
2. Browse available voices from the Kokoros TTS repository
3. Download desired voices; every downloaded voice is available in `PDF Reader`, `Text Reader`,
   and `Speech Dispatcher`
4. Set favorite voice as default for `Speech Dispatcher` usage without specified model

### Speech Dispatcher Integration

Fox Reader integrates with Speech Dispatcher through the app's CLI mode.
A special script located in `~/.config/speech-dispatcher/fox-reader.sh`
will forward data and options properly. If `Fox Reader` is missing in 
`$PATH` you have to specify location by yourself in the script.

### CLI Usage

Fox Reader can be used via command line interface for quick text-to-speech conversion without launching the GUI.

#### Basic Command Structure

```bash
fox-reader --cli --model <MODEL_PATH> --text <TEXT> [--rate <RATE>] [--output <OUTPUT_PATH>]
```

#### Required Arguments

- `--cli`: Run in CLI mode without launching the GUI
- `--text` or `-t`: Text to synthesize

#### Optional Arguments

- `--voice` or `-v`: Voice name from `Voice List` tab, example: `pm_alex`
- `--speed` or `-s`: Speech rate adjustment (0.5 to 2)
- `--output` or `-o`: Path to save the audio output in WAV format
  - If not specified, audio will play immediately
- `--list-voices` or `-l`: List all available voices

#### Examples

**Play speech immediately:**
```bash
fox-reader --cli --voice pm_alex --text "Hello, this is Fox Reader speaking."
```

**Adjust speech rate:**
```bash
fox-reader --cli --voice pm_alex --text "This is faster speech." --speed 1.5
```

**Save to file instead of playing:**
```bash
fox-reader --cli --voice pm_alex --text "This will be saved to a file." --output ~/output.wav
```

## Configuration

Fox Reader uses GSettings for storing user preferences and configuration options. These settings include:

- UI theme and appearance preferences
- Default speech parameters
- Window size and position
- Selected voice preferences
- AI chat configuration (model selection, api keys, temperature settings)

You can view and modify these settings using the built-in preferences dialog or through the gsettings command-line tool. All other assets like voices, the pdfium library, and whisper models are stored separately in ~/.local/share/fox-reader/.

## Troubleshooting

### Common Issues

1. **Voice download fails**
   - Check your internet connection
   - Ensure you have write permissions to the voices directory

2. **PDF reader doesn't load**
   - Make sure pdfium is installed or let Fox Reader install it automatically
   - Check if the PDF file is not corrupted or password-protected

3. **Speech Dispatcher integration issues**
   - Verify Speech Dispatcher is installed and running
   - Check the configuration in `~/.config/speech-dispatcher/`

## Development Status

### Current Focus

- Improve performance for better user experience on slow PCs
- Enhance the PDF reader with better text extraction
- Optimize voice processing for lower latency

### Future Considerations

- Experiment with higher-quality voice models
- Consider migration to another GUI Rust library for better cross-platform support
- Expand AI chat functionality

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Kokoros TTS](https://github.com/lucasjinreal/Kokoros) for providing high-quality TTS voices
- [GTK4 team](https://www.gtk.org/) for the UI framework
- [Speech Dispatcher project](https://freebsoft.org/speechd)
- [Pdfium-render](https://github.com/ajrcarey/pdfium-render) After using multiple crates that work with PDFs, this one seems the best
- [Whisper-rs](https://github.com/tazz4843/whisper-rs) seamless integration with Whisper models.
