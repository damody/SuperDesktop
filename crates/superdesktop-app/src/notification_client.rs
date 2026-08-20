use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use shell_provider_protocol::{
    NotificationHostResponse, NotificationMutation, NotificationSnapshot,
};

const MAX_CONSECUTIVE_RESTARTS: u8 = 3;

pub struct NotificationClient {
    executable: PathBuf,
    child: Option<Child>,
    input: Option<ChildStdin>,
    responses: Option<Receiver<NotificationHostResponse>>,
    compatibility_enabled: bool,
    consecutive_failures: u8,
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
            consecutive_failures: 0,
        })
    }

    pub fn request(
        &mut self,
        request: &NotificationMutation,
        timeout: Duration,
    ) -> Result<NotificationHostResponse, &'static str> {
        if self.child.is_none() {
            if self.consecutive_failures >= MAX_CONSECUTIVE_RESTARTS {
                return Err("notification-restart-capacity");
            }
            if let Err(error) = self.start() {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                return Err(error);
            }
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
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            self.reset();
        } else {
            self.consecutive_failures = 0;
        }
        result
    }

    pub fn dismiss_notification(
        &mut self,
        notification_id: String,
        expected_generation: u64,
        timeout: Duration,
    ) -> Result<NotificationSnapshot, &'static str> {
        self.mutate_and_snapshot(
            NotificationMutation::DismissNotification {
                notification_id,
                expected_generation,
            },
            timeout,
        )
    }

    pub fn clear_notifications(
        &mut self,
        expected_generation: u64,
        timeout: Duration,
    ) -> Result<NotificationSnapshot, &'static str> {
        self.mutate_and_snapshot(
            NotificationMutation::ClearNotifications {
                expected_generation,
            },
            timeout,
        )
    }

    fn mutate_and_snapshot(
        &mut self,
        mutation: NotificationMutation,
        timeout: Duration,
    ) -> Result<NotificationSnapshot, &'static str> {
        match self.request(&mutation, timeout)? {
            NotificationHostResponse::Accepted { .. } => {}
            NotificationHostResponse::Rejected(_) => return Err("notification-mutation-rejected"),
            _ => return Err("notification-mutation-terminal"),
        }
        match self.request(&NotificationMutation::Snapshot, timeout)? {
            NotificationHostResponse::Snapshot(snapshot) => Ok(snapshot),
            NotificationHostResponse::Rejected(_) => Err("notification-snapshot-rejected"),
            _ => Err("notification-snapshot-terminal"),
        }
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
        assert!(composition.contains("SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT"));
        assert!(composition.contains("verification_surface.as_deref() == Some(\"taskbar\")"));
        assert!(composition.contains("shell || verification_notifyicon_compatibility"));
        assert!(!composition.contains("NotificationClient::adjacent(true)"));
        assert!(client.contains("MAX_CONSECUTIVE_RESTARTS"));
        assert!(client.contains("notification-restart-capacity"));
        assert!(composition.contains("notification-compatibility-handshake"));
        assert!(client.contains("NotificationMutation::DismissNotification"));
        assert!(client.contains("NotificationMutation::ClearNotifications"));
        assert!(client.contains("mutate_and_snapshot"));
        assert!(
            composition.contains("Ok(NotificationHostResponse::Health(health)) if health.healthy")
        );
    }
}
