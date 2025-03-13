use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;

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
