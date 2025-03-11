use super::{runtime::runtime, voice_manager::VoiceManager};
use crate::utils::{audio_player::AudioPlayer, highlighter::ReadingBlock};
use std::{
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::{
    broadcast::{self, Sender},
    Mutex, Semaphore,
};

struct AudioData {
    id: u32,
    source_audio: rodio::buffer::SamplesBuffer<f32>,
}

struct BoundedBuffer<T> {
    buffer: Mutex<Vec<T>>,
    slots: Semaphore,
    items: Semaphore,
}

impl<T> BoundedBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Mutex::new(Vec::with_capacity(capacity)),
            slots: Semaphore::new(capacity),
            items: Semaphore::new(0),
        }
    }

    async fn wait_for_slots(&self) {
        let permit = self.slots.acquire().await.unwrap();
        permit.forget();
    }

    async fn push_to_start(&self, item: T) {
        {
            let mut buffer = self.buffer.lock().await;
            buffer.insert(0, item);
        }

        self.items.add_permits(1);
    }

    async fn produce(&self, item: T) {
        {
            let mut buffer = self.buffer.lock().await;
            buffer.push(item);
        }

        self.items.add_permits(1);
    }

    async fn purge_buffer(&self) {
        self.buffer.lock().await.clear();
        let all_perm = self.items.available_permits();
        self.items.forget_permits(all_perm);
        self.slots.add_permits(2);
    }

    async fn consume(&self) -> T {
        let permit = self.items.acquire().await.unwrap();
        permit.forget();

        let item = {
            let mut buffer = self.buffer.lock().await;
            buffer.remove(0)
        };

        self.slots.add_permits(1);

        item
    }
}

#[derive(Clone)]
pub struct Tts {
    pub sender: Arc<Sender<TTSEvent>>,
    idx: Arc<AtomicUsize>,
    reading_speed: Arc<AtomicUsize>,
    audio_player: Arc<AudioPlayer>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { block_id: u32 },
    Stop,
    Next,
    Repeat,
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
            reading_speed: Arc::new(AtomicUsize::new(100)),
            audio_player: Arc::new(AudioPlayer::default()),
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
        if reading_blocks.is_empty() {
            return Ok(());
        }
        let reading_blocks_len = reading_blocks.len();
        let buffer = Arc::new(BoundedBuffer::new(2));
        self.idx
            .store(reading_blocks[0].get_id() as usize, Ordering::SeqCst);
        let idx_clone = Arc::clone(&self.idx);
        let producer_buffer = buffer.clone();
        let last_id = reading_blocks[reading_blocks_len - 1].get_id();
        let reading_speed = self.reading_speed.clone();

        let producer_handle = runtime().spawn(async move {
            while idx_clone.load(Ordering::SeqCst) < reading_blocks_len {
                producer_buffer.wait_for_slots().await;
                let idx = idx_clone.load(Ordering::SeqCst);
                let reading_block = &reading_blocks[idx];

                let reading_speed = reading_speed.load(Ordering::SeqCst);
                let speed = Self::spin_value_to_rate_percent(reading_speed);
                let source_audio = VoiceManager::generate_piper_raw_speech(
                    &reading_block.get_text(),
                    &voice,
                    speed,
                )
                .await?;

                let id = reading_block.get_id();
                // if Prev occurse, we'll have diffrent idx so
                // we have to skip move to proper block
                if idx_clone.load(Ordering::SeqCst) != idx {
                    producer_buffer.slots.add_permits(1);
                    continue;
                }
                producer_buffer
                    .produce(AudioData { id, source_audio })
                    .await;
                idx_clone.fetch_add(1, Ordering::SeqCst);
            }
            Ok::<_, Box<dyn Error + Send + Sync>>(())
        });

        loop {
            let audio_data = buffer.consume().await;
            let id = audio_data.id;
            self.sender.send(TTSEvent::Progress {
                block_id: id as u32,
            })?;

            let event = self
                .read_block_of_text(audio_data.source_audio.clone())
                .await;

            match event {
                Ok(Some(TTSEvent::Stop)) => {
                    self.idx.store(0, Ordering::SeqCst);
                    break;
                }
                Ok(Some(TTSEvent::Repeat)) => {
                    buffer.purge_buffer().await;
                    buffer.push_to_start(audio_data).await;
                    self.idx.store(id as usize, Ordering::SeqCst);
                    continue;
                }
                Ok(Some(TTSEvent::Prev)) => {
                    if audio_data.id > 0 {
                        buffer.purge_buffer().await;
                        self.idx.store((id - 1) as usize, Ordering::SeqCst);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Error(e))) => return Err(e.into()),
                Err(e) => return Err(e),
                _ => {
                    if last_id == id {
                        break;
                    }
                    continue;
                }
            };
        }

        if let Err(e) = producer_handle.await {
            return Err(Box::new(e));
        }

        self.idx.store(0, Ordering::SeqCst);

        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        source_audio: rodio::buffer::SamplesBuffer<f32>,
    ) -> Result<Option<TTSEvent>, Box<dyn Error>> {
        let mut receiver = self.sender.subscribe();
        let audio_player = self.audio_player.clone();

        let result = runtime()
            .spawn(async move {
                let play_handle =
                    tokio::spawn(async move { audio_player.play_audio(source_audio) });

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
        self.sender.send(TTSEvent::Repeat)?;
        self.stop(false).await?;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.audio_player.is_paused()
    }

    pub fn is_playing(&self) -> bool {
        self.audio_player.is_playing()
    }

    pub fn set_speed(&self, speed: f64) {
        self.reading_speed.store(speed as usize, Ordering::SeqCst);
    }

    pub fn get_speed(&self) -> u8 {
        let speed = self.reading_speed.load(Ordering::SeqCst);

        ((speed as f32 / 200.0) * 100.0) as u8
    }

    fn spin_value_to_rate_percent(spin_value: usize) -> Option<u8> {
        if spin_value == 100 {
            return None;
        }
        // Map 50 to 0, 100 to ~9, 550 to 100
        let normalized = (spin_value as f32 - 50.0) / 5.0;
        Some(normalized.clamp(0.0, 100.0).round() as u8)
    }
}
