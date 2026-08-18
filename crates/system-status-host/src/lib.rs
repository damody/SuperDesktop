use std::time::{Duration, SystemTime, UNIX_EPOCH};

use shell_provider_protocol::{
    SystemStatusCommand, SystemStatusCommandRequest, SystemStatusCommandTerminal,
    SystemStatusHostHealth, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusTerminalKind, Validate,
};

pub const MAX_PENDING_COMMANDS: usize = 64;

#[derive(Debug)]
pub struct SystemStatusRuntime {
    host_generation: u64,
    snapshot_generation: u64,
    overflowed: bool,
}

impl Default for SystemStatusRuntime {
    fn default() -> Self {
        let generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |value| value.as_millis().max(1) as u64);
        Self {
            host_generation: generation,
            snapshot_generation: 0,
            overflowed: false,
        }
    }
}

impl SystemStatusRuntime {
    pub fn apply(&mut self, request: SystemStatusHostRequest) -> SystemStatusHostResponse {
        if let Err(error) = request.validate() {
            return SystemStatusHostResponse::Rejected(error.to_string());
        }
        match request {
            SystemStatusHostRequest::Handshake => SystemStatusHostResponse::Handshake {
                protocol_major: 1,
                protocol_minor: 0,
                max_frame_bytes: shell_provider_protocol::MAX_FRAME_BYTES,
                max_pending_commands: MAX_PENDING_COMMANDS,
            },
            SystemStatusHostRequest::Health => {
                SystemStatusHostResponse::Health(SystemStatusHostHealth {
                    healthy: true,
                    host_generation: self.host_generation,
                    snapshot_generation: self.snapshot_generation,
                    pending_commands: 0,
                    capacity: MAX_PENDING_COMMANDS,
                    overflowed: self.overflowed,
                })
            }
            SystemStatusHostRequest::Snapshot => self.snapshot(),
            SystemStatusHostRequest::Command { request } => self.command(request),
            SystemStatusHostRequest::Cancel { correlation_id } => {
                SystemStatusHostResponse::Terminal(SystemStatusCommandTerminal {
                    correlation_id,
                    host_generation: self.host_generation,
                    observed_snapshot_generation: None,
                    terminal: SystemStatusTerminalKind::Cancelled,
                    message: "no pending command remained".into(),
                })
            }
        }
    }

    fn snapshot(&mut self) -> SystemStatusHostResponse {
        self.snapshot_generation = self.snapshot_generation.saturating_add(1).max(1);
        match platform_win::common::system_status::system_status_snapshot(
            self.host_generation,
            self.snapshot_generation,
        ) {
            Ok(mut snapshot) => {
                snapshot.overflowed = std::mem::take(&mut self.overflowed);
                SystemStatusHostResponse::Snapshot(snapshot)
            }
            Err(error) => SystemStatusHostResponse::Rejected(error),
        }
    }

    fn command(&mut self, request: SystemStatusCommandRequest) -> SystemStatusHostResponse {
        if request.expected_host_generation != self.host_generation {
            return self.terminal(
                request.correlation_id,
                SystemStatusTerminalKind::StaleGeneration,
                None,
                "host generation changed".into(),
            );
        }
        if request.deadline_unix_ms <= unix_ms() {
            return self.terminal(
                request.correlation_id,
                SystemStatusTerminalKind::Timeout,
                None,
                "command deadline expired".into(),
            );
        }
        let result = match request.command {
            SystemStatusCommand::ActivateInputProfile { profile_id } => {
                platform_win::common::system_status::request_input_profile(
                    &profile_id,
                    Duration::from_millis(750),
                )
                .map(|_| ())
            }
            SystemStatusCommand::SetVolume { volume_percent } => {
                platform_win::common::system_status::set_volume_and_observe(volume_percent)
                    .map(|_| ())
            }
            SystemStatusCommand::SetMute { muted } => {
                platform_win::common::system_status::set_mute_and_observe(muted).map(|_| ())
            }
        };
        match result {
            Ok(()) => {
                self.snapshot_generation = self.snapshot_generation.saturating_add(1).max(1);
                self.terminal(
                    request.correlation_id,
                    SystemStatusTerminalKind::Observed,
                    Some(self.snapshot_generation),
                    String::new(),
                )
            }
            Err(message) => self.terminal(
                request.correlation_id,
                SystemStatusTerminalKind::ProviderFailure,
                None,
                message,
            ),
        }
    }

    fn terminal(
        &self,
        correlation_id: String,
        terminal: SystemStatusTerminalKind,
        observed_snapshot_generation: Option<u64>,
        message: String,
    ) -> SystemStatusHostResponse {
        SystemStatusHostResponse::Terminal(SystemStatusCommandTerminal {
            correlation_id,
            host_generation: self.host_generation,
            observed_snapshot_generation,
            terminal,
            message,
        })
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_snapshot_health_and_stale_command_are_deterministic() {
        let mut runtime = SystemStatusRuntime::default();
        assert!(matches!(
            runtime.apply(SystemStatusHostRequest::Handshake),
            SystemStatusHostResponse::Handshake { .. }
        ));
        let snapshot = runtime.apply(SystemStatusHostRequest::Snapshot);
        let SystemStatusHostResponse::Snapshot(snapshot) = snapshot else {
            panic!("missing snapshot")
        };
        snapshot.validate().unwrap();
        assert!(matches!(
            runtime.apply(SystemStatusHostRequest::Health),
            SystemStatusHostResponse::Health(_)
        ));
        let terminal = runtime.apply(SystemStatusHostRequest::Command {
            request: SystemStatusCommandRequest {
                correlation_id: "stale".into(),
                expected_host_generation: snapshot.host_generation + 1,
                deadline_unix_ms: unix_ms() + 1_000,
                command: SystemStatusCommand::SetMute { muted: false },
            },
        });
        assert!(matches!(
            terminal,
            SystemStatusHostResponse::Terminal(SystemStatusCommandTerminal {
                terminal: SystemStatusTerminalKind::StaleGeneration,
                ..
            })
        ));

        let expired = runtime.apply(SystemStatusHostRequest::Command {
            request: SystemStatusCommandRequest {
                correlation_id: "expired".into(),
                expected_host_generation: snapshot.host_generation,
                deadline_unix_ms: unix_ms().max(1),
                command: SystemStatusCommand::SetMute { muted: false },
            },
        });
        assert!(matches!(
            expired,
            SystemStatusHostResponse::Terminal(SystemStatusCommandTerminal {
                terminal: SystemStatusTerminalKind::Timeout,
                ..
            })
        ));
    }
}
