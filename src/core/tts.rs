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
    Terminate,
    Done,
    Error(String),
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

    pub async fn read_text_by_voice(&self, voice: &str, text: &str) -> Result<(), Box<dyn Error>> {
        let sentences = Self::split_text_to_sentences(text);
        let mut current_offset = 0;
        for sentence in sentences {
            println!("{current_offset}");
            let offset_start = current_offset;
            let offset_end = offset_start + sentence.len() as i32;

            let _ = self.sender.send(TTSEvent::Progress {
                offset_start,
                offset_end,
            });

            let event = self.read_sentence(sentence, voice).await;
            {
                match event {
                    Ok(TTSEvent::Terminate) => break,
                    Ok(TTSEvent::Error(e)) => return Err(e.into()),
                    Err(e) => e,
                    _ => {
                        current_offset = offset_end;
                        continue;
                    }
                };
            }
        }

        self.in_progress.store(false, Ordering::Relaxed);

        Ok(())
    }

    pub async fn read_sentence(
        &self,
        sentence: &str,
        voice: &str,
    ) -> Result<TTSEvent, Box<dyn Error>> {
        let mut process =
            runtime().block_on(VoiceManager::play_text_using_piper(sentence, voice))?;

        let mut reciever = self.sender.subscribe();
        let in_progress = self.in_progress.clone();
        let result = runtime()
            .spawn(async move {
                in_progress.store(true, Ordering::Relaxed);
                tokio::select! {
                    _res = process.wait() => TTSEvent::Done,
                    event = reciever.recv() => {
                        if let Ok(TTSEvent::Terminate) = event {
                            if let Err(e) = process.terminate_group().await {
                                TTSEvent::Error(format!("{e}"))
                            } else {
                                TTSEvent::Terminate
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
        let _ = self.sender.send(TTSEvent::Terminate);
    }

    pub fn is_running(&self) -> bool {
        self.in_progress.load(Ordering::Relaxed)
    }

    fn split_text_to_sentences(text: &str) -> Vec<&str> {
        text.split(['.']).filter(|s| !s.trim().is_empty()).collect()
    }
}
