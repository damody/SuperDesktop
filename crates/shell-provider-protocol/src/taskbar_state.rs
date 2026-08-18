use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, MAX_TEXT_BYTES, Validate, ValidationError};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TaskbarWindowIdentity {
    pub process_id: u32,
    pub session_id: u32,
    pub hwnd_identity: i64,
    pub observation_generation: u64,
}

impl Validate for TaskbarWindowIdentity {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.process_id == 0 || self.hwnd_identity == 0 || self.observation_generation == 0 {
            Err(ValidationError::OutOfRange("taskbar_state.window_identity"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarProgressKind {
    None,
    Indeterminate,
    Normal,
    Paused,
    Error,
}

impl TaskbarProgressKind {
    pub const fn group_priority(self) -> u8 {
        match self {
            Self::Error => 4,
            Self::Paused => 3,
            Self::Normal => 2,
            Self::Indeterminate => 1,
            Self::None => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskbarProgressState {
    pub kind: TaskbarProgressKind,
    pub completed: u64,
    pub total: u64,
}

impl TaskbarProgressState {
    pub const fn none() -> Self {
        Self {
            kind: TaskbarProgressKind::None,
            completed: 0,
            total: 0,
        }
    }

    pub fn permille(self) -> Result<u16, ValidationError> {
        self.validate()?;
        match self.kind {
            TaskbarProgressKind::None | TaskbarProgressKind::Indeterminate => Ok(0),
            _ => {
                let scaled = u128::from(self.completed)
                    .checked_mul(1000)
                    .ok_or(ValidationError::OutOfRange("taskbar_state.progress_ratio"))?;
                Ok((scaled / u128::from(self.total)).min(1000) as u16)
            }
        }
    }
}

impl Validate for TaskbarProgressState {
    fn validate(&self) -> Result<(), ValidationError> {
        match self.kind {
            TaskbarProgressKind::None | TaskbarProgressKind::Indeterminate
                if self.completed == 0 && self.total == 0 =>
            {
                Ok(())
            }
            TaskbarProgressKind::Normal
            | TaskbarProgressKind::Paused
            | TaskbarProgressKind::Error
                if self.total > 0 && self.completed <= self.total =>
            {
                Ok(())
            }
            _ => Err(ValidationError::OutOfRange("taskbar_state.progress")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarAttentionMode {
    None,
    Finite,
    UntilForeground,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskbarAttentionState {
    pub mode: TaskbarAttentionMode,
    pub cadence_ms: u32,
    pub flashes_remaining: u16,
    pub phase_on: bool,
    pub steady: bool,
}

impl TaskbarAttentionState {
    pub const fn none() -> Self {
        Self {
            mode: TaskbarAttentionMode::None,
            cadence_ms: 0,
            flashes_remaining: 0,
            phase_on: false,
            steady: false,
        }
    }

    pub fn tick(&mut self, became_foreground: bool) -> bool {
        if became_foreground || self.mode == TaskbarAttentionMode::None {
            let changed = *self != Self::none();
            *self = Self::none();
            return changed;
        }
        if self.mode == TaskbarAttentionMode::Finite && self.flashes_remaining == 0 {
            let changed = !self.steady || self.phase_on;
            self.phase_on = false;
            self.steady = true;
            return changed;
        }
        self.phase_on = !self.phase_on;
        if self.mode == TaskbarAttentionMode::Finite && !self.phase_on {
            self.flashes_remaining = self.flashes_remaining.saturating_sub(1);
        }
        true
    }
}

impl Validate for TaskbarAttentionState {
    fn validate(&self) -> Result<(), ValidationError> {
        match self.mode {
            TaskbarAttentionMode::None if *self == Self::none() => Ok(()),
            TaskbarAttentionMode::Finite if (1..=60_000).contains(&self.cadence_ms) => Ok(()),
            TaskbarAttentionMode::UntilForeground
                if (1..=60_000).contains(&self.cadence_ms) && self.flashes_remaining == 0 =>
            {
                Ok(())
            }
            _ => Err(ValidationError::OutOfRange("taskbar_state.attention")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskbarWindowState {
    pub identity: TaskbarWindowIdentity,
    pub progress: TaskbarProgressState,
    pub attention: TaskbarAttentionState,
}

impl TaskbarWindowState {
    pub fn clear_attention(&mut self) {
        self.attention = TaskbarAttentionState::none();
    }

    pub fn provider_unavailable(&mut self) {
        self.progress = TaskbarProgressState::none();
        self.attention = TaskbarAttentionState::none();
    }
}

impl Validate for TaskbarWindowState {
    fn validate(&self) -> Result<(), ValidationError> {
        self.identity.validate()?;
        self.progress.validate()?;
        self.attention.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskbarStateSnapshot {
    pub host_generation: u64,
    pub snapshot_generation: u64,
    pub windows: Vec<TaskbarWindowState>,
    pub overflowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarStateHostRequest {
    Snapshot,
    Health,
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TaskbarStateHostResponse {
    Snapshot(TaskbarStateSnapshot),
    Health {
        host_generation: u64,
        provider_available: bool,
    },
    Shutdown,
    InvalidRequest,
}

impl Validate for TaskbarStateSnapshot {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.host_generation == 0 || self.snapshot_generation == 0 {
            return Err(ValidationError::OutOfRange(
                "taskbar_state.snapshot_generation",
            ));
        }
        if self.windows.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("taskbar_state.windows"));
        }
        let mut identities = std::collections::BTreeSet::new();
        for state in &self.windows {
            state.validate()?;
            if !identities.insert(&state.identity) {
                return Err(ValidationError::InvalidValue(
                    "taskbar_state.duplicate_window",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarStateTerminalKind {
    Applied,
    NoChange,
    InvalidRequest,
    StaleGeneration,
    Timeout,
    Cancelled,
    ProviderFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskbarStateTerminal {
    pub correlation_id: String,
    pub host_generation: u64,
    pub terminal: TaskbarStateTerminalKind,
    pub message: String,
}

impl Validate for TaskbarStateTerminal {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.correlation_id.trim().is_empty() || self.host_generation == 0 {
            return Err(ValidationError::InvalidValue(
                "taskbar_state.terminal_identity",
            ));
        }
        if self.message.len() > MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong(
                "taskbar_state.terminal_message",
            ));
        }
        Ok(())
    }
}

pub fn reduce_group_progress(
    states: impl IntoIterator<Item = TaskbarProgressState>,
) -> TaskbarProgressState {
    states
        .into_iter()
        .filter(|state| state.validate().is_ok() && state.kind != TaskbarProgressKind::None)
        .max_by(|left, right| {
            left.kind
                .group_priority()
                .cmp(&right.kind.group_priority())
                .then_with(|| {
                    let left_permille = left.permille().unwrap_or(0);
                    let right_permille = right.permille().unwrap_or(0);
                    right_permille.cmp(&left_permille)
                })
        })
        .unwrap_or_else(TaskbarProgressState::none)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(kind: TaskbarProgressKind, completed: u64, total: u64) -> TaskbarProgressState {
        TaskbarProgressState {
            kind,
            completed,
            total,
        }
    }

    #[test]
    fn checked_ratio_handles_maximum_values_and_rejects_zero_total() {
        assert_eq!(
            progress(TaskbarProgressKind::Normal, u64::MAX, u64::MAX).permille(),
            Ok(1000)
        );
        assert!(
            progress(TaskbarProgressKind::Normal, 1, 0)
                .validate()
                .is_err()
        );
        assert!(
            progress(TaskbarProgressKind::Normal, 2, 1)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn group_priority_and_least_progress_match_windows() {
        assert_eq!(
            reduce_group_progress([
                progress(TaskbarProgressKind::Indeterminate, 0, 0),
                progress(TaskbarProgressKind::Normal, 80, 100),
                progress(TaskbarProgressKind::Normal, 20, 100),
                progress(TaskbarProgressKind::Paused, 60, 100),
                progress(TaskbarProgressKind::Error, 90, 100),
            ]),
            progress(TaskbarProgressKind::Error, 90, 100)
        );
        assert_eq!(
            reduce_group_progress([
                progress(TaskbarProgressKind::Normal, 80, 100),
                progress(TaskbarProgressKind::Normal, 20, 100),
            ]),
            progress(TaskbarProgressKind::Normal, 20, 100)
        );
    }

    #[test]
    fn finite_and_until_foreground_attention_terminate_deterministically() {
        let mut finite = TaskbarAttentionState {
            mode: TaskbarAttentionMode::Finite,
            cadence_ms: 500,
            flashes_remaining: 1,
            phase_on: false,
            steady: false,
        };
        assert!(finite.tick(false));
        assert!(finite.phase_on);
        finite.tick(false);
        assert_eq!(finite.flashes_remaining, 0);
        finite.tick(false);
        assert!(finite.steady);
        finite.tick(true);
        assert_eq!(finite, TaskbarAttentionState::none());
        let mut until = TaskbarAttentionState {
            mode: TaskbarAttentionMode::UntilForeground,
            cadence_ms: 500,
            flashes_remaining: 0,
            phase_on: false,
            steady: false,
        };
        until.tick(false);
        assert!(until.phase_on);
        until.tick(true);
        assert_eq!(until, TaskbarAttentionState::none());
    }

    #[test]
    fn snapshot_round_trip_rejects_duplicate_and_stale_identity_fields() {
        let identity = TaskbarWindowIdentity {
            process_id: 1,
            session_id: 1,
            hwnd_identity: 2,
            observation_generation: 1,
        };
        let state = TaskbarWindowState {
            identity,
            progress: progress(TaskbarProgressKind::Normal, 1, 2),
            attention: TaskbarAttentionState::none(),
        };
        let snapshot = TaskbarStateSnapshot {
            host_generation: 1,
            snapshot_generation: 1,
            windows: vec![state.clone()],
            overflowed: false,
        };
        snapshot.validate().unwrap();
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        assert_eq!(
            serde_json::from_slice::<TaskbarStateSnapshot>(&bytes).unwrap(),
            snapshot
        );
        let duplicate = TaskbarStateSnapshot {
            windows: vec![state.clone(), state],
            ..snapshot
        };
        assert!(duplicate.validate().is_err());
    }

    #[test]
    fn host_requests_use_bounded_lowercase_json_frames() {
        let encoded = serde_json::to_string(&TaskbarStateHostRequest::Snapshot).unwrap();
        assert_eq!(encoded, "\"snapshot\"");
        assert_eq!(
            serde_json::from_str::<TaskbarStateHostRequest>(&encoded).unwrap(),
            TaskbarStateHostRequest::Snapshot
        );
    }

    #[test]
    fn activation_close_restart_and_hwnd_reuse_clear_only_authorized_fields() {
        let mut state = TaskbarWindowState {
            identity: TaskbarWindowIdentity {
                process_id: 1,
                session_id: 1,
                hwnd_identity: 2,
                observation_generation: 7,
            },
            progress: progress(TaskbarProgressKind::Normal, 40, 100),
            attention: TaskbarAttentionState {
                mode: TaskbarAttentionMode::UntilForeground,
                cadence_ms: 500,
                flashes_remaining: 0,
                phase_on: true,
                steady: false,
            },
        };
        state.clear_attention();
        assert_eq!(
            state.progress,
            progress(TaskbarProgressKind::Normal, 40, 100)
        );
        assert_eq!(state.attention, TaskbarAttentionState::none());
        let reused = TaskbarWindowIdentity {
            observation_generation: 8,
            ..state.identity.clone()
        };
        assert_ne!(state.identity, reused);
        state.provider_unavailable();
        assert_eq!(state.progress, TaskbarProgressState::none());
        assert_eq!(state.attention, TaskbarAttentionState::none());
    }
}
