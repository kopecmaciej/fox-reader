use super::{runtime::runtime, voice_manager::VoiceManager};
use crate::{
    core::runtime::spawn_tokio,
    utils::{audio_player::AudioPlayer, highlighter::ReadingBlock},
};
use rodio::buffer::SamplesBuffer;
use std::{
    collections::{BTreeMap, HashMap},
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
    current_id: Arc<AtomicUsize>,
    reading_speed: Arc<AtomicUsize>,
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
            current_id: Arc::new(AtomicUsize::new(0)),
            reading_speed: Arc::new(AtomicUsize::new(100)),
            audio_player: Arc::new(AudioPlayer::default()),
        }
    }

    pub async fn read_blocks_by_voice<T>(
        &self,
        voice: String,
        blocks_map: BTreeMap<u32, T>,
        start_from: u32,
    ) -> Result<(), Box<dyn Error>>
    where
        T: ReadingBlock + Send + Sync + 'static + Clone,
    {
        if blocks_map.is_empty() {
            return Ok(());
        }

        self.current_id.store(start_from as usize, Ordering::SeqCst);
        let mut processed_blocks: HashMap<usize, SamplesBuffer<f32>> = HashMap::new();

        while self.current_id.load(Ordering::SeqCst) < blocks_map.len() {
            let current_idx = self.current_id.load(Ordering::SeqCst);
            let reading_speed_value = self.reading_speed.load(Ordering::SeqCst);
            let speed = Self::spin_value_to_rate_percent(reading_speed_value);

            let reading_block = &blocks_map.get(&(current_idx as u32)).unwrap();
            if let std::collections::hash_map::Entry::Vacant(e) =
                processed_blocks.entry(current_idx)
            {
                let source_audio = VoiceManager::generate_piper_raw_speech(
                    &reading_block.get_text(),
                    &voice,
                    speed,
                )
                .await?;
                e.insert(source_audio);
            }

            let source_audio = processed_blocks.get(&current_idx).unwrap().clone();
            let event_future = self.read_block_of_text(source_audio.clone());

            self.sender.send(TTSEvent::Progress {
                block_id: current_idx as u32,
            })?;

            let next_block = current_idx + 1;
            let event =
                if !processed_blocks.contains_key(&(next_block)) && next_block < blocks_map.len() {
                    let reading_block = blocks_map.get(&(next_block as u32)).unwrap().clone();
                    let voice_clone = voice.clone();
                    let voice_future = spawn_tokio(async move {
                        match VoiceManager::generate_piper_raw_speech(
                            &reading_block.get_text(),
                            &voice_clone,
                            speed,
                        )
                        .await
                        {
                            Ok(audio) => Ok::<_, Box<dyn std::error::Error + Send + Sync>>(audio),
                            Err(e) => Err(format!("Error generating speech: {}", e).into()),
                        }
                    });

                    let (event_res, next_audio_result) = tokio::join!(event_future, voice_future);
                    match next_audio_result {
                        Ok(audio) => processed_blocks.insert(next_block, audio),
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    event_res
                } else {
                    self.read_block_of_text(source_audio.clone()).await
                };

            match event {
                Ok(Some(TTSEvent::Stop)) => {
                    self.current_id.store(0, Ordering::SeqCst);
                    processed_blocks.clear();
                    break;
                }
                Ok(Some(TTSEvent::Next)) => {
                    if current_idx + 1 < blocks_map.len() {
                        self.current_id.store(current_idx + 1, Ordering::SeqCst);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Prev)) => {
                    if current_idx > 0 {
                        self.current_id.store(current_idx - 1, Ordering::SeqCst);
                    }
                    continue;
                }
                Ok(Some(TTSEvent::Error(e))) => return Err(e.into()),
                Err(e) => return Err(e),
                _ => {
                    self.current_id.fetch_add(1, Ordering::SeqCst);
                    continue;
                }
            };
        }

        self.current_id.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub async fn read_block_of_text(
        &self,
        source_audio: SamplesBuffer<f32>,
    ) -> Result<Option<TTSEvent>, Box<dyn Error>> {
        let mut receiver = self.sender.subscribe();
        let audio_player = self.audio_player.clone();

        let result = runtime()
            .spawn(async move {
                let play_handle = spawn_tokio(async move { audio_player.play_audio(source_audio) });

                tokio::select! {
                    play_result = play_handle => {
                        match play_result {
                            Ok(_) => None,
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
        self.current_id.fetch_sub(1, Ordering::SeqCst);
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
