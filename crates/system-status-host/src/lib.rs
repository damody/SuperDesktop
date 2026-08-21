use std::{
    collections::BTreeSet,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use shell_provider_protocol::{
    SystemStatusCommand, SystemStatusCommandRequest, SystemStatusCommandTerminal,
    SystemStatusHostHealth, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusTerminalKind, Validate,
};

pub const MAX_PENDING_COMMANDS: usize = 64;
pub const MAX_PENDING_PROVIDER_EVENTS: usize = 4;
const AUTHORITATIVE_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderEvent {
    Input,
    Audio,
    Network,
    Power,
    Clock,
}

pub struct ProviderCallbackRegistration {
    accepting: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for ProviderCallbackRegistration {
    fn drop(&mut self) {
        self.accepting.store(false, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn register_provider_callbacks() -> (Receiver<ProviderEvent>, ProviderCallbackRegistration) {
    let (sender, receiver) = sync_channel(MAX_PENDING_PROVIDER_EVENTS * 2);
    let accepting = Arc::new(AtomicBool::new(true));
    let worker_accepting = Arc::clone(&accepting);
    let worker = thread::spawn(move || provider_callback_loop(worker_accepting, sender));
    (
        receiver,
        ProviderCallbackRegistration {
            accepting,
            worker: Some(worker),
        },
    )
}

fn provider_callback_loop(accepting: Arc<AtomicBool>, sender: SyncSender<ProviderEvent>) {
    let mut previous = provider_fingerprints();
    while accepting.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(200));
        if !accepting.load(Ordering::Acquire) {
            break;
        }
        let Some(current) = catch_unwind(AssertUnwindSafe(provider_fingerprints)).ok() else {
            continue;
        };
        for (index, event) in [
            ProviderEvent::Input,
            ProviderEvent::Audio,
            ProviderEvent::Network,
            ProviderEvent::Power,
            ProviderEvent::Clock,
        ]
        .into_iter()
        .enumerate()
        {
            if current[index] != previous[index] {
                let _ = sender.try_send(event);
            }
        }
        previous = current;
    }
}

fn provider_fingerprints() -> [String; 5] {
    [
        format!("{:?}", platform_win::common::system_status::input_status()),
        format!("{:?}", platform_win::common::system_status::audio_status()),
        format!(
            "{:?}",
            platform_win::common::system_status::network_status()
        ),
        format!("{:?}", platform_win::common::system_status::power_status()),
        format!(
            "{:?}",
            platform_win::common::system_status::clock_calendar_status()
        ),
    ]
}

#[derive(Clone, Debug)]
pub struct CallbackFence {
    accepting: Arc<AtomicBool>,
}

impl Default for CallbackFence {
    fn default() -> Self {
        Self {
            accepting: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl CallbackFence {
    pub fn invoke<T>(&self, callback: impl FnOnce() -> T) -> Option<T> {
        if !self.accepting.load(Ordering::Acquire) {
            return None;
        }
        catch_unwind(AssertUnwindSafe(callback)).ok()
    }

    pub fn shutdown(&self) {
        self.accepting.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
struct ProviderEventQueue {
    pending: BTreeSet<ProviderEvent>,
    overflowed: bool,
    authoritative_pending: bool,
    last_authoritative: Instant,
}

impl Default for ProviderEventQueue {
    fn default() -> Self {
        Self {
            pending: BTreeSet::new(),
            overflowed: false,
            authoritative_pending: true,
            last_authoritative: Instant::now(),
        }
    }
}

impl ProviderEventQueue {
    fn push(&mut self, event: ProviderEvent) {
        if self.pending.contains(&event) {
            return;
        }
        if self.pending.len() == MAX_PENDING_PROVIDER_EVENTS {
            self.overflowed = true;
            self.authoritative_pending = true;
            return;
        }
        self.pending.insert(event);
    }

    fn schedule_if_due(&mut self, now: Instant) {
        if now.duration_since(self.last_authoritative) >= AUTHORITATIVE_RECONCILIATION_INTERVAL {
            self.authoritative_pending = true;
        }
    }

    fn take_for_snapshot(&mut self, now: Instant) -> bool {
        self.schedule_if_due(now);
        let overflowed = std::mem::take(&mut self.overflowed);
        self.pending.clear();
        self.authoritative_pending = false;
        self.last_authoritative = now;
        overflowed
    }
}

#[derive(Debug)]
pub struct SystemStatusRuntime {
    host_generation: u64,
    snapshot_generation: u64,
    overflowed: bool,
    callback_fence: CallbackFence,
    events: ProviderEventQueue,
    callback_events: Option<Receiver<ProviderEvent>>,
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
            callback_fence: CallbackFence::default(),
            events: ProviderEventQueue::default(),
            callback_events: None,
        }
    }
}

impl SystemStatusRuntime {
    pub fn attach_provider_callbacks(&mut self, events: Receiver<ProviderEvent>) {
        self.callback_events = Some(events);
    }

    fn drain_provider_callbacks(&mut self) {
        let Some(receiver) = &self.callback_events else {
            return;
        };
        while let Ok(event) = receiver.try_recv() {
            self.events.push(event);
        }
    }

    pub fn provider_callback(&mut self, callback: impl FnOnce() -> ProviderEvent) -> bool {
        let Some(event) = self.callback_fence.invoke(callback) else {
            return false;
        };
        self.events.push(event);
        true
    }

    pub fn shutdown_callbacks(&self) {
        self.callback_fence.shutdown();
    }

    pub fn apply(&mut self, request: SystemStatusHostRequest) -> SystemStatusHostResponse {
        self.drain_provider_callbacks();
        self.events.schedule_if_due(Instant::now());
        if let Err(error) = request.validate() {
            return SystemStatusHostResponse::Rejected(error.to_string());
        }
        match request {
            SystemStatusHostRequest::Handshake => SystemStatusHostResponse::Handshake {
                protocol_major: 1,
                protocol_minor: 2,
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
                    overflowed: self.overflowed || self.events.overflowed,
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
        self.overflowed |= self.events.take_for_snapshot(Instant::now());
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
        let remaining =
            Duration::from_millis(request.deadline_unix_ms.saturating_sub(unix_ms()).max(1));
        let (result, accepted_event, accepted_message) = match request.command {
            SystemStatusCommand::ActivateInputProfile { profile_id } => (
                platform_win::common::system_status::request_input_profile(
                    &profile_id,
                    remaining.min(Duration::from_secs(5)),
                )
                .map(|_| ()),
                None,
                "",
            ),
            SystemStatusCommand::OpenLanguagePreferences => (
                platform_win::common::system_status::open_language_preferences(),
                Some(None),
                "Language preferences launch accepted",
            ),
            SystemStatusCommand::SetVolume { volume_percent } => (
                platform_win::common::system_status::set_volume_and_observe(volume_percent)
                    .map(|_| ()),
                None,
                "",
            ),
            SystemStatusCommand::SetMute { muted } => (
                platform_win::common::system_status::set_mute_and_observe(muted).map(|_| ()),
                None,
                "",
            ),
            SystemStatusCommand::RefreshWifi => (
                platform_win::common::system_status::refresh_wifi(),
                Some(Some(ProviderEvent::Network)),
                "WLAN request accepted; awaiting authoritative snapshot",
            ),
            SystemStatusCommand::ConnectWifi {
                interface_id,
                profile_name,
            } => (
                platform_win::common::system_status::connect_wifi_profile(
                    &interface_id,
                    &profile_name,
                ),
                Some(Some(ProviderEvent::Network)),
                "WLAN request accepted; awaiting authoritative snapshot",
            ),
            SystemStatusCommand::DisconnectWifi { interface_id } => (
                platform_win::common::system_status::disconnect_wifi(&interface_id),
                Some(Some(ProviderEvent::Network)),
                "WLAN request accepted; awaiting authoritative snapshot",
            ),
        };
        match result {
            Ok(()) => {
                if let Some(event) = accepted_event {
                    if let Some(event) = event {
                        self.events.push(event);
                    }
                    self.terminal(
                        request.correlation_id,
                        SystemStatusTerminalKind::Accepted,
                        None,
                        accepted_message.into(),
                    )
                } else {
                    self.snapshot_generation = self.snapshot_generation.saturating_add(1).max(1);
                    self.terminal(
                        request.correlation_id,
                        SystemStatusTerminalKind::Observed,
                        Some(self.snapshot_generation),
                        String::new(),
                    )
                }
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

impl Drop for SystemStatusRuntime {
    fn drop(&mut self) {
        self.shutdown_callbacks();
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

    #[test]
    fn generation_mismatch_is_rejected_before_any_platform_command_dispatch() {
        let source = include_str!("lib.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let gate = production
            .find("if request.expected_host_generation != self.host_generation")
            .expect("generation admission gate");
        let dispatch = production
            .find("let (result, accepted_event, accepted_message) = match request.command")
            .expect("platform command dispatch");
        let volume = production
            .find("set_volume_and_observe(volume_percent)")
            .expect("Core Audio volume dispatch");
        let mute = production
            .find("set_mute_and_observe(muted)")
            .expect("Core Audio mute dispatch");
        assert!(gate < dispatch && dispatch < volume && dispatch < mute);
    }

    #[test]
    fn callback_panic_shutdown_coalescing_and_overflow_are_bounded() {
        let mut runtime = SystemStatusRuntime::default();
        assert!(runtime.provider_callback(|| ProviderEvent::Audio));
        assert!(runtime.provider_callback(|| ProviderEvent::Audio));
        assert!(!runtime.provider_callback(|| panic!("fixture callback panic")));
        for event in [
            ProviderEvent::Input,
            ProviderEvent::Network,
            ProviderEvent::Power,
            ProviderEvent::Clock,
        ] {
            runtime.provider_callback(|| event);
        }
        assert!(matches!(
            runtime.apply(SystemStatusHostRequest::Health),
            SystemStatusHostResponse::Health(SystemStatusHostHealth {
                overflowed: true,
                ..
            })
        ));
        let snapshot = runtime.apply(SystemStatusHostRequest::Snapshot);
        assert!(matches!(
            snapshot,
            SystemStatusHostResponse::Snapshot(shell_provider_protocol::SystemStatusSnapshot {
                overflowed: true,
                ..
            })
        ));
        runtime.shutdown_callbacks();
        assert!(!runtime.provider_callback(|| ProviderEvent::Audio));
    }

    #[test]
    fn authoritative_reconciliation_timer_is_single_flight() {
        let mut runtime = SystemStatusRuntime::default();
        runtime.events.authoritative_pending = false;
        runtime.events.last_authoritative = Instant::now() - AUTHORITATIVE_RECONCILIATION_INTERVAL;
        let _ = runtime.apply(SystemStatusHostRequest::Health);
        assert!(runtime.events.authoritative_pending);
        let _ = runtime.apply(SystemStatusHostRequest::Snapshot);
        assert!(!runtime.events.authoritative_pending);
    }

    #[test]
    fn wifi_refresh_returns_accepted_or_truthful_provider_failure_and_routes_network_event() {
        let mut runtime = SystemStatusRuntime::default();
        let SystemStatusHostResponse::Handshake {
            protocol_major,
            protocol_minor,
            ..
        } = runtime.apply(SystemStatusHostRequest::Handshake)
        else {
            panic!("handshake")
        };
        assert_eq!(protocol_major, 1);
        assert_eq!(protocol_minor, 2);
        let SystemStatusHostResponse::Snapshot(snapshot) =
            runtime.apply(SystemStatusHostRequest::Snapshot)
        else {
            panic!("snapshot")
        };
        let response = runtime.apply(SystemStatusHostRequest::Command {
            request: SystemStatusCommandRequest {
                correlation_id: "wifi-refresh".into(),
                expected_host_generation: snapshot.host_generation,
                deadline_unix_ms: unix_ms() + 1_000,
                command: SystemStatusCommand::RefreshWifi,
            },
        });
        let SystemStatusHostResponse::Terminal(terminal) = response else {
            panic!("terminal")
        };
        assert!(matches!(
            terminal.terminal,
            SystemStatusTerminalKind::Accepted | SystemStatusTerminalKind::ProviderFailure
        ));
        if terminal.terminal == SystemStatusTerminalKind::Accepted {
            assert_eq!(terminal.observed_snapshot_generation, None);
            assert!(runtime.events.pending.contains(&ProviderEvent::Network));
        }
    }

    #[test]
    fn wifi_host_source_routes_only_typed_identity_bound_commands() {
        let source = include_str!("lib.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "SystemStatusCommand::RefreshWifi",
            "SystemStatusCommand::ConnectWifi",
            "SystemStatusCommand::DisconnectWifi",
            "SystemStatusTerminalKind::Accepted",
            "Some(Some(ProviderEvent::Network))",
            "connect_wifi_profile",
            "disconnect_wifi",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        assert!(!production.contains("WlanConnect"));
        assert!(!production.contains("profile xml"));
    }

    #[test]
    fn language_preferences_route_is_fieldless_fixed_and_accepted_without_observation() {
        let source = include_str!("lib.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "SystemStatusCommand::OpenLanguagePreferences",
            "open_language_preferences()",
            "Some(None)",
            "Language preferences launch accepted",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        for forbidden in ["explorer.exe", "uri:", "executable:", "arguments:"] {
            assert!(!production.contains(forbidden), "forbidden {forbidden}");
        }
    }
}
