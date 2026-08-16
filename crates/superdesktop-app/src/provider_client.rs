use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use shell_provider_protocol::{Envelope, ProviderRequest, ProviderResponse};

pub struct ProviderClient {
    executable: PathBuf,
    child: Option<Child>,
    input: Option<ChildStdin>,
    responses: Option<Receiver<ProviderResponse>>,
}

impl ProviderClient {
    pub fn adjacent() -> Result<Self, &'static str> {
        let executable = std::env::current_exe()
            .map_err(|_| "provider-current-executable")?
            .parent()
            .ok_or("provider-no-binary-directory")?
            .join("shell-provider-host.exe");
        Ok(Self {
            executable,
            child: None,
            input: None,
            responses: None,
        })
    }

    pub fn request(
        &mut self,
        request: &Envelope<ProviderRequest>,
        timeout: Duration,
    ) -> Result<ProviderResponse, &'static str> {
        if self.child.is_none() {
            self.start()?;
        }
        let input = self.input.as_mut().ok_or("provider-input-unavailable")?;
        serde_json::to_writer(&mut *input, request).map_err(|_| "provider-request-serialize")?;
        input
            .write_all(b"\n")
            .and_then(|()| input.flush())
            .map_err(|_| {
                self.reset();
                "provider-request-write"
            })?;
        let response = self
            .responses
            .as_ref()
            .ok_or("provider-response-unavailable")?
            .recv_timeout(timeout)
            .map_err(|_| {
                self.reset();
                "provider-response-timeout"
            })?;
        if response.request_id != request.request_id
            || response.correlation_id != request.correlation_id
        {
            self.reset();
            return Err("provider-response-mismatch");
        }
        Ok(response)
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if !self.executable.is_file() {
            return Err("provider-executable-missing");
        }
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "provider-spawn")?;
        let Some(input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("provider-stdin");
        };
        let Some(output) = child.stdout.take() else {
            drop(input);
            let _ = child.kill();
            let _ = child.wait();
            return Err("provider-stdout");
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                let Ok(response) = serde_json::from_str::<ProviderResponse>(&line) else {
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

impl Drop for ProviderClient {
    fn drop(&mut self) {
        self.reset();
    }
}
