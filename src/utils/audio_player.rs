use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;

use super::audio_processing::wsola_normalized;

pub enum State {
    Idle,
    Paused,
    Playing,
}

pub struct AudioPlayer {
    sink: Arc<Mutex<Option<Arc<Sink>>>>,
    state: Arc<Mutex<State>>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        AudioPlayer {
            sink: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(State::Idle)),
        }
    }
}

impl AudioPlayer {
    pub fn play_mp3(&self, audio_data: Vec<u8>) -> Result<(), String> {
        let cursor = Cursor::new(audio_data);

        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to setup audio output: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        let sink = Arc::new(sink);

        *self.sink.lock().unwrap() = Some(Arc::clone(&sink));

        let source =
            rodio::Decoder::new(cursor).map_err(|e| format!("Failed to decode audio: {}", e))?;

        sink.append(source);
        sink.sleep_until_end();

        self.clean();

        Ok(())
    }

    pub fn play_audio(
        &self,
        source_audio: SamplesBuffer<f32>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.stop();

        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to setup audio output: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        let sink = Arc::new(sink);

        *self.sink.lock().unwrap() = Some(Arc::clone(&sink));

        *self.state.lock().unwrap() = State::Playing;

        sink.append(source_audio);
        sink.sleep_until_end();

        self.clean();

        Ok(())
    }

    fn clean(&self) {
        *self.sink.lock().unwrap() = None;
        *self.state.lock().unwrap() = State::Idle;
    }

    pub fn generate_source(audio_data: Vec<u8>, speed: f32) -> SamplesBuffer<f32> {
        let pcm_data = if audio_data.starts_with(b"RIFF") && audio_data.len() > 44 {
            &audio_data[44..]
        } else {
            audio_data.as_slice()
        };

        let samples = Self::process_audio(pcm_data, speed);

        SamplesBuffer::new(1, 22050, samples)
    }

    /// Process audio data for playback, applying time-stretching if needed
    fn process_audio(audio_data: &[u8], speed: f32) -> Vec<f32> {
        let mut samples: Vec<f32> = audio_data
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / 32768.0
            })
            .collect();

        if (speed - 1.0).abs() > 0.01 {
            samples = wsola_normalized(&samples, speed, 60);
        }

        samples
    }

    pub fn pause(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(sink) = &*self.sink.lock().unwrap() {
            if !sink.is_paused() {
                sink.pause();
                *self.state.lock().unwrap() = State::Paused;
            } else {
                sink.play();
                *self.state.lock().unwrap() = State::Playing;
            }
        }
        Ok(())
    }

    pub fn play(&self) {
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.play();
            *self.state.lock().unwrap() = State::Playing;
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(*self.state.lock().unwrap(), State::Playing)
    }

    pub fn is_paused(&self) -> bool {
        matches!(*self.state.lock().unwrap(), State::Paused)
    }

    pub fn stop(&self) {
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.stop();
        }
        self.clean();
    }
}
