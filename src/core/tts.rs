use crate::utils::text_highlighter::ReadBlock;

use super::{runtime::runtime, voice_manager::VoiceManager};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    Mutex,
};

#[derive(Debug, Clone)]
pub struct Tts {
    pub sender: Arc<Sender<TTSEvent>>,
    pub receiver: Arc<Mutex<Receiver<TTSEvent>>>,
    pub in_progress: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum TTSEvent {
    Progress { offset_start: i32, offset_end: i32 },
    Stop,
    Done,
    Error(String),
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

            let event = self.read_block_of_text(&reading_block.block, voice).await;
            {
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
        reading_block: &str,
        voice: &str,
    ) -> Result<TTSEvent, Box<dyn Error>> {
        let mut process =
            runtime().block_on(VoiceManager::play_text_using_piper(reading_block, voice))?;

        let mut reciever = self.sender.subscribe();
        let in_progress = self.in_progress.clone();
        let result = runtime()
            .spawn(async move {
                in_progress.store(true, Ordering::Relaxed);
                tokio::select! {
                    _res = process.wait() => TTSEvent::Done,
                    event = reciever.recv() => {
                        if let Ok(TTSEvent::Stop) = event {
                            if let Err(e) = process.terminate_group().await {
                                TTSEvent::Error(format!("{e}"))
                            } else {
                                TTSEvent::Stop
                            }
                        } else {
                            TTSEvent::Done
                        }
                    },
                }
            })
            .await?;

        Ok(result)
    }

    pub async fn stop(&self) {
        let _ = self.sender.send(TTSEvent::Stop);
    }

    pub fn is_running(&self) -> bool {
        self.in_progress.load(Ordering::Relaxed)
    }
}
