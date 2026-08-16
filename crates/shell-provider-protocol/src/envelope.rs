use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, ProviderCapability, Validate, ValidationError};

pub const CURRENT_PROTOCOL: ProtocolVersion = ProtocolVersion { major: 1, minor: 0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const fn is_compatible_with(self, supported: Self) -> bool {
        self.major == supported.major && self.minor <= supported.minor
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub protocol: ProtocolVersion,
    pub request_id: String,
    pub correlation_id: String,
    pub deadline_unix_ms: Option<u64>,
    pub payload: T,
}

impl<T: Validate> Envelope<T> {
    pub fn validate_at(&self, now_unix_ms: u64) -> Result<(), ValidationError> {
        if !self.protocol.is_compatible_with(CURRENT_PROTOCOL) {
            return Err(ValidationError::UnsupportedProtocol {
                major: self.protocol.major,
            });
        }
        if self.request_id.trim().is_empty() {
            return Err(ValidationError::Empty("request_id"));
        }
        if self.correlation_id.trim().is_empty() {
            return Err(ValidationError::Empty("correlation_id"));
        }
        if let Some(deadline) = self.deadline_unix_ms
            && deadline <= now_unix_ms
        {
            return Err(ValidationError::Expired);
        }
        self.payload.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderRequest {
    Handshake,
    Health,
    Execute {
        capability: ProviderCapability,
        arguments: Vec<String>,
    },
    Cancel {
        target_request_id: String,
    },
}

impl Validate for ProviderRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Execute { arguments, .. } => {
                if arguments.len() > MAX_COLLECTION_ITEMS {
                    return Err(ValidationError::CollectionTooLarge("request.arguments"));
                }
                if arguments
                    .iter()
                    .any(|value| value.len() > crate::MAX_TEXT_BYTES)
                {
                    return Err(ValidationError::TextTooLong("request.arguments"));
                }
            }
            Self::Cancel { target_request_id } if target_request_id.trim().is_empty() => {
                return Err(ValidationError::Empty("target_request_id"));
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol: ProtocolVersion,
    pub capabilities: Vec<ProviderCapability>,
    pub max_active_requests: usize,
    pub max_frame_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostHealth {
    pub healthy: bool,
    pub active_requests: usize,
    pub capacity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalKind {
    Success,
    Unavailable,
    Cancelled,
    Timeout,
    InvalidRequest,
    Busy,
    ProviderFailure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ResponseBody {
    Handshake(Handshake),
    Health(HostHealth),
    Arguments(Vec<String>),
    Message(String),
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub request_id: String,
    pub correlation_id: String,
    pub terminal: TerminalKind,
    pub body: ResponseBody,
}

pub fn contract_manifest() -> serde_json::Value {
    serde_json::json!({
        "name": "superdesktop-shell-provider-protocol",
        "protocol": { "major": CURRENT_PROTOCOL.major, "minor": CURRENT_PROTOCOL.minor },
        "max_frame_bytes": crate::MAX_FRAME_BYTES,
        "max_collection_items": crate::MAX_COLLECTION_ITEMS,
        "terminal_outcomes": ["success", "unavailable", "cancelled", "timeout", "invalid_request", "busy", "provider_failure"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(payload: ProviderRequest) -> Envelope<ProviderRequest> {
        Envelope {
            protocol: CURRENT_PROTOCOL,
            request_id: "request-1".into(),
            correlation_id: "correlation-1".into(),
            deadline_unix_ms: Some(2_000),
            payload,
        }
    }

    #[test]
    fn request_round_trips_deterministically() {
        let request = envelope(ProviderRequest::Health);
        let json = serde_json::to_string(&request).unwrap();
        let decoded: Envelope<ProviderRequest> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
    }

    #[test]
    fn additive_unknown_field_is_accepted() {
        let json = r#"{"protocol":{"major":1,"minor":0},"request_id":"r","correlation_id":"c","deadline_unix_ms":null,"payload":{"kind":"health"},"future":true}"#;
        let decoded: Envelope<ProviderRequest> = serde_json::from_str(json).unwrap();
        assert!(decoded.validate_at(0).is_ok());
    }

    #[test]
    fn incompatible_major_and_expired_deadline_fail_closed() {
        let mut request = envelope(ProviderRequest::Health);
        request.protocol.major = 2;
        assert!(matches!(
            request.validate_at(1_000),
            Err(ValidationError::UnsupportedProtocol { .. })
        ));
        request.protocol = CURRENT_PROTOCOL;
        assert_eq!(request.validate_at(2_000), Err(ValidationError::Expired));
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let bytes = vec![b'x'; crate::MAX_FRAME_BYTES + 1];
        assert!(matches!(
            crate::validate_frame_size(&bytes),
            Err(ValidationError::FrameTooLarge { .. })
        ));
    }
}
