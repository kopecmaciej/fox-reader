use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Command;

pub struct AudioPlayer {}

impl AudioPlayer {
    fn socket_path() -> PathBuf {
        std::env::temp_dir().join("mpv-socket")
    }

    async fn socket() -> std::io::Result<UnixStream> {
        UnixStream::connect(&Self::socket_path()).await
    }

    pub async fn play_audio(
        audio_data: Vec<u8>,
        speed: f32,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        let socket_path = Self::socket_path();
        let mut mpv_process = Command::new("mpv")
            .args([
                &format!("--speed={}", speed),
                "--no-terminal",
                "--keep-open=no",
                &format!("--input-ipc-server={}", socket_path.display()),
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = mpv_process.stdin.take() {
            tokio::io::AsyncWriteExt::write_all(&mut stdin, &audio_data).await?;
        }

        mpv_process.wait().await?;

        Ok(())
    }

    pub async fn pause() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut socket = Self::socket().await?;
        socket
            .write_all(b"{\"command\": [\"cycle\", \"pause\"]}\n")
            .await?;

        Ok(())
    }

    pub async fn stop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let socket_path = Self::socket_path();
        let mut socket = Self::socket().await?;
        socket.write_all(b"{\"command\": [\"quit\"]}\n").await?;
        let _ = std::fs::remove_file(socket_path);

        Ok(())
    }
}
