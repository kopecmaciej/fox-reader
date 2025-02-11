use super::{runtime::runtime, voice_manager::VoiceManager};
use crate::utils::{audio_player::AudioPlayer, text_highlighter::ReadBlock};
use std::{
    cell::RefCell,
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::broadcast::{self, Sender};

#[derive(Clone)]
pub struct Tts {
    pub sender: Arc<Sender<TTSEvent>>,
    idx: Arc<AtomicUsize>,
    reading_speed: RefCell<f32>,
    audio_player: Arc<AudioPlayer>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { offset_start: i32, offset_end: i32 },
    Stop,
    Next,
    Prev,
    Error(String),
}

impl fmt::Debug for Tts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tts").field("sender", &self.sender).finish()
    }
}

impl Default for Tts {
    fn default() -> Self {
        Self::new()
    }
}

impl Tts {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(4);
        Self {
            sender: Arc::new(sender),
            idx: Arc::new(AtomicUsize::new(0)),
            reading_speed: RefCell::new(1.0),
            audio_player: Arc::new(AudioPlayer::new()),
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

            match event {
                Ok(Some(TTSEvent::Stop)) => {
                    self.idx.store(0, Ordering::Relaxed);
                    break;
                }
                Ok(Some(TTSEvent::Next)) => {
                    if current_idx + 1 < reading_blocks.len() {
                        self.idx.store(current_idx + 1, Ordering::Relaxed);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Prev)) => {
                    if current_idx > 0 {
                        self.idx.store(current_idx - 1, Ordering::Relaxed);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Error(e))) => return Err(e.into()),
                Err(e) => return Err(e),
                _ => {
                    self.idx.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };
        }

        self.idx.store(0, Ordering::Relaxed);
        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        reading_block: String,
        voice: String,
    ) -> Result<Option<TTSEvent>, Box<dyn Error>> {
        let mut receiver = self.sender.subscribe();

        let raw_audio = runtime().block_on(VoiceManager::generate_piper_raw_speech(
            &reading_block,
            &voice,
        ))?;

        let reading_speed = *self.reading_speed.borrow();
        let audio_player = self.audio_player.clone();

        let result = runtime()
            .spawn(async move {
                let play_handle =
                    tokio::spawn(async move { audio_player.play_wav(raw_audio, reading_speed) });

                tokio::select! {
                    play_result = play_handle => {
                        match play_result {
                            Ok(Ok(_)) => None,
                            Ok(Err(e)) => Some(TTSEvent::Error(e.to_string())),
                            Err(e) => Some(TTSEvent::Error(e.to_string())),
                        }
                    }
                    Ok(event) = receiver.recv() => {
                        match event {
                            TTSEvent::Stop => Some(TTSEvent::Stop),
                            TTSEvent::Next => Some(TTSEvent::Next),
                            TTSEvent::Prev => Some(TTSEvent::Prev),
                            _ => None,
                        }
                    }
                }
            })
            .await?;

        Ok(result)
    }

    pub async fn stop(&self, send_event: bool) -> Result<(), Box<dyn Error>> {
        self.audio_player.stop();
        if send_event {
            self.sender.send(TTSEvent::Stop)?;
        }
        Ok(())
    }

    pub async fn pause_if_playing(&self) -> bool {
        if self.is_playing().await {
            if let Err(e) = self.audio_player.pause() {
                let _ = self.sender.send(TTSEvent::Error(e.to_string()));
                return false;
            }
            return true;
        }
        false
    }

    pub async fn resume_if_paused(&self) -> bool {
        if self.is_paused() {
            self.audio_player.play();
            return true;
        }
        false
    }

    pub async fn prev(&self) -> Result<(), Box<dyn Error>> {
        self.sender.send(TTSEvent::Prev)?;
        self.stop(false).await
    }

    pub async fn next(&self) -> Result<(), Box<dyn Error>> {
        self.sender.send(TTSEvent::Next)?;
        self.stop(false).await
    }

    pub async fn repeat_block(&self) -> Result<(), Box<dyn Error>> {
        self.idx.fetch_sub(1, Ordering::Relaxed);
        self.stop(false).await?;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.audio_player.is_paused()
    }

    pub async fn is_playing(&self) -> bool {
        self.audio_player.is_playing()
    }

    pub fn set_speed(&self, speed: f32) {
        self.reading_speed.replace(speed);
    }
}
