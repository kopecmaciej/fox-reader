use tokio::process::Child;

pub struct ProcessManager {
    child: Child,
}

//TODO: REFACTOR
impl ProcessManager {
    pub fn new(child: Child) -> Self {
        Self { child }
    }

    pub async fn terminate(&mut self) {
        let pid = self.child.id().expect("Failed to get process ID");

        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if let Err(e) = self.child.kill().await {
            eprintln!("Failed to kill process: {}", e);
        }

        if let Err(e) = self.child.wait().await {
            eprintln!("Failed to wait for process termination: {}", e);
        }
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}
