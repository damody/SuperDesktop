use std::collections::{BTreeMap, BTreeSet};

use platform_win::common::virtual_desktop::VirtualDesktopCapabilities;
use shell_core::WindowId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopCard {
    pub id: u128,
    pub name: String,
    pub active: bool,
    pub windows: Vec<WindowId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VirtualDesktopSnapshot {
    pub generation: u64,
    pub desktops: Vec<DesktopCard>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskViewEffect {
    Switch(u128),
    Create,
    Remove(u128),
    Rename {
        desktop_id: u128,
        name: String,
    },
    MoveWindow {
        window_id: WindowId,
        desktop_id: u128,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskViewAccessibleNode {
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub focused: bool,
    pub available: bool,
}

#[derive(Clone, Debug)]
pub struct TaskViewModel {
    pub visible: bool,
    pub generation: u64,
    pub desktops: Vec<DesktopCard>,
    pub focused: Option<usize>,
    pub capabilities: VirtualDesktopCapabilities,
    pub unavailable_reason: Option<String>,
}

impl TaskViewModel {
    pub fn new(capabilities: VirtualDesktopCapabilities) -> Self {
        Self {
            visible: false,
            generation: 0,
            desktops: Vec::new(),
            focused: None,
            capabilities,
            unavailable_reason: None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.focused = (!self.desktops.is_empty()).then_some(0)
    }
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.focused = None
    }

    pub fn apply_snapshot(&mut self, mut snapshot: VirtualDesktopSnapshot) -> bool {
        if snapshot.generation < self.generation || !self.capabilities.enumerate {
            return false;
        }
        let focused_id = self
            .focused
            .and_then(|index| self.desktops.get(index))
            .map(|desktop| desktop.id);
        snapshot.desktops.sort_by_key(|desktop| desktop.id);
        for desktop in &mut snapshot.desktops {
            desktop.windows.sort();
            desktop.windows.dedup();
        }
        self.generation = snapshot.generation;
        self.desktops = snapshot.desktops;
        self.focused = focused_id
            .and_then(|id| self.desktops.iter().position(|desktop| desktop.id == id))
            .or_else(|| (!self.desktops.is_empty()).then_some(0));
        true
    }

    pub fn reconcile_windows(&mut self, available: &BTreeSet<WindowId>) {
        for desktop in &mut self.desktops {
            desktop.windows.retain(|window| available.contains(window));
        }
    }

    pub fn observed_membership(&mut self, membership: BTreeMap<u128, Vec<WindowId>>) {
        if self.capabilities.enumerate {
            return;
        }
        let mut desktops: Vec<_> = membership
            .into_iter()
            .map(|(id, mut windows)| {
                windows.sort();
                windows.dedup();
                DesktopCard {
                    id,
                    name: format!("Desktop {id:032x}"),
                    active: false,
                    windows,
                }
            })
            .collect();
        desktops.sort_by_key(|desktop| desktop.id);
        self.desktops = desktops;
        self.focused = (!self.desktops.is_empty()).then_some(0);
    }

    pub fn switch(&mut self, desktop_id: u128) -> Option<TaskViewEffect> {
        capability_effect(
            self.capabilities.switch,
            &mut self.unavailable_reason,
            TaskViewEffect::Switch(desktop_id),
        )
    }
    pub fn create(&mut self) -> Option<TaskViewEffect> {
        capability_effect(
            self.capabilities.create,
            &mut self.unavailable_reason,
            TaskViewEffect::Create,
        )
    }
    pub fn remove(&mut self, desktop_id: u128) -> Option<TaskViewEffect> {
        capability_effect(
            self.capabilities.remove,
            &mut self.unavailable_reason,
            TaskViewEffect::Remove(desktop_id),
        )
    }
    pub fn rename(&mut self, desktop_id: u128, name: String) -> Option<TaskViewEffect> {
        capability_effect(
            self.capabilities.rename && !name.trim().is_empty(),
            &mut self.unavailable_reason,
            TaskViewEffect::Rename { desktop_id, name },
        )
    }
    pub fn move_window(&mut self, window_id: WindowId, desktop_id: u128) -> Option<TaskViewEffect> {
        capability_effect(
            self.capabilities.move_window,
            &mut self.unavailable_reason,
            TaskViewEffect::MoveWindow {
                window_id,
                desktop_id,
            },
        )
    }

    pub fn accessibility_nodes(&self) -> Vec<TaskViewAccessibleNode> {
        self.desktops
            .iter()
            .enumerate()
            .map(|(index, desktop)| TaskViewAccessibleNode {
                stable_id: format!("task-view:{:032x}", desktop.id),
                name: desktop.name.clone(),
                role: "tab",
                focused: self.focused == Some(index),
                available: self.capabilities.switch,
            })
            .collect()
    }
}

fn capability_effect(
    available: bool,
    unavailable_reason: &mut Option<String>,
    effect: TaskViewEffect,
) -> Option<TaskViewEffect> {
    if available {
        *unavailable_reason = None;
        Some(effect)
    } else {
        *unavailable_reason = Some("virtual-desktop-capability-unavailable".into());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partial() -> VirtualDesktopCapabilities {
        VirtualDesktopCapabilities {
            query_window: true,
            move_window: true,
            ..VirtualDesktopCapabilities::UNAVAILABLE
        }
    }

    #[test]
    fn partial_capability_is_truthful_and_window_move_remains_available() {
        let mut model = TaskViewModel::new(partial());
        model.observed_membership(
            [(1, vec![WindowId::new("window").unwrap()])]
                .into_iter()
                .collect(),
        );
        model.open();
        assert!(model.switch(1).is_none());
        assert!(model.unavailable_reason.is_some());
        assert!(matches!(
            model.move_window(WindowId::new("window").unwrap(), 1),
            Some(TaskViewEffect::MoveWindow { .. })
        ));
        assert!(!model.accessibility_nodes()[0].available);
    }

    #[test]
    fn stale_snapshot_and_stale_windows_are_ignored_or_reconciled() {
        let mut capabilities = partial();
        capabilities.enumerate = true;
        capabilities.switch = true;
        let mut model = TaskViewModel::new(capabilities);
        assert!(model.apply_snapshot(VirtualDesktopSnapshot {
            generation: 2,
            desktops: vec![DesktopCard {
                id: 2,
                name: "Two".into(),
                active: true,
                windows: vec![WindowId::new("old").unwrap()]
            }]
        }));
        assert!(!model.apply_snapshot(VirtualDesktopSnapshot {
            generation: 1,
            desktops: Vec::new()
        }));
        model.reconcile_windows(&BTreeSet::new());
        assert!(model.desktops[0].windows.is_empty());
    }
}
