use std::collections::BTreeMap;

use shell_core::{
    ApplicationId, BridgeLaunchRequest, BridgeLaunchSource, BridgeTerminal, CorrelationId,
    MessageKey, RequestId, WindowId,
};

use crate::TaskGroup;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSource {
    Pointer,
    Keyboard,
    Accessibility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskEffect {
    Activate(WindowId),
    Minimize(WindowId),
    RestoreAndActivate(WindowId),
    ShowGroup(Vec<WindowId>),
    LaunchBridge(BridgeLaunchRequest),
    LaunchPinned(ApplicationId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskAction {
    Focus,
    Select,
    Invoke,
    Minimize,
    Restore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibleTask {
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub active: bool,
    pub minimized: bool,
    pub attention: bool,
    pub group_size: usize,
    pub available: bool,
    pub actions: Vec<TaskAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedEntry {
    pub stable_id: &'static str,
    pub label: &'static str,
    pub role: &'static str,
    pub message_key: Option<MessageKey>,
}
impl Default for FixedEntry {
    fn default() -> Self {
        Self {
            stable_id: "taskbar:superexplorer",
            label: "SuperExplorer",
            role: "button",
            message_key: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GroupSelection {
    pub visible: bool,
    pub window_ids: Vec<WindowId>,
    pub focus_index: usize,
}
impl GroupSelection {
    pub fn escape(&mut self) {
        self.visible = false;
        self.focus_index = 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairPrompt {
    None,
    LocateExecutable,
    RetrySpawn,
    TimedOut,
    Cancelled,
}

#[derive(Clone, Debug, Default)]
pub struct TaskInteraction {
    next_request: u64,
    next_correlation: u128,
    pub group_selection: GroupSelection,
    fixed_terminals: BTreeMap<CorrelationId, RepairPrompt>,
}

impl TaskInteraction {
    pub fn activate_group(
        &mut self,
        group: &TaskGroup,
        active: Option<&WindowId>,
        minimized: bool,
        _source: TaskSource,
    ) -> Option<TaskEffect> {
        match group.windows.as_slice() {
            [] if group.pinned => Some(TaskEffect::LaunchPinned(group.application_id.clone())),
            [] => None,
            [only] if active == Some(only) => Some(TaskEffect::Minimize(only.clone())),
            [only] if minimized => Some(TaskEffect::RestoreAndActivate(only.clone())),
            [only] => Some(TaskEffect::Activate(only.clone())),
            many => {
                self.group_selection = GroupSelection {
                    visible: true,
                    window_ids: many.to_vec(),
                    focus_index: 0,
                };
                Some(TaskEffect::ShowGroup(many.to_vec()))
            }
        }
    }
    pub fn activate_fixed(&mut self, _source: TaskSource) -> TaskEffect {
        self.next_request = self.next_request.saturating_add(1);
        self.next_correlation = self.next_correlation.saturating_add(1);
        let request = BridgeLaunchRequest::default_location(
            RequestId(self.next_request),
            CorrelationId(self.next_correlation),
            BridgeLaunchSource::TaskbarFixedEntry,
        );
        self.fixed_terminals
            .insert(request.correlation_id, RepairPrompt::None);
        TaskEffect::LaunchBridge(request)
    }
    pub fn apply_fixed_terminal(
        &mut self,
        correlation_id: CorrelationId,
        terminal: BridgeTerminal,
    ) -> bool {
        let Some(state) = self.fixed_terminals.get_mut(&correlation_id) else {
            return false;
        };
        if *state != RepairPrompt::None {
            return false;
        }
        *state = match terminal {
            BridgeTerminal::Launched => RepairPrompt::None,
            BridgeTerminal::ResolverUnavailable => RepairPrompt::LocateExecutable,
            BridgeTerminal::SpawnRejected | BridgeTerminal::AdmissionFailed => {
                RepairPrompt::RetrySpawn
            }
            BridgeTerminal::TimedOut => RepairPrompt::TimedOut,
            BridgeTerminal::Cancelled => RepairPrompt::Cancelled,
        };
        if terminal == BridgeTerminal::Launched {
            self.fixed_terminals.remove(&correlation_id);
        }
        true
    }
    pub fn repair_prompt(&self, correlation_id: CorrelationId) -> Option<RepairPrompt> {
        self.fixed_terminals.get(&correlation_id).copied()
    }
    pub fn accessible(
        group: &TaskGroup,
        title: &str,
        active: bool,
        minimized: bool,
    ) -> AccessibleTask {
        AccessibleTask {
            stable_id: format!("taskbar:{}", group.application_id),
            name: title.into(),
            role: "button",
            active,
            minimized,
            attention: group.attention,
            group_size: group.windows.len().max(1),
            available: true,
            actions: vec![
                TaskAction::Focus,
                TaskAction::Select,
                TaskAction::Invoke,
                TaskAction::Minimize,
                TaskAction::Restore,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_core::ApplicationId;
    fn group(windows: &[&str]) -> TaskGroup {
        TaskGroup {
            application_id: ApplicationId::new("app").unwrap(),
            windows: windows
                .iter()
                .map(|id| WindowId::new(*id).unwrap())
                .collect(),
            pinned: false,
            attention: false,
            order: 1,
        }
    }
    #[test]
    fn active_minimizes_inactive_activates_and_minimized_restores() {
        let mut i = TaskInteraction::default();
        let g = group(&["w"]);
        let w = WindowId::new("w").unwrap();
        assert_eq!(
            i.activate_group(&g, Some(&w), false, TaskSource::Pointer),
            Some(TaskEffect::Minimize(w.clone()))
        );
        assert_eq!(
            i.activate_group(&g, None, true, TaskSource::Keyboard),
            Some(TaskEffect::RestoreAndActivate(w.clone()))
        );
        assert_eq!(
            i.activate_group(&g, None, false, TaskSource::Accessibility),
            Some(TaskEffect::Activate(w))
        )
    }
    #[test]
    fn multi_window_selection_escapes_and_returns_focus() {
        let mut i = TaskInteraction::default();
        let g = group(&["a", "b"]);
        assert!(matches!(
            i.activate_group(&g, None, false, TaskSource::Pointer),
            Some(TaskEffect::ShowGroup(_))
        ));
        assert!(i.group_selection.visible);
        i.group_selection.escape();
        assert!(!i.group_selection.visible)
    }
    #[test]
    fn fixed_entry_sources_emit_equivalent_truthful_bridge_request() {
        let mut i = TaskInteraction::default();
        let effects = [
            TaskSource::Pointer,
            TaskSource::Keyboard,
            TaskSource::Accessibility,
        ]
        .map(|source| i.activate_fixed(source));
        for effect in effects {
            let TaskEffect::LaunchBridge(request) = effect else {
                panic!()
            };
            assert_eq!(request.initial_path, None);
            assert_eq!(request.source, BridgeLaunchSource::TaskbarFixedEntry)
        }
        let entry = FixedEntry::default();
        assert_eq!(entry.label, "SuperExplorer");
        assert_eq!(entry.role, "button")
    }
    #[test]
    fn pinned_without_windows_launches_and_fixed_failure_is_first_terminal() {
        let mut interaction = TaskInteraction::default();
        let mut pinned = group(&[]);
        pinned.pinned = true;
        assert!(matches!(
            interaction.activate_group(&pinned, None, false, TaskSource::Pointer),
            Some(TaskEffect::LaunchPinned(_))
        ));
        let TaskEffect::LaunchBridge(request) = interaction.activate_fixed(TaskSource::Pointer)
        else {
            panic!()
        };
        assert!(
            interaction.apply_fixed_terminal(request.correlation_id, BridgeTerminal::SpawnRejected)
        );
        assert_eq!(
            interaction.repair_prompt(request.correlation_id),
            Some(RepairPrompt::RetrySpawn)
        );
        assert!(
            !interaction.apply_fixed_terminal(request.correlation_id, BridgeTerminal::Launched)
        );
    }
}
