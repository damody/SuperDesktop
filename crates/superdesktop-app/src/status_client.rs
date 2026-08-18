use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use shell_provider_protocol::{
    SystemStatusCommandTerminal, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusSnapshot, Validate,
};

const MAX_RESTART_ATTEMPTS: u8 = 3;

pub struct SystemStatusClient {
    executable: PathBuf,
    child: Option<Child>,
    input: Option<ChildStdin>,
    responses: Option<Receiver<SystemStatusHostResponse>>,
}

impl SystemStatusClient {
    pub fn adjacent() -> Result<Self, &'static str> {
        let executable = std::env::var_os("SUPERDESKTOP_SYSTEM_STATUS_HOST")
            .map(PathBuf::from)
            .unwrap_or(
                std::env::current_exe()
                    .map_err(|_| "status-current-executable")?
                    .parent()
                    .ok_or("status-no-binary-directory")?
                    .join("system-status-host.exe"),
            );
        Ok(Self::with_executable(executable))
    }

    pub fn with_executable(executable: PathBuf) -> Self {
        Self {
            executable,
            child: None,
            input: None,
            responses: None,
        }
    }

    pub fn request(
        &mut self,
        request: &SystemStatusHostRequest,
        timeout: Duration,
    ) -> Result<SystemStatusHostResponse, &'static str> {
        if self.child.is_none() {
            self.start(timeout)?;
        }
        let result = self.send(request, timeout);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn start(&mut self, timeout: Duration) -> Result<(), &'static str> {
        if !self.executable.is_file() {
            return Err("status-executable-missing");
        }
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "status-spawn")?;
        let Some(input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("status-stdin");
        };
        let Some(output) = child.stdout.take() else {
            drop(input);
            let _ = child.kill();
            let _ = child.wait();
            return Err("status-stdout");
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                let Ok(response) = serde_json::from_str::<SystemStatusHostResponse>(&line) else {
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
        match self.send(&SystemStatusHostRequest::Handshake, timeout)? {
            SystemStatusHostResponse::Handshake {
                protocol_major: 1,
                max_frame_bytes,
                max_pending_commands,
                ..
            } if max_frame_bytes <= shell_provider_protocol::MAX_FRAME_BYTES
                && max_pending_commands > 0 =>
            {
                Ok(())
            }
            _ => {
                self.reset();
                Err("status-handshake-invalid")
            }
        }
    }

    fn send(
        &mut self,
        request: &SystemStatusHostRequest,
        timeout: Duration,
    ) -> Result<SystemStatusHostResponse, &'static str> {
        request.validate().map_err(|_| "status-request-invalid")?;
        let input = self.input.as_mut().ok_or("status-input-unavailable")?;
        serde_json::to_writer(&mut *input, request).map_err(|_| "status-request-serialize")?;
        input
            .write_all(b"\n")
            .and_then(|()| input.flush())
            .map_err(|_| "status-request-write")?;
        self.responses
            .as_ref()
            .ok_or("status-response-unavailable")?
            .recv_timeout(timeout)
            .map_err(|_| "status-response-timeout")
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

impl Drop for SystemStatusClient {
    fn drop(&mut self) {
        self.reset();
    }
}

#[derive(Clone, Debug, Default)]
pub struct StatusReconciler {
    current: Option<SystemStatusSnapshot>,
    terminals: BTreeMap<String, SystemStatusCommandTerminal>,
    restart_attempts: u8,
}

impl StatusReconciler {
    pub fn snapshot(&self) -> Option<&SystemStatusSnapshot> {
        self.current.as_ref()
    }

    pub fn apply(&mut self, response: SystemStatusHostResponse) -> bool {
        match response {
            SystemStatusHostResponse::Snapshot(snapshot) => self.apply_snapshot(snapshot),
            SystemStatusHostResponse::Terminal(terminal) => self.apply_terminal(terminal),
            _ => false,
        }
    }

    pub fn provider_unavailable(&mut self) -> bool {
        let changed = self.current.take().is_some();
        self.restart_attempts = self.restart_attempts.saturating_add(1);
        changed
    }

    pub fn restart_allowed(&self) -> bool {
        self.restart_attempts < MAX_RESTART_ATTEMPTS
    }

    #[cfg(test)]
    pub fn terminal(&self, correlation_id: &str) -> Option<&SystemStatusCommandTerminal> {
        self.terminals.get(correlation_id)
    }

    fn apply_snapshot(&mut self, snapshot: SystemStatusSnapshot) -> bool {
        if snapshot.validate().is_err() {
            return false;
        }
        let accepted = self.current.as_ref().is_none_or(|current| {
            snapshot.host_generation > current.host_generation
                || (snapshot.host_generation == current.host_generation
                    && snapshot.snapshot_generation > current.snapshot_generation)
        });
        if accepted {
            if self
                .current
                .as_ref()
                .is_none_or(|current| snapshot.host_generation > current.host_generation)
            {
                self.terminals.clear();
                self.restart_attempts = 0;
            }
            self.current = Some(snapshot);
        }
        accepted
    }

    fn apply_terminal(&mut self, terminal: SystemStatusCommandTerminal) -> bool {
        if terminal.validate().is_err() || self.terminals.contains_key(&terminal.correlation_id) {
            return false;
        }
        if self
            .current
            .as_ref()
            .is_none_or(|snapshot| snapshot.host_generation != terminal.host_generation)
        {
            return false;
        }
        self.terminals
            .insert(terminal.correlation_id.clone(), terminal);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::{
        AudioStatus, ClockCalendarStatus, InputProfile, InputStatus, NetworkStatus, PowerStatus,
        StatusAvailability, SystemStatusTerminalKind,
    };

    fn snapshot(host: u64, generation: u64) -> SystemStatusSnapshot {
        SystemStatusSnapshot {
            host_generation: host,
            snapshot_generation: generation,
            network: StatusAvailability::Available(NetworkStatus {
                connected: true,
                internet: true,
                display_name: "network".into(),
            }),
            audio: StatusAvailability::Available(AudioStatus {
                endpoint_id: "audio".into(),
                volume_percent: 20,
                muted: false,
            }),
            power: StatusAvailability::Available(PowerStatus {
                ac_online: true,
                charging: false,
                battery_percent: None,
            }),
            clock: StatusAvailability::Available(ClockCalendarStatus {
                unix_ms: 1,
                locale: "en-US".into(),
                time_zone: "UTC".into(),
            }),
            input: StatusAvailability::Available(InputStatus {
                active_profile_id: "input".into(),
                profiles: vec![InputProfile {
                    id: "input".into(),
                    language_tag: "en-US".into(),
                    display_name: "English".into(),
                }],
            }),
            overflowed: false,
        }
    }

    #[test]
    fn stale_restart_and_duplicate_terminal_are_fail_closed() {
        let mut reconciler = StatusReconciler::default();
        assert!(reconciler.apply(SystemStatusHostResponse::Snapshot(snapshot(1, 2))));
        assert!(!reconciler.apply(SystemStatusHostResponse::Snapshot(snapshot(1, 1))));
        let terminal = SystemStatusCommandTerminal {
            correlation_id: "c".into(),
            host_generation: 1,
            observed_snapshot_generation: Some(2),
            terminal: SystemStatusTerminalKind::Observed,
            message: String::new(),
        };
        assert!(reconciler.apply(SystemStatusHostResponse::Terminal(terminal.clone())));
        assert!(!reconciler.apply(SystemStatusHostResponse::Terminal(terminal)));
        assert!(reconciler.provider_unavailable());
        assert!(reconciler.snapshot().is_none());
        assert!(reconciler.apply(SystemStatusHostResponse::Snapshot(snapshot(2, 1))));
        assert!(reconciler.terminal("c").is_none());
    }

    #[test]
    fn restart_attempts_are_bounded_until_a_new_full_snapshot() {
        let mut reconciler = StatusReconciler::default();
        for _ in 0..MAX_RESTART_ATTEMPTS {
            reconciler.provider_unavailable();
        }
        assert!(!reconciler.restart_allowed());
        assert!(reconciler.apply(SystemStatusHostResponse::Snapshot(snapshot(9, 1))));
        assert!(reconciler.restart_allowed());
    }
}
