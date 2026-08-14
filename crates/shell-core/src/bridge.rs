use crate::{CorrelationId, RequestId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeLaunchSource {
    DesktopFixedEntry,
    DesktopFolder,
    TaskbarFixedEntry,
    PinnedApplication,
    LifecycleProbe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeLaunchRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub source: BridgeLaunchSource,
    pub initial_path: Option<String>,
}

impl BridgeLaunchRequest {
    pub fn default_location(
        request_id: RequestId,
        correlation_id: CorrelationId,
        source: BridgeLaunchSource,
    ) -> Self {
        Self {
            request_id,
            correlation_id,
            source,
            initial_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeTerminal {
    Launched,
    ResolverUnavailable,
    SpawnRejected,
    AdmissionFailed,
    TimedOut,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeRepair {
    LocateExecutable,
    Retry,
    OpenSettings,
    None,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MessageKey {
    BridgeResolverUnavailable,
    BridgeSpawnRejected,
    BridgeAdmissionFailed,
    BridgeTimedOut,
    BridgeCancelled,
    BridgeRetry,
    BridgeLocateExecutable,
    StartUnavailable,
}

impl MessageKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BridgeResolverUnavailable => "bridge.resolver_unavailable",
            Self::BridgeSpawnRejected => "bridge.spawn_rejected",
            Self::BridgeAdmissionFailed => "bridge.admission_failed",
            Self::BridgeTimedOut => "bridge.timed_out",
            Self::BridgeCancelled => "bridge.cancelled",
            Self::BridgeRetry => "bridge.retry",
            Self::BridgeLocateExecutable => "bridge.locate_executable",
            Self::StartUnavailable => "taskbar.start_unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_launch_is_truthful_and_has_no_fake_local_path() {
        let request = BridgeLaunchRequest::default_location(
            RequestId(1),
            CorrelationId(2),
            BridgeLaunchSource::DesktopFixedEntry,
        );
        assert_eq!(request.initial_path, None);
    }

    #[test]
    fn message_keys_are_accessibility_safe_stable_tokens() {
        for key in [
            MessageKey::BridgeResolverUnavailable,
            MessageKey::BridgeSpawnRejected,
            MessageKey::BridgeAdmissionFailed,
            MessageKey::BridgeTimedOut,
            MessageKey::BridgeCancelled,
            MessageKey::BridgeRetry,
            MessageKey::BridgeLocateExecutable,
            MessageKey::StartUnavailable,
        ] {
            let value = key.as_str();
            assert!(!value.is_empty());
            assert!(
                value
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'.' || byte == b'_')
            );
        }
    }
}
