//! Bounded dispatcher for providers hosted outside the GPUI shell process.

use std::collections::BTreeSet;

use shell_provider_protocol::{
    CURRENT_PROTOCOL, Envelope, Handshake, HostHealth, MAX_FRAME_BYTES, ProviderCapability,
    ProviderRequest, ProviderResponse, ResponseBody, TerminalKind, ValidationError,
};

pub const DEFAULT_MAX_ACTIVE_REQUESTS: usize = 32;

#[derive(Debug)]
pub struct Dispatcher {
    active: BTreeSet<String>,
    max_active: usize,
    capabilities: Vec<ProviderCapability>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ACTIVE_REQUESTS)
    }
}

impl Dispatcher {
    pub fn new(max_active: usize) -> Self {
        assert!(max_active > 0, "provider host capacity must be non-zero");
        Self {
            active: BTreeSet::new(),
            max_active,
            capabilities: vec![
                ProviderCapability::ContextMenu,
                ProviderCapability::SearchApplications,
                ProviderCapability::SearchFiles,
                ProviderCapability::SearchSettings,
                ProviderCapability::NotificationArea,
                ProviderCapability::TaskPreview,
                ProviderCapability::VirtualDesktop,
            ],
        }
    }

    pub fn active_requests(&self) -> usize {
        self.active.len()
    }

    pub fn begin(&mut self, request_id: &str) -> Result<(), TerminalKind> {
        if self.active.contains(request_id) {
            return Err(TerminalKind::InvalidRequest);
        }
        if self.active.len() >= self.max_active {
            return Err(TerminalKind::Busy);
        }
        self.active.insert(request_id.to_owned());
        Ok(())
    }

    pub fn finish(&mut self, request_id: &str) -> bool {
        self.active.remove(request_id)
    }

    pub fn dispatch(
        &mut self,
        request: Envelope<ProviderRequest>,
        now_unix_ms: u64,
    ) -> ProviderResponse {
        if let Err(error) = request.validate_at(now_unix_ms) {
            return terminal_for_validation(&request, error);
        }

        if let ProviderRequest::Cancel { target_request_id } = &request.payload {
            let cancelled = self.finish(target_request_id);
            return response(
                &request,
                if cancelled {
                    TerminalKind::Cancelled
                } else {
                    TerminalKind::Unavailable
                },
                ResponseBody::Empty,
            );
        }

        if let Err(terminal) = self.begin(&request.request_id) {
            return response(&request, terminal, ResponseBody::Empty);
        }

        let result = match &request.payload {
            ProviderRequest::Handshake => response(
                &request,
                TerminalKind::Success,
                ResponseBody::Handshake(Handshake {
                    protocol: CURRENT_PROTOCOL,
                    capabilities: self.capabilities.clone(),
                    max_active_requests: self.max_active,
                    max_frame_bytes: MAX_FRAME_BYTES,
                }),
            ),
            ProviderRequest::Health => response(
                &request,
                TerminalKind::Success,
                ResponseBody::Health(HostHealth {
                    healthy: true,
                    active_requests: self.active.len(),
                    capacity: self.max_active,
                }),
            ),
            ProviderRequest::Execute {
                capability,
                arguments,
            } => {
                let supported = self.capabilities.contains(capability);
                response(
                    &request,
                    if supported {
                        TerminalKind::Success
                    } else {
                        TerminalKind::Unavailable
                    },
                    if supported {
                        ResponseBody::Arguments(arguments.clone())
                    } else {
                        ResponseBody::Empty
                    },
                )
            }
            ProviderRequest::Cancel { .. } => unreachable!("cancel returns before dispatch"),
        };
        self.finish(&request.request_id);
        result
    }
}

fn response(
    request: &Envelope<ProviderRequest>,
    terminal: TerminalKind,
    body: ResponseBody,
) -> ProviderResponse {
    ProviderResponse {
        request_id: request.request_id.clone(),
        correlation_id: request.correlation_id.clone(),
        terminal,
        body,
    }
}

fn terminal_for_validation(
    request: &Envelope<ProviderRequest>,
    error: ValidationError,
) -> ProviderResponse {
    let terminal = match error {
        ValidationError::Expired => TerminalKind::Timeout,
        _ => TerminalKind::InvalidRequest,
    };
    response(request, terminal, ResponseBody::Message(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, payload: ProviderRequest) -> Envelope<ProviderRequest> {
        Envelope {
            protocol: CURRENT_PROTOCOL,
            request_id: id.into(),
            correlation_id: "correlation".into(),
            deadline_unix_ms: Some(2_000),
            payload,
        }
    }

    #[test]
    fn handshake_and_health_report_limits() {
        let mut dispatcher = Dispatcher::new(3);
        let handshake = dispatcher.dispatch(request("one", ProviderRequest::Handshake), 1_000);
        assert_eq!(handshake.terminal, TerminalKind::Success);
        assert!(matches!(
            handshake.body,
            ResponseBody::Handshake(Handshake {
                max_active_requests: 3,
                ..
            })
        ));
        let health = dispatcher.dispatch(request("two", ProviderRequest::Health), 1_000);
        assert!(matches!(
            health.body,
            ResponseBody::Health(HostHealth { healthy: true, .. })
        ));
    }

    #[test]
    fn duplicate_capacity_deadline_and_cancel_are_terminal() {
        let mut dispatcher = Dispatcher::new(1);
        dispatcher.begin("held").unwrap();
        assert_eq!(dispatcher.begin("held"), Err(TerminalKind::InvalidRequest));
        assert_eq!(dispatcher.begin("other"), Err(TerminalKind::Busy));
        let cancel = dispatcher.dispatch(
            request(
                "cancel",
                ProviderRequest::Cancel {
                    target_request_id: "held".into(),
                },
            ),
            1_000,
        );
        assert_eq!(cancel.terminal, TerminalKind::Cancelled);
        let expired = dispatcher.dispatch(request("expired", ProviderRequest::Health), 2_000);
        assert_eq!(expired.terminal, TerminalKind::Timeout);
    }
}
