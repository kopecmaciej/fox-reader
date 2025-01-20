use crate::utils::text_highlighter::ReadBlock;

use super::{runtime::runtime, voice_manager::VoiceManager};
use rodio::Sink;
use std::{
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    Mutex,
};

#[derive(Clone)]
pub struct Tts {
    pub sender: Arc<Sender<TTSEvent>>,
    pub receiver: Arc<Mutex<Receiver<TTSEvent>>>,
    pub sink: Arc<std::sync::Mutex<Option<Arc<Sink>>>>,
    pub idx: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { offset_start: i32, offset_end: i32 },
    Stop,
    Next,
    Prev,
    Done,
    Error(String),
}

impl fmt::Debug for Tts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tts")
            .field("sender", &self.sender)
            .field("receiver", &self.receiver)
            .field("sink", &"<sink>")
            .finish()
    }
}

impl Default for Tts {
    fn default() -> Self {
        Self::new()
    }
}

impl Tts {
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(4);
        Self {
            sender: Arc::new(sender),
            receiver: Arc::new(Mutex::new(receiver)),
            sink: Arc::new(std::sync::Mutex::new(None::<Arc<Sink>>)),
            idx: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn read_block_by_voice(
        &self,
        voice: &str,
        reading_blocks: Vec<ReadBlock>,
    ) -> Result<(), Box<dyn Error>> {
        while self.idx.load(Ordering::Relaxed) < reading_blocks.len() {
            let current_idx = self.idx.load(Ordering::Relaxed);
            let reading_block = &reading_blocks[current_idx];
            self.sender.send(TTSEvent::Progress {
                offset_start: reading_block.start_offset,
                offset_end: reading_block.end_offset,
            })?;

            let event = self
                .read_block_of_text(reading_block.block.clone(), voice.to_string())
                .await;
            {
                match event {
                    Ok(TTSEvent::Stop) => break,
                    Ok(TTSEvent::Next) => {
                        if current_idx + 1 < reading_blocks.len() {
                            self.idx.store(current_idx + 1, Ordering::Relaxed);
                        }
                        continue;
                    }
                    Ok(TTSEvent::Prev) => {
                        if current_idx > 0 {
                            self.idx.store(current_idx - 1, Ordering::Relaxed);
                        }
                        continue;
                    }
                    Ok(TTSEvent::Error(e)) => return Err(e.into()),
                    Err(e) => e,
                    _ => {
                        self.idx.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                };
            }
        }

        self.idx.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        reading_block: String,
        voice: String,
    ) -> Result<TTSEvent, Box<dyn Error>> {
        let sink = self.sink.clone();
        let mut receiver = self.sender.subscribe();

        let raw_audio = runtime().block_on(VoiceManager::generate_piper_raw_speech(
            &reading_block,
            &voice,
        ))?;

        let result = runtime()
            .spawn(async move {
                let play_handle =
                    tokio::spawn(async move { VoiceManager::play_mkv_raw_audio(raw_audio, sink) });

                tokio::select! {
                    play_result = play_handle => {
                        match play_result {
                            Ok(Ok(_)) => TTSEvent::Done,
                            Ok(Err(e)) => TTSEvent::Error(e),
                            Err(e) => TTSEvent::Error(e.to_string()),
                        }
                    }
                    Ok(event) = receiver.recv() => {
                        match event {
                            TTSEvent::Stop => TTSEvent::Stop,
                            TTSEvent::Next => TTSEvent::Next,
                            TTSEvent::Prev => TTSEvent::Prev,
                            _ => TTSEvent::Done,
                        }
                    }
                }
            })
            .await?;

        Ok(result)
    }

    pub async fn stop(&self, send_event: bool) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            if send_event {
                let _ = self.sender.send(TTSEvent::Stop);
            }
            sink.stop();
        }
        self.idx.store(0, Ordering::Relaxed);
    }

    pub fn pause_if_playing(&self) -> bool {
        if self.is_running() {
            if let Some(sink) = self.sink.lock().unwrap().as_ref() {
                if !sink.is_paused() {
                    sink.pause();
                    return true;
                }
            }
        }
        false
    }

    pub fn resume_if_paused(&self) -> bool {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            if sink.is_paused() {
                sink.play();
                return true;
            }
        }
        false
    }

    pub async fn prev(&self) {
        let _ = self.sender.send(TTSEvent::Prev);
        self.stop(false).await;
    }

    pub async fn next(&self) {
        let _ = self.sender.send(TTSEvent::Next);
        self.stop(false).await;
    }

    pub fn is_running(&self) -> bool {
        match self.sink.lock() {
            Ok(sink) => sink.as_ref().map_or(false, |s| !s.empty()),
            Err(_) => false,
        }
    }
}
