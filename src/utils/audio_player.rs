use crate::core::runtime::runtime;
use serde_json::json;
use std::error::Error;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Command;

pub struct AudioPlayer {}

impl AudioPlayer {
    const SOCKET_NAME: &'static str = "\0mpv-socket-fox-reader";

    async fn socket() -> std::io::Result<UnixStream> {
        UnixStream::connect(Self::SOCKET_NAME).await
    }

    pub async fn play_audio(
        audio_data: Vec<u8>,
        speed: f32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Self::stop().await?;
        let mut mpv_process = Command::new("mpv")
            .args([
                &format!("--speed={}", speed),
                "--no-terminal",
                "--keep-open=no",
                &format!("--input-ipc-server=@{}", &Self::SOCKET_NAME[1..]),
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        if let Some(mut stdin) = mpv_process.stdin.take() {
            tokio::io::AsyncWriteExt::write_all(&mut stdin, &audio_data)
                .await
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        }

        mpv_process
            .wait()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(())
    }

    pub async fn pause() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::send_command(&["cycle", "pause"]).await
    }

    pub async fn stop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::send_command(&["quit"]).await
    }

    async fn send_command(command: &[&str]) -> Result<(), Box<dyn Error + Send + Sync>> {
        match Self::socket().await {
            Ok(mut socket) => {
                let command_json = json!({
                    "command": command
                });
                let mut command_bytes = command_json.to_string().into_bytes();
                command_bytes.push(b'\n');
                socket.write_all(&command_bytes).await?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn cleanup() {
        runtime().spawn(async { Self::stop().await });
    }
}
