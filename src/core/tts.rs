use crate::utils::text_highlighter::ReadBlock;

use super::{runtime::runtime, voice_manager::VoiceManager};
use rodio::Sink;
use std::{
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
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
    pub in_progress: Arc<AtomicBool>,
    pub sink: Arc<std::sync::Mutex<Option<Arc<Sink>>>>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { offset_start: i32, offset_end: i32 },
    Stop,
    Done,
    Error(String),
}

impl fmt::Debug for Tts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tts")
            .field("sender", &self.sender)
            .field("receiver", &self.receiver)
            .field("in_progress", &self.in_progress)
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
            in_progress: Arc::new(AtomicBool::new(false)),
            sink: Arc::new(std::sync::Mutex::new(None::<Arc<Sink>>)),
        }
    }

    pub async fn read_block_by_voice(
        &self,
        voice: &str,
        reading_block: Vec<ReadBlock>,
    ) -> Result<(), Box<dyn Error>> {
        for reading_block in reading_block {
            self.sender.send(TTSEvent::Progress {
                offset_start: reading_block.start_offset,
                offset_end: reading_block.end_offset,
            })?;

            let event = self
                .read_block_of_text(reading_block.block, voice.to_string())
                .await;
            {
                println!("{:?}", event);
                match event {
                    Ok(TTSEvent::Stop) => break,
                    Ok(TTSEvent::Error(e)) => return Err(e.into()),
                    Err(e) => e,
                    _ => {
                        continue;
                    }
                };
            }
        }

        self.in_progress.store(false, Ordering::Relaxed);

        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        reading_block: String,
        voice: String,
    ) -> Result<TTSEvent, Box<dyn Error>> {
        let in_progress = self.in_progress.clone();
        let sink = self.sink.clone();
        let mut receiver = self.sender.subscribe();

        let raw_audio = runtime().block_on(VoiceManager::generate_piper_raw_speech(
            &reading_block,
            &voice,
        ))?;

        let result = runtime()
            .spawn(async move {
                in_progress.store(true, Ordering::Relaxed);

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
                            _ => TTSEvent::Done,
                        }
                    }
                }
            })
            .await?;

        Ok(result)
    }

    pub async fn stop(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            let _ = self.sender.send(TTSEvent::Stop);
            sink.stop();
        }
    }

    pub fn is_running(&self) -> bool {
        self.in_progress.load(Ordering::Relaxed)
    }
}
