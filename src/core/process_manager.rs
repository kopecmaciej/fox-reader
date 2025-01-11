use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::error::Error;
use tokio::process::Child;

pub struct ProcessManager {
    child: Child,
}

impl ProcessManager {
    pub fn new(child: Child) -> Self {
        Self { child }
    }

    pub async fn terminate(&mut self) -> Result<(), Box<dyn Error>> {
        let pid = self.child.id().ok_or("Failed to get process ID")?;

        signal::kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM)?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if let Err(e) = self.child.kill().await {
            eprintln!("Failed to kill process: {}", e);
        }

        self.child.wait().await?;
        Ok(())
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}
