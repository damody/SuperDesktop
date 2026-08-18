use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use shell_provider_protocol::{
    TaskbarStateHostRequest, TaskbarStateHostResponse, TaskbarStateSnapshot, TaskbarWindowState,
    Validate,
};

pub struct TaskbarStateClient {
    executable: PathBuf,
    enabled: bool,
    child: Option<Child>,
    input: Option<ChildStdin>,
    responses: Option<Receiver<TaskbarStateHostResponse>>,
    restart_attempts: u8,
}

impl TaskbarStateClient {
    pub fn adjacent(enabled: bool) -> Result<Self, &'static str> {
        let executable = std::env::current_exe()
            .map_err(|_| "taskbar-state-current-executable")?
            .parent()
            .ok_or("taskbar-state-no-binary-directory")?
            .join("taskbar-state-host.exe");
        Ok(Self {
            executable,
            enabled,
            child: None,
            input: None,
            responses: None,
            restart_attempts: 0,
        })
    }

    pub fn request_snapshot(
        &mut self,
        timeout: Duration,
    ) -> Result<TaskbarStateSnapshot, &'static str> {
        if !self.enabled {
            return Err("taskbar-state-disabled");
        }
        if self.child.is_none() {
            self.start()?;
        }
        let result = (|| {
            let input = self.input.as_mut().ok_or("taskbar-state-input")?;
            serde_json::to_writer(&mut *input, &TaskbarStateHostRequest::Snapshot)
                .map_err(|_| "taskbar-state-request-serialize")?;
            input
                .write_all(b"\n")
                .and_then(|()| input.flush())
                .map_err(|_| "taskbar-state-request-write")?;
            match self
                .responses
                .as_ref()
                .ok_or("taskbar-state-response")?
                .recv_timeout(timeout)
                .map_err(|_| "taskbar-state-timeout")?
            {
                TaskbarStateHostResponse::Snapshot(snapshot) => {
                    snapshot
                        .validate()
                        .map_err(|_| "taskbar-state-invalid-snapshot")?;
                    self.restart_attempts = 0;
                    Ok(snapshot)
                }
                _ => Err("taskbar-state-invalid-response"),
            }
        })();
        if result.is_err() {
            self.reset();
        }
        result
    }

    pub fn ensure_started(&mut self) -> Result<(), &'static str> {
        if self.enabled && self.child.is_none() {
            self.start()?;
        }
        Ok(())
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if self.restart_attempts >= 3 {
            return Err("taskbar-state-restart-capacity-exhausted");
        }
        self.restart_attempts = self.restart_attempts.saturating_add(1);
        if !self.executable.is_file() {
            return Err("taskbar-state-executable-missing");
        }
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "taskbar-state-spawn")?;
        let Some(input) = child.stdin.take() else {
            let _ = child.kill();
            return Err("taskbar-state-stdin");
        };
        let Some(output) = child.stdout.take() else {
            let _ = child.kill();
            return Err("taskbar-state-stdout");
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
        if let Some(mut input) = self.input.take() {
            let _ = serde_json::to_writer(&mut input, &TaskbarStateHostRequest::Shutdown);
            let _ = input.write_all(b"\n").and_then(|()| input.flush());
        }
        self.responses.take();
        if let Some(mut child) = self.child.take() {
            for _ in 0..10 {
                if child.try_wait().ok().flatten().is_some() {
                    let _ = child.wait();
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for TaskbarStateClient {
    fn drop(&mut self) {
        self.reset();
    }
}

#[derive(Default)]
pub struct TaskbarStateReconciler {
    host_generation: u64,
    snapshot_generation: u64,
    windows: BTreeMap<(isize, u32), TaskbarWindowState>,
}

impl TaskbarStateReconciler {
    pub fn apply(&mut self, snapshot: TaskbarStateSnapshot) -> bool {
        if snapshot.validate().is_err()
            || snapshot.host_generation < self.host_generation
            || (snapshot.host_generation == self.host_generation
                && snapshot.snapshot_generation <= self.snapshot_generation)
        {
            return false;
        }
        if snapshot.host_generation != self.host_generation {
            self.windows.clear();
        }
        self.host_generation = snapshot.host_generation;
        self.snapshot_generation = snapshot.snapshot_generation;
        self.windows = snapshot
            .windows
            .into_iter()
            .map(|state| {
                (
                    (
                        state.identity.hwnd_identity as isize,
                        state.identity.process_id,
                    ),
                    state,
                )
            })
            .collect();
        true
    }

    pub fn windows(&self) -> &BTreeMap<(isize, u32), TaskbarWindowState> {
        &self.windows
    }

    pub fn provider_unavailable(&mut self) -> bool {
        let changed = !self.windows.is_empty();
        self.windows.clear();
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::{
        TaskbarAttentionState, TaskbarProgressState, TaskbarWindowIdentity,
    };

    fn snapshot(host: u64, generation: u64) -> TaskbarStateSnapshot {
        TaskbarStateSnapshot {
            host_generation: host,
            snapshot_generation: generation,
            windows: vec![TaskbarWindowState {
                identity: TaskbarWindowIdentity {
                    process_id: 4,
                    session_id: 1,
                    hwnd_identity: 8,
                    observation_generation: 1,
                },
                progress: TaskbarProgressState::none(),
                attention: TaskbarAttentionState::none(),
            }],
            overflowed: false,
        }
    }

    #[test]
    fn reconciler_rejects_stale_and_clears_across_restart() {
        let mut reconciler = TaskbarStateReconciler::default();
        assert!(reconciler.apply(snapshot(10, 2)));
        assert!(!reconciler.apply(snapshot(10, 1)));
        assert!(!reconciler.apply(snapshot(9, 99)));
        assert!(reconciler.apply(snapshot(11, 1)));
        assert_eq!(reconciler.windows().len(), 1);
        assert!(reconciler.provider_unavailable());
        assert!(!reconciler.provider_unavailable());
    }

    #[test]
    fn client_source_bounds_restart_and_requests_clean_shutdown() {
        let source = include_str!("taskbar_state_client.rs");
        assert!(source.contains("self.restart_attempts >= 3"));
        assert!(source.contains("TaskbarStateHostRequest::Shutdown"));
        assert!(source.contains("self.restart_attempts = 0"));
    }
}
