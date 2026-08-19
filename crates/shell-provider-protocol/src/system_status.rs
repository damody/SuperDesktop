use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, MAX_TEXT_BYTES, Validate, ValidationError};

pub const MAX_INPUT_PROFILES: usize = 64;
pub const MAX_WIFI_NETWORKS: usize = 64;

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
    #[serde(default = "legacy_wifi_unavailable")]
    pub wifi: StatusAvailability<WifiStatus>,
}

fn legacy_wifi_unavailable() -> StatusAvailability<WifiStatus> {
    StatusAvailability::Unavailable {
        reason: "Wi-Fi status was not supplied by this provider".into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub interface_id: String,
    pub ssid: String,
    pub profile_name: Option<String>,
    pub signal_quality: u8,
    pub secure: bool,
    pub connected: bool,
    pub connectable: bool,
}

impl Validate for WifiNetwork {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.interface_id, "system_status.wifi.interface_id")?;
        validate_text(&self.ssid, "system_status.wifi.ssid")?;
        if let Some(profile_name) = &self.profile_name {
            validate_text(profile_name, "system_status.wifi.profile_name")?;
        }
        if self.signal_quality > 100 {
            Err(ValidationError::OutOfRange(
                "system_status.wifi.signal_quality",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WifiStatus {
    pub enabled: bool,
    pub networks: Vec<WifiNetwork>,
}

impl Validate for WifiStatus {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.networks.len() > MAX_WIFI_NETWORKS || self.networks.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge(
                "system_status.wifi.networks",
            ));
        }
        let mut ssids = std::collections::BTreeSet::new();
        for network in &self.networks {
            network.validate()?;
            if !ssids.insert(network.ssid.as_str()) {
                return Err(ValidationError::InvalidValue(
                    "system_status.wifi.duplicate_ssid",
                ));
            }
        }
        Ok(())
    }
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
            self.wifi.validate()
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputProfileKind {
    #[default]
    LegacyKeyboardLayout,
    KeyboardLayout,
    InputProcessor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputProfile {
    pub id: String,
    pub language_tag: String,
    pub display_name: String,
    #[serde(default)]
    pub input_method_name: String,
    #[serde(default)]
    pub kind: InputProfileKind,
    #[serde(default)]
    pub language_id: u16,
    #[serde(default)]
    pub tsf_class_id: Option<String>,
    #[serde(default)]
    pub tsf_profile_id: Option<String>,
    #[serde(default)]
    pub hkl: Option<String>,
}

impl Validate for InputProfile {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text(&self.id, "system_status.input.id")?;
        validate_text(&self.language_tag, "system_status.input.language_tag")?;
        validate_text(&self.display_name, "system_status.input.display_name")?;
        if !self.input_method_name.is_empty() {
            validate_text(
                &self.input_method_name,
                "system_status.input.input_method_name",
            )?;
        }
        match self.kind {
            InputProfileKind::LegacyKeyboardLayout => Ok(()),
            InputProfileKind::KeyboardLayout => {
                if self.input_method_name.trim().is_empty() {
                    return Err(ValidationError::Empty(
                        "system_status.input.input_method_name",
                    ));
                }
                if self.language_id == 0
                    || self.tsf_class_id.is_some()
                    || self.tsf_profile_id.is_some()
                {
                    return Err(ValidationError::InvalidValue(
                        "system_status.input.keyboard_identity",
                    ));
                }
                let hkl = self.hkl.as_deref().ok_or(ValidationError::InvalidValue(
                    "system_status.input.keyboard_hkl",
                ))?;
                validate_fixed_hex(hkl, 16, "system_status.input.keyboard_hkl")?;
                let expected = format!("input:v1:kbd:{:04x}:{hkl}", self.language_id);
                if self.id == expected {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidValue(
                        "system_status.input.keyboard_id",
                    ))
                }
            }
            InputProfileKind::InputProcessor => {
                if self.language_id == 0 || self.input_method_name.trim().is_empty() {
                    return Err(ValidationError::InvalidValue(
                        "system_status.input.processor_language",
                    ));
                }
                let class_id =
                    self.tsf_class_id
                        .as_deref()
                        .ok_or(ValidationError::InvalidValue(
                            "system_status.input.processor_class",
                        ))?;
                let profile_id =
                    self.tsf_profile_id
                        .as_deref()
                        .ok_or(ValidationError::InvalidValue(
                            "system_status.input.processor_profile",
                        ))?;
                validate_fixed_hex(class_id, 32, "system_status.input.processor_class")?;
                validate_fixed_hex(profile_id, 32, "system_status.input.processor_profile")?;
                let hkl = self.hkl.as_deref().unwrap_or("none");
                if hkl != "none" {
                    validate_fixed_hex(hkl, 16, "system_status.input.processor_hkl")?;
                }
                let expected = format!(
                    "input:v1:tip:{:04x}:{class_id}:{profile_id}:{hkl}",
                    self.language_id
                );
                if self.id == expected {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidValue(
                        "system_status.input.processor_id",
                    ))
                }
            }
        }
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
    ActivateInputProfile {
        profile_id: String,
    },
    OpenLanguagePreferences,
    SetVolume {
        volume_percent: u8,
    },
    SetMute {
        muted: bool,
    },
    RefreshWifi,
    ConnectWifi {
        interface_id: String,
        profile_name: String,
    },
    DisconnectWifi {
        interface_id: String,
    },
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
            Self::ConnectWifi {
                interface_id,
                profile_name,
            } => {
                validate_text(interface_id, "system_status.command.wifi_interface_id")?;
                validate_text(profile_name, "system_status.command.wifi_profile_name")
            }
            Self::DisconnectWifi { interface_id } => {
                validate_text(interface_id, "system_status.command.wifi_interface_id")
            }
            Self::SetVolume { .. }
            | Self::SetMute { .. }
            | Self::RefreshWifi
            | Self::OpenLanguagePreferences => Ok(()),
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
    Accepted,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SystemStatusHostRequest {
    Handshake,
    Health,
    Snapshot,
    Command { request: SystemStatusCommandRequest },
    Cancel { correlation_id: String },
}

impl Validate for SystemStatusHostRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Command { request } => request.validate(),
            Self::Cancel { correlation_id } => {
                validate_text(correlation_id, "system_status.cancel.correlation_id")
            }
            Self::Handshake | Self::Health | Self::Snapshot => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemStatusHostHealth {
    pub healthy: bool,
    pub host_generation: u64,
    pub snapshot_generation: u64,
    pub pending_commands: usize,
    pub capacity: usize,
    pub overflowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SystemStatusHostResponse {
    Handshake {
        protocol_major: u16,
        protocol_minor: u16,
        max_frame_bytes: usize,
        max_pending_commands: usize,
    },
    Health(SystemStatusHostHealth),
    Snapshot(SystemStatusSnapshot),
    Terminal(SystemStatusCommandTerminal),
    Rejected(String),
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
        match self.terminal {
            SystemStatusTerminalKind::Observed if self.observed_snapshot_generation.is_none() => {
                return Err(ValidationError::InvalidValue(
                    "system_status.terminal.observed_generation",
                ));
            }
            SystemStatusTerminalKind::Accepted if self.observed_snapshot_generation.is_some() => {
                return Err(ValidationError::InvalidValue(
                    "system_status.terminal.accepted_generation",
                ));
            }
            _ => {}
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

fn validate_fixed_hex(
    value: &str,
    length: usize,
    field: &'static str,
) -> Result<(), ValidationError> {
    if value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ValidationError::InvalidValue(field))
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
                wifi: StatusAvailability::Available(WifiStatus {
                    enabled: true,
                    networks: vec![WifiNetwork {
                        interface_id: "interface-1".into(),
                        ssid: "network-1".into(),
                        profile_name: Some("profile-1".into()),
                        signal_quality: 80,
                        secure: true,
                        connected: true,
                        connectable: true,
                    }],
                }),
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
                    input_method_name: String::new(),
                    kind: InputProfileKind::LegacyKeyboardLayout,
                    language_id: 0,
                    tsf_class_id: None,
                    tsf_profile_id: None,
                    hkl: None,
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

        let host_request = SystemStatusHostRequest::Command { request };
        host_request.validate().unwrap();
        let json = serde_json::to_string(&host_request).unwrap();
        let decoded: SystemStatusHostRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, host_request);
    }

    #[test]
    fn wifi_snapshot_is_additive_bounded_and_rejects_duplicate_ssids() {
        let legacy: NetworkStatus =
            serde_json::from_str(r#"{"connected":true,"internet":true,"display_name":"legacy"}"#)
                .unwrap();
        assert!(matches!(
            legacy.wifi,
            StatusAvailability::Unavailable { .. }
        ));

        let snapshot = fixture_snapshot();
        snapshot.validate().unwrap();
        let encoded = serde_json::to_string(&snapshot).unwrap();
        assert_eq!(
            serde_json::from_str::<SystemStatusSnapshot>(&encoded).unwrap(),
            snapshot
        );

        let mut duplicate = fixture_snapshot();
        if let StatusAvailability::Available(network) = &mut duplicate.network
            && let StatusAvailability::Available(wifi) = &mut network.wifi
        {
            wifi.networks.push(wifi.networks[0].clone());
        }
        assert!(duplicate.validate().is_err());

        let mut oversized = fixture_snapshot();
        if let StatusAvailability::Available(network) = &mut oversized.network
            && let StatusAvailability::Available(wifi) = &mut network.wifi
        {
            wifi.networks.clear();
            for index in 0..=MAX_WIFI_NETWORKS {
                wifi.networks.push(WifiNetwork {
                    interface_id: "interface".into(),
                    ssid: format!("network-{index}"),
                    profile_name: None,
                    signal_quality: 50,
                    secure: true,
                    connected: false,
                    connectable: false,
                });
            }
        }
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn wifi_commands_and_accepted_terminal_require_exact_bounded_identity() {
        for command in [
            SystemStatusCommand::RefreshWifi,
            SystemStatusCommand::ConnectWifi {
                interface_id: "interface-1".into(),
                profile_name: "profile-1".into(),
            },
            SystemStatusCommand::DisconnectWifi {
                interface_id: "interface-1".into(),
            },
        ] {
            command.validate().unwrap();
        }
        assert!(
            SystemStatusCommand::ConnectWifi {
                interface_id: String::new(),
                profile_name: "profile".into(),
            }
            .validate()
            .is_err()
        );
        assert!(
            SystemStatusCommand::DisconnectWifi {
                interface_id: "x".repeat(MAX_TEXT_BYTES + 1),
            }
            .validate()
            .is_err()
        );

        SystemStatusCommandTerminal {
            correlation_id: "wifi".into(),
            host_generation: 1,
            observed_snapshot_generation: None,
            terminal: SystemStatusTerminalKind::Accepted,
            message: "WLAN request accepted".into(),
        }
        .validate()
        .unwrap();
        assert!(
            SystemStatusCommandTerminal {
                correlation_id: "wifi".into(),
                host_generation: 1,
                observed_snapshot_generation: Some(2),
                terminal: SystemStatusTerminalKind::Accepted,
                message: String::new(),
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn input_profiles_are_additive_exact_and_bounded() {
        let legacy: InputProfile = serde_json::from_str(
            r#"{"id":"hkl:0000000000000409","language_tag":"en-US","display_name":"English"}"#,
        )
        .unwrap();
        assert_eq!(legacy.kind, InputProfileKind::LegacyKeyboardLayout);
        legacy.validate().unwrap();

        let keyboard = InputProfile {
            id: "input:v1:kbd:0409:0000000000000409".into(),
            language_tag: "en-US".into(),
            display_name: "English (United States)".into(),
            input_method_name: "US keyboard".into(),
            kind: InputProfileKind::KeyboardLayout,
            language_id: 0x0409,
            tsf_class_id: None,
            tsf_profile_id: None,
            hkl: Some("0000000000000409".into()),
        };
        keyboard.validate().unwrap();

        let processor = InputProfile {
            id: "input:v1:tip:0404:b115690aea0248d5a231e3578d2fdf80:b2f9c502174211d497900080c882687e:none".into(),
            language_tag: "zh-TW".into(),
            display_name: "Chinese (Traditional, Taiwan)".into(),
            input_method_name: "Microsoft Bopomofo".into(),
            kind: InputProfileKind::InputProcessor,
            language_id: 0x0404,
            tsf_class_id: Some("b115690aea0248d5a231e3578d2fdf80".into()),
            tsf_profile_id: Some("b2f9c502174211d497900080c882687e".into()),
            hkl: None,
        };
        processor.validate().unwrap();
        let encoded = serde_json::to_string(&processor).unwrap();
        assert_eq!(
            serde_json::from_str::<InputProfile>(&encoded).unwrap(),
            processor
        );

        let mut invalid = keyboard.clone();
        invalid.id.push('0');
        assert!(invalid.validate().is_err());
        let mut invalid = processor;
        invalid.tsf_profile_id = Some("not-a-guid".into());
        assert!(invalid.validate().is_err());

        let profiles = (1..=MAX_INPUT_PROFILES)
            .map(|index| {
                let hkl = format!("{index:016x}");
                InputProfile {
                    id: format!("input:v1:kbd:0409:{hkl}"),
                    hkl: Some(hkl),
                    ..keyboard.clone()
                }
            })
            .collect::<Vec<_>>();
        InputStatus {
            active_profile_id: profiles[0].id.clone(),
            profiles: profiles.clone(),
        }
        .validate()
        .unwrap();
        let mut oversized = profiles;
        let hkl = format!("{:016x}", MAX_INPUT_PROFILES + 1);
        oversized.push(InputProfile {
            id: format!("input:v1:kbd:0409:{hkl}"),
            hkl: Some(hkl),
            ..keyboard
        });
        assert!(
            InputStatus {
                active_profile_id: oversized[0].id.clone(),
                profiles: oversized,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn language_preferences_is_fieldless_and_accepted_is_not_observed() {
        let command = SystemStatusCommand::OpenLanguagePreferences;
        command.validate().unwrap();
        assert_eq!(
            serde_json::to_string(&command).unwrap(),
            r#"{"kind":"open_language_preferences"}"#
        );
        assert!(
            SystemStatusCommandTerminal {
                correlation_id: "settings".into(),
                host_generation: 1,
                observed_snapshot_generation: Some(2),
                terminal: SystemStatusTerminalKind::Accepted,
                message: String::new(),
            }
            .validate()
            .is_err()
        );
    }
}
