use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::FluentBuilder as _, px, rgb,
};
use platform_win::common::taskbar_preview::{LiveThumbnail, ThumbnailRect};

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
    pub preview_source: Option<isize>,
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

pub type FlyoutWindowAction = Rc<dyn Fn(FlyoutAction)>;
pub type FlyoutDismissAction = Rc<dyn Fn(&mut Window, &mut gpui::App)>;
pub type JumpListInvokeAction = Rc<dyn Fn(&CommandDescriptor)>;

pub struct TaskFlyoutView {
    pub model: FlyoutModel,
    action: FlyoutWindowAction,
    dismiss: FlyoutDismissAction,
    focus: FocusHandle,
    destination_hwnd: isize,
    thumbnails: Rc<RefCell<BTreeMap<WindowId, LiveThumbnail>>>,
}

impl TaskFlyoutView {
    pub fn new(
        cards: Vec<PreviewCard>,
        action: FlyoutWindowAction,
        dismiss: FlyoutDismissAction,
        destination_hwnd: isize,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut model = FlyoutModel::default();
        model.schedule(cards, 0);
        model.tick(HOVER_PREVIEW_DELAY_MS);
        Self {
            model,
            action,
            dismiss,
            focus: cx.focus_handle(),
            destination_hwnd,
            thumbnails: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }
}

impl Render for TaskFlyoutView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let action_for_key = self.action.clone();
        let dismiss_for_key = self.dismiss.clone();
        let scale_factor = window.scale_factor();
        div()
            .id("task-group-flyout")
            .role(gpui::Role::Dialog)
            .aria_label("Window previews")
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_2()
            .flex()
            .gap_2()
            .bg(rgb(0x182028))
            .text_color(rgb(0xf4f7fa))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "left" => this.model.move_focus(-1),
                        "right" => this.model.move_focus(1),
                        "enter" => {
                            if let Some(index) = this.model.focused
                                && let Some(effect) = this.model.activate(index)
                            {
                                action_for_key(effect);
                                dismiss_for_key(window, cx);
                            }
                        }
                        "delete" => {
                            if let Some(index) = this.model.focused
                                && let Some(effect) = this.model.close(index)
                            {
                                action_for_key(effect);
                                dismiss_for_key(window, cx);
                            }
                        }
                        "escape" => {
                            this.model.dismiss();
                            dismiss_for_key(window, cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .children(self.model.cards.iter().enumerate().map(|(index, card)| {
                let action = self.action.clone();
                let close_action = self.action.clone();
                let dismiss = self.dismiss.clone();
                let close_dismiss = self.dismiss.clone();
                let activate_effect = FlyoutAction::Activate(card.window_id.clone());
                let close_effect = FlyoutAction::Close(card.window_id.clone());
                let preview_source = card.preview_source;
                let preview_window = card.window_id.clone();
                let destination_hwnd = self.destination_hwnd;
                let thumbnails = Rc::clone(&self.thumbnails);
                div()
                    .id(format!("flyout-card-{index}"))
                    .role(gpui::Role::Button)
                    .aria_label(card.title.clone())
                    .tab_index(0)
                    .w(px(220.))
                    .h_full()
                    .p_2()
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(self.model.focused == Some(index), |element| {
                        element.border_2().border_color(rgb(0x4aa3ff))
                    })
                    .on_click(cx.listener(move |_, _, window, cx| {
                        action(activate_effect.clone());
                        dismiss(window, cx);
                    }))
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .rounded_md()
                            .bg(rgb(0x2a343e))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when_some(preview_source, move |element, source| {
                                element.child(
                                    canvas(
                                        |bounds, _, _| bounds,
                                        move |bounds, _, _, _| {
                                            let rect = ThumbnailRect {
                                                left: (bounds.origin.x.as_f32() * scale_factor)
                                                    .round()
                                                    as i32,
                                                top: (bounds.origin.y.as_f32() * scale_factor)
                                                    .round()
                                                    as i32,
                                                right: ((bounds.origin.x + bounds.size.width)
                                                    .as_f32()
                                                    * scale_factor)
                                                    .round()
                                                    as i32,
                                                bottom: ((bounds.origin.y + bounds.size.height)
                                                    .as_f32()
                                                    * scale_factor)
                                                    .round()
                                                    as i32,
                                            };
                                            let mut thumbnails = thumbnails.borrow_mut();
                                            if !thumbnails.contains_key(&preview_window)
                                                && let Ok(thumbnail) = LiveThumbnail::register(
                                                    destination_hwnd,
                                                    source,
                                                )
                                            {
                                                thumbnails
                                                    .insert(preview_window.clone(), thumbnail);
                                            }
                                            let failed = thumbnails
                                                .get(&preview_window)
                                                .is_some_and(|thumbnail| {
                                                    thumbnail.update_destination(rect).is_err()
                                                });
                                            if failed {
                                                thumbnails.remove(&preview_window);
                                            }
                                        },
                                    )
                                    .absolute()
                                    .inset_0(),
                                )
                            })
                            .when(!card.preview_available, |element| {
                                element.child("Preview unavailable")
                            }),
                    )
                    .child(card.title.clone())
                    .child(
                        div()
                            .id(format!("flyout-close-{index}"))
                            .role(gpui::Role::Button)
                            .aria_label(format!("Close {}", card.title))
                            .tab_index(0)
                            .px_2()
                            .py_1()
                            .on_click(cx.listener(move |_, _, window, cx| {
                                close_action(close_effect.clone());
                                close_dismiss(window, cx);
                                cx.stop_propagation();
                            }))
                            .child("Close"),
                    )
            }))
    }
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

    pub fn entries(&self) -> Vec<(JumpListGroup, usize, &CommandDescriptor)> {
        self.groups
            .iter()
            .flat_map(|(group, commands)| {
                commands
                    .iter()
                    .enumerate()
                    .map(move |(index, command)| (*group, index, command))
            })
            .collect()
    }
}

pub struct JumpListView {
    pub model: JumpListModel,
    focused: usize,
    invoke: JumpListInvokeAction,
    dismiss: FlyoutDismissAction,
    focus: FocusHandle,
}

impl JumpListView {
    pub fn new(
        model: JumpListModel,
        invoke: JumpListInvokeAction,
        dismiss: FlyoutDismissAction,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            model,
            focused: 0,
            invoke,
            dismiss,
            focus: cx.focus_handle(),
        }
    }

    fn move_focus(&mut self, delta: i32) {
        let length = self.model.entries().len();
        if length > 0 {
            self.focused = (self.focused as i32 + delta).rem_euclid(length as i32) as usize;
        }
    }

    fn focused_command(&self) -> Option<CommandDescriptor> {
        let (group, index, _) = self.model.entries().get(self.focused).copied()?;
        self.model.invoke(group, index)
    }
}

impl Render for JumpListView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let invoke_for_key = self.invoke.clone();
        let dismiss_for_key = self.dismiss.clone();
        let entries = self
            .model
            .entries()
            .into_iter()
            .enumerate()
            .map(|(focus_index, (group, index, command))| {
                (focus_index, group, index, command.clone())
            })
            .collect::<Vec<_>>();
        div()
            .id("task-jump-list")
            .role(gpui::Role::Menu)
            .aria_label("Jump List")
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .bg(rgb(0x182028))
            .text_color(rgb(0xf4f7fa))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "up" => this.move_focus(-1),
                        "down" => this.move_focus(1),
                        "enter" => {
                            if let Some(command) = this.focused_command() {
                                invoke_for_key(&command);
                                dismiss_for_key(window, cx);
                            }
                        }
                        "escape" => dismiss_for_key(window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .children(
                entries
                    .into_iter()
                    .map(|(focus_index, group, index, command)| {
                        let invoke = self.invoke.clone();
                        let dismiss = self.dismiss.clone();
                        let enabled = command.enabled;
                        let label = command.label.clone();
                        div()
                            .id(format!("jump-list-{group:?}-{index}"))
                            .role(gpui::Role::MenuItem)
                            .aria_label(label.clone())
                            .tab_index(0)
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .when(self.focused == focus_index, |element| {
                                element.bg(rgb(0x285b8f))
                            })
                            .when(!enabled, |element| element.opacity(0.5))
                            .when(enabled, |element| {
                                element.cursor_pointer().on_click(cx.listener(
                                    move |_, _, window, cx| {
                                        invoke(&command);
                                        dismiss(window, cx);
                                    },
                                ))
                            })
                            .child(label)
                    }),
            )
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
                    preview_source: None,
                },
                PreviewCard {
                    window_id: window("two"),
                    title: "Two".into(),
                    minimized: true,
                    preview_available: true,
                    preview_source: Some(2),
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
