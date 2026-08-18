use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, MAX_TEXT_BYTES, Validate, ValidationError};

pub const MAX_INPUT_PROFILES: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum StatusAvailability<T> {
    Available(T),
    NotPresent,
    Unavailable { reason: String },
}

impl<T: Validate> Validate for StatusAvailability<T> {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Available(value) => value.validate(),
            Self::NotPresent => Ok(()),
            Self::Unavailable { reason } if reason.trim().is_empty() => {
                Err(ValidationError::Empty("system_status.unavailable_reason"))
            }
            Self::Unavailable { reason } if reason.len() > MAX_TEXT_BYTES => Err(
                ValidationError::TextTooLong("system_status.unavailable_reason"),
            ),
            Self::Unavailable { .. } => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub connected: bool,
    pub internet: bool,
    pub display_name: String,
}

impl Validate for NetworkStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.display_name.len() > MAX_TEXT_BYTES {
            Err(ValidationError::TextTooLong(
                "system_status.network.display_name",
            ))
        } else if self.internet && !self.connected {
            Err(ValidationError::InvalidValue(
                "system_status.network.internet",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioStatus {
    pub endpoint_id: String,
    pub volume_percent: u8,
    pub muted: bool,
}

impl Validate for AudioStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.endpoint_id, "system_status.audio.endpoint_id")?;
        if self.volume_percent > 100 {
            Err(ValidationError::OutOfRange(
                "system_status.audio.volume_percent",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PowerStatus {
    pub ac_online: bool,
    pub charging: bool,
    pub battery_percent: Option<u8>,
}

impl Validate for PowerStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.battery_percent.is_some_and(|value| value > 100) {
            Err(ValidationError::OutOfRange(
                "system_status.power.battery_percent",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClockCalendarStatus {
    pub unix_ms: u64,
    pub locale: String,
    pub time_zone: String,
}

impl Validate for ClockCalendarStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.locale, "system_status.clock.locale")?;
        validate_text(&self.time_zone, "system_status.clock.time_zone")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputProfile {
    pub id: String,
    pub language_tag: String,
    pub display_name: String,
}

impl Validate for InputProfile {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.id, "system_status.input.id")?;
        validate_text(&self.language_tag, "system_status.input.language_tag")?;
        validate_text(&self.display_name, "system_status.input.display_name")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputStatus {
    pub active_profile_id: String,
    pub profiles: Vec<InputProfile>,
}

impl Validate for InputStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(
            &self.active_profile_id,
            "system_status.input.active_profile_id",
        )?;
        if self.profiles.len() > MAX_INPUT_PROFILES || self.profiles.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge(
                "system_status.input.profiles",
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            if !ids.insert(&profile.id) {
                return Err(ValidationError::InvalidValue(
                    "system_status.input.duplicate_profile",
                ));
            }
        }
        if !ids.contains(&self.active_profile_id) {
            return Err(ValidationError::InvalidValue(
                "system_status.input.active_profile_id",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemStatusSnapshot {
    pub host_generation: u64,
    pub snapshot_generation: u64,
    pub network: StatusAvailability<NetworkStatus>,
    pub audio: StatusAvailability<AudioStatus>,
    pub power: StatusAvailability<PowerStatus>,
    pub clock: StatusAvailability<ClockCalendarStatus>,
    pub input: StatusAvailability<InputStatus>,
    pub overflowed: bool,
}

impl Validate for SystemStatusSnapshot {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.host_generation == 0 || self.snapshot_generation == 0 {
            return Err(ValidationError::OutOfRange(
                "system_status.snapshot_generation",
            ));
        }
        self.network.validate()?;
        self.audio.validate()?;
        self.power.validate()?;
        self.clock.validate()?;
        self.input.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SystemStatusCommand {
    ActivateInputProfile { profile_id: String },
    SetVolume { volume_percent: u8 },
    SetMute { muted: bool },
}

impl Validate for SystemStatusCommand {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::ActivateInputProfile { profile_id } => {
                validate_text(profile_id, "system_status.command.profile_id")
            }
            Self::SetVolume { volume_percent } if *volume_percent > 100 => Err(
                ValidationError::OutOfRange("system_status.command.volume_percent"),
            ),
            Self::SetVolume { .. } | Self::SetMute { .. } => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemStatusCommandRequest {
    pub correlation_id: String,
    pub expected_host_generation: u64,
    pub deadline_unix_ms: u64,
    pub command: SystemStatusCommand,
}

impl Validate for SystemStatusCommandRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.correlation_id, "system_status.command.correlation_id")?;
        if self.expected_host_generation == 0 || self.deadline_unix_ms == 0 {
            return Err(ValidationError::OutOfRange(
                "system_status.command.generation_or_deadline",
            ));
        }
        self.command.validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemStatusTerminalKind {
    Observed,
    Unavailable,
    Cancelled,
    Timeout,
    InvalidRequest,
    StaleGeneration,
    ProviderFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemStatusCommandTerminal {
    pub correlation_id: String,
    pub host_generation: u64,
    pub observed_snapshot_generation: Option<u64>,
    pub terminal: SystemStatusTerminalKind,
    pub message: String,
}

impl Validate for SystemStatusCommandTerminal {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(
            &self.correlation_id,
            "system_status.terminal.correlation_id",
        )?;
        if self.host_generation == 0 {
            return Err(ValidationError::OutOfRange(
                "system_status.terminal.host_generation",
            ));
        }
        if self.message.len() > MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong(
                "system_status.terminal.message",
            ));
        }
        if self.terminal == SystemStatusTerminalKind::Observed
            && self.observed_snapshot_generation.is_none()
        {
            return Err(ValidationError::InvalidValue(
                "system_status.terminal.observed_generation",
            ));
        }
        Ok(())
    }
}

fn validate_text(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::Empty(field))
    } else if value.len() > MAX_TEXT_BYTES {
        Err(ValidationError::TextTooLong(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> SystemStatusSnapshot {
        SystemStatusSnapshot {
            host_generation: 1,
            snapshot_generation: 2,
            network: StatusAvailability::Available(NetworkStatus {
                connected: true,
                internet: true,
                display_name: "Ethernet".into(),
            }),
            audio: StatusAvailability::Available(AudioStatus {
                endpoint_id: "endpoint".into(),
                volume_percent: 40,
                muted: false,
            }),
            power: StatusAvailability::NotPresent,
            clock: StatusAvailability::Available(ClockCalendarStatus {
                unix_ms: 10,
                locale: "zh-TW".into(),
                time_zone: "Taipei Standard Time".into(),
            }),
            input: StatusAvailability::Available(InputStatus {
                active_profile_id: "profile:zh-tw".into(),
                profiles: vec![InputProfile {
                    id: "profile:zh-tw".into(),
                    language_tag: "zh-TW".into(),
                    display_name: "Traditional Chinese".into(),
                }],
            }),
            overflowed: false,
        }
    }

    #[test]
    fn snapshot_round_trips_deterministically() {
        let snapshot = fixture_snapshot();
        snapshot.validate().unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();
        let decoded: SystemStatusSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, snapshot);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
    }

    #[test]
    fn bounds_duplicate_and_active_identity_fail_closed() {
        let mut snapshot = fixture_snapshot();
        if let StatusAvailability::Available(audio) = &mut snapshot.audio {
            audio.volume_percent = 101;
        }
        assert!(snapshot.validate().is_err());

        let mut snapshot = fixture_snapshot();
        if let StatusAvailability::Available(input) = &mut snapshot.input {
            input.profiles.push(input.profiles[0].clone());
        }
        assert!(snapshot.validate().is_err());

        let mut snapshot = fixture_snapshot();
        if let StatusAvailability::Available(input) = &mut snapshot.input {
            input.active_profile_id = "missing".into();
        }
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn commands_and_observed_terminals_require_bounded_identity_and_generation() {
        let request = SystemStatusCommandRequest {
            correlation_id: "request-1".into(),
            expected_host_generation: 1,
            deadline_unix_ms: 2,
            command: SystemStatusCommand::SetVolume {
                volume_percent: 100,
            },
        };
        request.validate().unwrap();
        let mut invalid = request.clone();
        invalid.command = SystemStatusCommand::SetVolume {
            volume_percent: 101,
        };
        assert!(invalid.validate().is_err());

        let terminal = SystemStatusCommandTerminal {
            correlation_id: "request-1".into(),
            host_generation: 1,
            observed_snapshot_generation: None,
            terminal: SystemStatusTerminalKind::Observed,
            message: String::new(),
        };
        assert!(terminal.validate().is_err());
    }
}
