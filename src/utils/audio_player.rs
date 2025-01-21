use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub struct AudioPlayer {
    process: Option<Child>,
    ipc_socket: Option<Arc<Mutex<UnixStream>>>,
    socket_path: Option<PathBuf>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            process: None,
            ipc_socket: None,
            socket_path: None,
        }
    }

    pub async fn play_audio(
        &mut self,
        audio_data: Vec<u8>,
        speed: f32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean up previous instance if exists
        self.stop().await?;

        let socket_path = std::env::temp_dir().join("mpv-socket");
        self.socket_path = Some(socket_path.clone());

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

        // Write audio data
        if let Some(mut stdin) = mpv_process.stdin.take() {
            tokio::io::AsyncWriteExt::write_all(&mut stdin, &audio_data).await?;
        }

        // Wait a bit for MPV to create the socket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect to the IPC socket
        let socket = UnixStream::connect(&socket_path).await?;
        self.ipc_socket = Some(Arc::new(Mutex::new(socket)));
        self.process = Some(mpv_process);

        Ok(())
    }

    pub async fn pause(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(socket) = &self.ipc_socket {
            let mut socket = socket.lock().await;
            socket
                .write_all(b"{\"command\": [\"cycle\", \"pause\"]}\n")
                .await?;
        }
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Kill the process if it exists
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }

        // Clean up the socket
        if let Some(socket_path) = self.socket_path.take() {
            let _ = std::fs::remove_file(socket_path);
        }

        self.ipc_socket = None;

        Ok(())
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        // Clean up resources in case the struct is dropped
        if let Some(socket_path) = &self.socket_path {
            let _ = std::fs::remove_file(socket_path);
        }
    }
}
