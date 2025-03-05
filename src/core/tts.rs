use super::{runtime::runtime, voice_manager::VoiceManager};
use crate::utils::{audio_player::AudioPlayer, highlighter::ReadingBlock};
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
use tokio::sync::mpsc;

struct AudioData {
    id: usize,
    raw_audio: Vec<u8>,
}

struct AudioQueue {
    sender: mpsc::Sender<AudioData>,
    receiver: mpsc::Receiver<AudioData>,
}

impl AudioQueue {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self { sender, receiver }
    }

    pub fn get_sender(&self) -> mpsc::Sender<AudioData> {
        self.sender.clone()
    }

    pub fn get_receiver(&mut self) -> &mut mpsc::Receiver<AudioData> {
        &mut self.receiver
    }
}

#[derive(Clone)]
pub struct Tts {
    pub sender: Arc<Sender<TTSEvent>>,
    idx: Arc<AtomicUsize>,
    reading_speed: RefCell<f32>,
    audio_player: Arc<AudioPlayer>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { block_id: u32 },
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

    pub async fn read_blocks_by_voice<T>(
        &self,
        voice: String,
        reading_blocks: Vec<T>,
    ) -> Result<(), Box<dyn Error>>
    where
        T: ReadingBlock + Send + Sync + 'static,
    {
        let mut audio_queue = AudioQueue::new(2);
        let reading_blocks_len = reading_blocks.len();
        let last_block_id = reading_blocks.last().map(|b| b.get_id());

        let producer_sender = audio_queue.get_sender();
        let receiver = audio_queue.get_receiver();

        let idx_clone = Arc::clone(&self.idx);

        let producer_handle = runtime().spawn(async move {
            while idx_clone.load(Ordering::Relaxed) < reading_blocks_len {
                let current_idx = idx_clone.load(Ordering::Relaxed);
                let reading_block = &reading_blocks[current_idx];
                idx_clone.fetch_add(1, Ordering::Relaxed);
                let raw_audio =
                    VoiceManager::generate_piper_raw_speech(&reading_block.get_text(), &voice)
                        .await?;
                let id = reading_block.get_id() as usize;

                if producer_sender
                    .send(AudioData { id, raw_audio })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok::<_, Box<dyn Error + Send + Sync>>(())
        });

        while let Some(audio_data) = receiver.recv().await {
            self.sender.send(TTSEvent::Progress {
                block_id: audio_data.id as u32,
            })?;

            let event = self.read_block_of_text(audio_data.raw_audio).await;

            match event {
                Ok(Some(TTSEvent::Stop)) => {
                    self.idx.store(0, Ordering::Relaxed);
                    break;
                }
                //TODO: Fix going backwards
                Ok(Some(TTSEvent::Prev)) => {
                    if audio_data.id > 0 {
                        while receiver.try_recv().is_ok() {}
                        self.idx.store(audio_data.id - 1, Ordering::Relaxed);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Error(e))) => return Err(e.into()),
                Err(e) => return Err(e),
                _ => {
                    if let Some(last_id) = last_block_id {
                        if last_id == audio_data.id as u32 {
                            break;
                        }
                    }
                    continue;
                }
            };
        }

        if let Err(e) = producer_handle.await {
            return Err(Box::new(e));
        }

        self.idx.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        raw_audio: Vec<u8>,
    ) -> Result<Option<TTSEvent>, Box<dyn Error>> {
        let mut receiver = self.sender.subscribe();
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
        if self.is_playing() {
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

    pub fn is_playing(&self) -> bool {
        self.audio_player.is_playing()
    }

    pub fn set_speed(&self, speed: f32) {
        self.reading_speed.replace(speed);
    }
}
