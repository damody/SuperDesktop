use std::collections::{BTreeMap, BTreeSet};

use settings_store::TaskbarSettings;
use shell_core::{ApplicationId, WindowId};
use shell_provider_protocol::{CommandDescriptor, validate_command_tree};

pub const HOVER_PREVIEW_DELAY_MS: u64 = 400;
pub const MAX_JUMP_LIST_ITEMS: usize = 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewCard {
    pub window_id: WindowId,
    pub title: String,
    pub minimized: bool,
    pub preview_available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlyoutAction {
    Activate(WindowId),
    Close(WindowId),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlyoutModel {
    pub visible: bool,
    pub visible_after_ms: u64,
    pub cards: Vec<PreviewCard>,
    pub focused: Option<usize>,
}

impl FlyoutModel {
    pub fn schedule(&mut self, cards: Vec<PreviewCard>, now_ms: u64) {
        self.cards = cards;
        self.visible = false;
        self.visible_after_ms = now_ms.saturating_add(HOVER_PREVIEW_DELAY_MS);
        self.focused = (!self.cards.is_empty()).then_some(0);
    }

    pub fn tick(&mut self, now_ms: u64) {
        if !self.cards.is_empty() && now_ms >= self.visible_after_ms {
            self.visible = true;
        }
    }

    pub fn reconcile(&mut self, available: &BTreeSet<WindowId>) {
        let focused_id = self
            .focused
            .and_then(|index| self.cards.get(index))
            .map(|card| card.window_id.clone());
        self.cards
            .retain(|card| available.contains(&card.window_id));
        self.focused = focused_id
            .and_then(|id| self.cards.iter().position(|card| card.window_id == id))
            .or_else(|| (!self.cards.is_empty()).then_some(0));
        if self.cards.is_empty() {
            self.visible = false
        }
    }

    pub fn move_focus(&mut self, delta: i32) {
        if self.cards.is_empty() {
            self.focused = None;
            return;
        }
        let current = self.focused.unwrap_or(0) as i32;
        self.focused = Some((current + delta).rem_euclid(self.cards.len() as i32) as usize);
    }

    pub fn activate(&self, index: usize) -> Option<FlyoutAction> {
        self.cards
            .get(index)
            .map(|card| FlyoutAction::Activate(card.window_id.clone()))
    }

    pub fn close(&self, index: usize) -> Option<FlyoutAction> {
        self.cards
            .get(index)
            .map(|card| FlyoutAction::Close(card.window_id.clone()))
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
        self.focused = None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressState {
    None,
    Indeterminate,
    Normal(u16),
    Paused(u16),
    Error(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskOverlay {
    pub progress: ProgressState,
    pub attention: bool,
    pub badge: Option<String>,
}

impl Default for TaskOverlay {
    fn default() -> Self {
        Self {
            progress: ProgressState::None,
            attention: false,
            badge: None,
        }
    }
}

impl TaskOverlay {
    pub fn set_progress(&mut self, progress: ProgressState) {
        self.progress = progress
    }
    pub fn set_attention(&mut self, attention: bool) {
        self.attention = attention
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum JumpListGroup {
    Recent,
    Frequent,
    Tasks,
    Local,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JumpListModel {
    groups: BTreeMap<JumpListGroup, Vec<CommandDescriptor>>,
}

impl JumpListModel {
    pub fn compose(
        recent: Vec<CommandDescriptor>,
        frequent: Vec<CommandDescriptor>,
        tasks: Vec<CommandDescriptor>,
        local: Vec<CommandDescriptor>,
    ) -> Self {
        let mut seen = BTreeSet::new();
        let mut remaining = MAX_JUMP_LIST_ITEMS;
        let mut groups = BTreeMap::new();
        for (group, input) in [
            (JumpListGroup::Recent, recent),
            (JumpListGroup::Frequent, frequent),
            (JumpListGroup::Tasks, tasks),
            (JumpListGroup::Local, local),
        ] {
            let mut output = Vec::new();
            for command in input {
                if remaining == 0 {
                    break;
                }
                if validate_command_tree(std::slice::from_ref(&command)).is_ok()
                    && seen.insert(command.id.0.clone())
                {
                    output.push(command);
                    remaining -= 1;
                }
            }
            if !output.is_empty() {
                groups.insert(group, output);
            }
        }
        Self { groups }
    }

    pub fn groups(&self) -> &BTreeMap<JumpListGroup, Vec<CommandDescriptor>> {
        &self.groups
    }

    pub fn invoke(&self, group: JumpListGroup, index: usize) -> Option<CommandDescriptor> {
        self.groups
            .get(&group)?
            .get(index)
            .filter(|command| command.enabled)
            .cloned()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvancedTaskbarPreferences {
    pub version: u32,
    pub pin_order: Vec<ApplicationId>,
    pub combine_groups: bool,
    pub show_labels: bool,
    pub previews_enabled: bool,
    pub all_monitors: bool,
}

impl AdvancedTaskbarPreferences {
    pub fn from_settings(settings: &TaskbarSettings) -> Self {
        Self {
            version: 1,
            pin_order: settings
                .pins
                .iter()
                .filter_map(|id| ApplicationId::new(id).ok())
                .collect(),
            combine_groups: settings.combine_groups,
            show_labels: settings.show_labels,
            previews_enabled: settings.previews_enabled,
            all_monitors: settings.all_monitors,
        }
    }

    pub fn apply_to_settings(&self, settings: &mut TaskbarSettings) {
        settings.pins = self.pin_order.iter().map(ToString::to_string).collect();
        settings.combine_groups = self.combine_groups;
        settings.show_labels = self.show_labels;
        settings.previews_enabled = self.previews_enabled;
        settings.all_monitors = self.all_monitors;
    }

    pub fn reconcile(&mut self, available: &BTreeSet<ApplicationId>) {
        self.pin_order.retain(|id| available.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::{CommandId, CommandRisk};

    fn window(id: &str) -> WindowId {
        WindowId::new(id).unwrap()
    }
    fn command(id: &str, enabled: bool) -> CommandDescriptor {
        CommandDescriptor {
            id: CommandId(id.into()),
            label: id.into(),
            enabled,
            risk: CommandRisk::Normal,
            children: Vec::new(),
        }
    }

    #[test]
    fn flyout_delay_reconcile_actions_and_fallback_are_stable() {
        let mut model = FlyoutModel::default();
        model.schedule(
            vec![
                PreviewCard {
                    window_id: window("one"),
                    title: "One".into(),
                    minimized: false,
                    preview_available: false,
                },
                PreviewCard {
                    window_id: window("two"),
                    title: "Two".into(),
                    minimized: true,
                    preview_available: true,
                },
            ],
            100,
        );
        model.tick(499);
        assert!(!model.visible);
        model.tick(500);
        assert!(model.visible);
        assert_eq!(
            model.activate(0),
            Some(FlyoutAction::Activate(window("one")))
        );
        assert_eq!(model.close(0), Some(FlyoutAction::Close(window("one"))));
        model.reconcile(&[window("two")].into_iter().collect());
        assert_eq!(model.cards.len(), 1);
    }

    #[test]
    fn jump_lists_overlay_and_preferences_are_independent_and_bounded() {
        let list = JumpListModel::compose(
            vec![command("same", true)],
            vec![command("same", true)],
            vec![command("disabled", false)],
            Vec::new(),
        );
        assert_eq!(list.groups().values().map(Vec::len).sum::<usize>(), 2);
        assert!(list.invoke(JumpListGroup::Tasks, 0).is_none());
        let mut overlay = TaskOverlay::default();
        overlay.set_progress(ProgressState::Error(500));
        overlay.set_attention(true);
        overlay.set_progress(ProgressState::None);
        assert!(overlay.attention);
        let settings = TaskbarSettings {
            pins: vec!["one".into(), "missing".into()],
            ..TaskbarSettings::default()
        };
        let mut preferences = AdvancedTaskbarPreferences::from_settings(&settings);
        preferences.reconcile(&[ApplicationId::new("one").unwrap()].into_iter().collect());
        let mut round_trip = TaskbarSettings::default();
        preferences.apply_to_settings(&mut round_trip);
        assert_eq!(round_trip.pins, vec!["one"]);
    }
}
