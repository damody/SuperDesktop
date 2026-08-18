use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use shell_provider_protocol::{NotificationHostResponse, NotificationMutation};

pub struct NotificationClient {
    executable: PathBuf,
    child: Option<Child>,
    input: Option<ChildStdin>,
    responses: Option<Receiver<NotificationHostResponse>>,
    compatibility_enabled: bool,
}

impl NotificationClient {
    pub fn adjacent(compatibility_enabled: bool) -> Result<Self, &'static str> {
        let executable = std::env::current_exe()
            .map_err(|_| "notification-current-executable")?
            .parent()
            .ok_or("notification-no-binary-directory")?
            .join("notification-area-host.exe");
        Ok(Self {
            executable,
            child: None,
            input: None,
            responses: None,
            compatibility_enabled,
        })
    }

    pub fn request(
        &mut self,
        request: &NotificationMutation,
        timeout: Duration,
    ) -> Result<NotificationHostResponse, &'static str> {
        if self.child.is_none() {
            self.start()?;
        }
        let result = (|| {
            let input = self
                .input
                .as_mut()
                .ok_or("notification-input-unavailable")?;
            serde_json::to_writer(&mut *input, request)
                .map_err(|_| "notification-request-serialize")?;
            input
                .write_all(b"\n")
                .and_then(|()| input.flush())
                .map_err(|_| "notification-request-write")?;
            self.responses
                .as_ref()
                .ok_or("notification-response-unavailable")?
                .recv_timeout(timeout)
                .map_err(|_| "notification-response-timeout")
        })();
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if !self.executable.is_file() {
            return Err("notification-executable-missing");
        }
        let mut command = Command::new(&self.executable);
        if self.compatibility_enabled {
            command.arg("--shell-notifyicon");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "notification-spawn")?;
        let Some(input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("notification-stdin");
        };
        let Some(output) = child.stdout.take() else {
            drop(input);
            let _ = child.kill();
            let _ = child.wait();
            return Err("notification-stdout");
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                let Ok(response) = serde_json::from_str(&line) else {
                    break;
                };
                if sender.send(response).is_err() {
                    break;
                }
            }
        });
        self.child = Some(child);
        self.input = Some(input);
        self.responses = Some(receiver);
        Ok(())
    }

    fn reset(&mut self) {
        self.input.take();
        self.responses.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for NotificationClient {
    fn drop(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compatibility_switch_is_explicit_and_never_enabled_by_preview_default() {
        let client = include_str!("notification_client.rs");
        let composition = include_str!("surface_runtime.rs");
        assert!(client.contains("if self.compatibility_enabled"));
        assert!(client.contains("command.arg(\"--shell-notifyicon\")"));
        assert!(composition.contains("NotificationClient::adjacent(shell)"));
        assert!(!composition.contains("NotificationClient::adjacent(true)"));
    }
}
