use std::collections::BTreeMap;

use shell_core::{
    BridgeLaunchRequest, BridgeLaunchSource, BridgeTerminal, CorrelationId, MessageKey, RequestId,
    ShellItemId,
};

use crate::DesktopItem;
use crate::DesktopOperationRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationSource {
    Pointer,
    Keyboard,
    Accessibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredAction {
    ContextMenu,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub item_identity: ShellItemId,
    pub activation_token: String,
    pub admission_deadline_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationEffect {
    Bridge(BridgeLaunchRequest),
    Association(AssociationRequest),
    DeferredUnavailable(DeferredAction),
    DesktopOperation(DesktopOperationRequest),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalResult {
    Launched,
    ValidationFailed,
    LaunchFailed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairState {
    None,
    LocateExecutable,
    RetryAssociation,
    RetryBridge,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingActivation {
    terminal: Option<TerminalResult>,
    repair: RepairState,
}

#[derive(Clone, Debug, Default)]
pub struct ActivationController {
    next_request: u64,
    next_correlation: u128,
    pending: BTreeMap<CorrelationId, PendingActivation>,
}

impl ActivationController {
    pub fn activate_item(
        &mut self,
        item: &DesktopItem,
        _source: ActivationSource,
    ) -> Option<ActivationEffect> {
        let (request_id, correlation_id) = self.allocate();
        self.pending.insert(
            correlation_id,
            PendingActivation {
                terminal: None,
                repair: RepairState::None,
            },
        );
        if item.capabilities.folder {
            Some(ActivationEffect::Bridge(BridgeLaunchRequest {
                request_id,
                correlation_id,
                source: BridgeLaunchSource::DesktopFolder,
                initial_path: Some(item.activation_token.clone()),
            }))
        } else if item.capabilities.association {
            Some(ActivationEffect::Association(AssociationRequest {
                request_id,
                correlation_id,
                item_identity: item.identity.clone(),
                activation_token: item.activation_token.clone(),
                admission_deadline_ms: 5_000,
            }))
        } else {
            self.pending.remove(&correlation_id);
            None
        }
    }

    pub fn activate_fixed_entry(&mut self, _source: ActivationSource) -> ActivationEffect {
        let (request_id, correlation_id) = self.allocate();
        self.pending.insert(
            correlation_id,
            PendingActivation {
                terminal: None,
                repair: RepairState::None,
            },
        );
        ActivationEffect::Bridge(BridgeLaunchRequest::default_location(
            request_id,
            correlation_id,
            BridgeLaunchSource::DesktopFixedEntry,
        ))
    }

    pub fn deferred(&self, action: DeferredAction) -> ActivationEffect {
        ActivationEffect::DeferredUnavailable(action)
    }
    pub fn desktop_operation(&self, request: DesktopOperationRequest) -> ActivationEffect {
        ActivationEffect::DesktopOperation(request)
    }
    pub fn apply_bridge_terminal(
        &mut self,
        correlation_id: CorrelationId,
        terminal: BridgeTerminal,
    ) -> bool {
        let mapped = match terminal {
            BridgeTerminal::Launched => TerminalResult::Launched,
            BridgeTerminal::Cancelled => TerminalResult::Cancelled,
            BridgeTerminal::TimedOut => TerminalResult::TimedOut,
            BridgeTerminal::ResolverUnavailable => TerminalResult::ValidationFailed,
            BridgeTerminal::SpawnRejected | BridgeTerminal::AdmissionFailed => {
                TerminalResult::LaunchFailed
            }
        };
        self.apply_terminal(correlation_id, mapped, true)
    }
    pub fn apply_association_terminal(
        &mut self,
        correlation_id: CorrelationId,
        terminal: TerminalResult,
    ) -> bool {
        self.apply_terminal(correlation_id, terminal, false)
    }
    pub fn repair_state(&self, correlation_id: CorrelationId) -> Option<RepairState> {
        self.pending
            .get(&correlation_id)
            .map(|pending| pending.repair)
    }
    pub fn terminal(&self, correlation_id: CorrelationId) -> Option<TerminalResult> {
        self.pending
            .get(&correlation_id)
            .and_then(|pending| pending.terminal)
    }

    fn apply_terminal(
        &mut self,
        correlation_id: CorrelationId,
        terminal: TerminalResult,
        bridge: bool,
    ) -> bool {
        let Some(pending) = self.pending.get_mut(&correlation_id) else {
            return false;
        };
        if pending.terminal.is_some() {
            return false;
        }
        pending.terminal = Some(terminal);
        pending.repair = match terminal {
            TerminalResult::Launched => RepairState::None,
            TerminalResult::ValidationFailed if bridge => RepairState::LocateExecutable,
            TerminalResult::ValidationFailed | TerminalResult::LaunchFailed => {
                if bridge {
                    RepairState::RetryBridge
                } else {
                    RepairState::RetryAssociation
                }
            }
            TerminalResult::Cancelled => RepairState::Cancelled,
            TerminalResult::TimedOut => RepairState::TimedOut,
        };
        true
    }
    fn allocate(&mut self) -> (RequestId, CorrelationId) {
        self.next_request = self.next_request.saturating_add(1);
        self.next_correlation = self.next_correlation.saturating_add(1);
        (
            RequestId(self.next_request),
            CorrelationId(self.next_correlation),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibleAction {
    Focus,
    Select,
    Invoke,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibleNode {
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub selected: bool,
    pub focused: bool,
    pub actions: Vec<AccessibleAction>,
    pub message_key: Option<MessageKey>,
}

impl AccessibleNode {
    pub fn fixed_superexplorer(monitor_key: &str, selected: bool, focused: bool) -> Self {
        Self {
            stable_id: format!("desktop:{monitor_key}:superexplorer"),
            name: "SuperExplorer".into(),
            role: "button",
            selected,
            focused,
            actions: vec![
                AccessibleAction::Focus,
                AccessibleAction::Select,
                AccessibleAction::Invoke,
            ],
            message_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DesktopOrigin, IconDescriptor, ItemCapabilities};
    fn item(folder: bool, association: bool) -> DesktopItem {
        DesktopItem {
            identity: ShellItemId::new("item").unwrap(),
            display_name: "Item".into(),
            origin: DesktopOrigin::User,
            activation_token: "owned-token".into(),
            icon: IconDescriptor {
                source_key: "icon".into(),
                resource_index: None,
            },
            capabilities: ItemCapabilities {
                folder,
                association,
                hidden: false,
                system: false,
            },
        }
    }
    #[test]
    fn pointer_keyboard_and_uia_emit_equivalent_folder_commands() {
        for source in [
            ActivationSource::Pointer,
            ActivationSource::Keyboard,
            ActivationSource::Accessibility,
        ] {
            let mut controller = ActivationController::default();
            let command = controller
                .activate_item(&item(true, false), source)
                .unwrap();
            assert!(
                matches!(command,ActivationEffect::Bridge(BridgeLaunchRequest{initial_path:Some(ref token),source:BridgeLaunchSource::DesktopFolder,..}) if token=="owned-token")
            );
        }
    }
    #[test]
    fn fixed_entry_is_truthful_accessible_and_has_no_local_claim() {
        let node = AccessibleNode::fixed_superexplorer("m", false, true);
        assert_eq!(node.name, "SuperExplorer");
        assert_eq!(node.role, "button");
        assert!(node.actions.contains(&AccessibleAction::Invoke));
        let mut controller = ActivationController::default();
        assert!(matches!(
            controller.activate_fixed_entry(ActivationSource::Accessibility),
            ActivationEffect::Bridge(BridgeLaunchRequest {
                initial_path: None,
                ..
            })
        ));
    }
    #[test]
    fn first_terminal_wins_for_bridge_and_association() {
        let mut controller = ActivationController::default();
        let ActivationEffect::Bridge(request) = controller
            .activate_item(&item(true, false), ActivationSource::Keyboard)
            .unwrap()
        else {
            panic!()
        };
        assert!(controller.apply_bridge_terminal(request.correlation_id, BridgeTerminal::TimedOut));
        assert!(
            !controller.apply_bridge_terminal(request.correlation_id, BridgeTerminal::Launched)
        );
        assert_eq!(
            controller.terminal(request.correlation_id),
            Some(TerminalResult::TimedOut)
        );
        assert_eq!(
            controller.repair_state(request.correlation_id),
            Some(RepairState::TimedOut)
        );
        let ActivationEffect::Association(request) = controller
            .activate_item(&item(false, true), ActivationSource::Pointer)
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(request.admission_deadline_ms, 5_000);
        assert!(
            controller
                .apply_association_terminal(request.correlation_id, TerminalResult::LaunchFailed)
        );
        assert!(
            !controller
                .apply_association_terminal(request.correlation_id, TerminalResult::Launched)
        );
        assert_eq!(
            controller.repair_state(request.correlation_id),
            Some(RepairState::RetryAssociation)
        );
    }
    #[test]
    fn deferred_actions_never_emit_mutating_effect() {
        let controller = ActivationController::default();
        let action = DeferredAction::ContextMenu;
        assert_eq!(
            controller.deferred(action),
            ActivationEffect::DeferredUnavailable(action)
        );
    }
}
