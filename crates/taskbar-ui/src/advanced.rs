use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use gpui::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::FluentBuilder as _, px, rgb,
};
use platform_win::common::taskbar_preview::{LiveThumbnail, ThumbnailRect};

use settings_store::TaskbarSettings;
use shell_core::{ApplicationId, WindowId};
use shell_provider_protocol::{CommandDescriptor, validate_command_tree};

use crate::taskbar_settings::CommandSurfaceTokens;

pub const HOVER_PREVIEW_DELAY_MS: u64 = 400;
pub const HOVER_PREVIEW_CLOSE_GRACE_MS: u64 = 250;
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
pub type FlyoutHoverAction = Rc<dyn Fn(bool, &mut App)>;
pub type JumpListInvokeAction = Rc<dyn Fn(&CommandDescriptor)>;

pub struct TaskFlyoutView {
    pub model: FlyoutModel,
    action: FlyoutWindowAction,
    dismiss: FlyoutDismissAction,
    hover: FlyoutHoverAction,
    focus: FocusHandle,
    destination_hwnd: isize,
    thumbnails: Rc<RefCell<BTreeMap<WindowId, LiveThumbnail>>>,
}

impl TaskFlyoutView {
    pub fn new(
        cards: Vec<PreviewCard>,
        action: FlyoutWindowAction,
        dismiss: FlyoutDismissAction,
        hover: FlyoutHoverAction,
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
            hover,
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
        let hover_action = self.hover.clone();
        let tokens = preview_tokens();
        let scale_factor = window.scale_factor();
        div()
            .id("task-group-flyout")
            .role(gpui::Role::Dialog)
            .aria_label("Window previews")
            .tab_index(0)
            .track_focus(&self.focus)
            .w(px(360.))
            .h_full()
            .p_2()
            .border_1()
            .border_color(rgb(tokens.border))
            .rounded_md()
            .flex()
            .gap_2()
            .bg(rgb(tokens.panel))
            .text_color(rgb(tokens.text))
            .on_hover(cx.listener(move |_, &hovered, _, cx| hover_action(hovered, cx)))
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
                    .flex_none()
                    .p_2()
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(self.model.focused == Some(index), |element| {
                        element.border_2().border_color(rgb(tokens.focus))
                    })
                    .hover(move |style| style.bg(rgb(tokens.hover)))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        action(activate_effect.clone());
                        dismiss(window, cx);
                    }))
                    .child(
                        div()
                            .h(px(32.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(card.title.clone()),
                            )
                            .child(
                                div()
                                    .id(format!("flyout-close-{index}"))
                                    .role(gpui::Role::Button)
                                    .aria_label(format!("Close {}", card.title))
                                    .tab_index(0)
                                    .w(px(28.))
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .hover(move |style| style.bg(rgb(tokens.hover)))
                                    .on_click(cx.listener(move |_, _, window, cx| {
                                        close_action(close_effect.clone());
                                        close_dismiss(window, cx);
                                        cx.stop_propagation();
                                    }))
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .rounded_md()
                            .bg(rgb(tokens.card))
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
            }))
    }
}

#[derive(Clone, Copy)]
struct PreviewTokens {
    panel: u32,
    card: u32,
    text: u32,
    hover: u32,
    focus: u32,
    border: u32,
}

fn preview_tokens() -> PreviewTokens {
    match std::env::var("SUPERDESKTOP_THEME").as_deref() {
        Ok("dark") => PreviewTokens {
            panel: 0x202020,
            card: 0x2b2b2b,
            text: 0xffffff,
            hover: 0x3b3b3b,
            focus: 0x60cdff,
            border: 0x454545,
        },
        Ok("high-contrast") => PreviewTokens {
            panel: 0x000000,
            card: 0x000000,
            text: 0xffffff,
            hover: 0x1f1f1f,
            focus: 0xffff00,
            border: 0xffffff,
        },
        _ => PreviewTokens {
            panel: 0xf3f3f3,
            card: 0xffffff,
            text: 0x202020,
            hover: 0xe5e5e5,
            focus: 0x0067c0,
            border: 0xd2d2d2,
        },
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HoverPreviewController {
    task: Option<String>,
    popup_hovered: bool,
    generation: u64,
}

impl HoverPreviewController {
    pub fn enter_task(&mut self, task: impl Into<String>) -> u64 {
        let task = task.into();
        if self.task.as_ref() != Some(&task) {
            self.generation = self.generation.wrapping_add(1);
            self.task = Some(task);
        }
        self.generation
    }

    pub fn leave_task(&mut self, task: &str) -> u64 {
        if self.task.as_deref() == Some(task) {
            self.generation = self.generation.wrapping_add(1);
            self.task = None;
        }
        self.generation
    }

    pub fn enter_popup(&mut self) -> u64 {
        if !self.popup_hovered {
            self.generation = self.generation.wrapping_add(1);
            self.popup_hovered = true;
        }
        self.generation
    }

    pub fn leave_popup(&mut self) -> u64 {
        if self.popup_hovered {
            self.generation = self.generation.wrapping_add(1);
            self.popup_hovered = false;
        }
        self.generation
    }

    pub fn can_open(&self, task: &str, token: u64) -> bool {
        self.generation == token && self.task.as_deref() == Some(task)
    }

    pub fn can_close(&self, token: u64) -> bool {
        self.generation == token && self.task.is_none() && !self.popup_hovered
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
    pub attention_phase_on: bool,
    pub attention_steady: bool,
    pub animation_phase: u16,
    pub badge: Option<String>,
}

impl Default for TaskOverlay {
    fn default() -> Self {
        Self {
            progress: ProgressState::None,
            attention: false,
            attention_phase_on: false,
            attention_steady: false,
            animation_phase: 0,
            badge: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskVisualState {
    pub indicator_width: f32,
    pub indicator_height: f32,
    pub indicator_color: u32,
    pub second_indicator: bool,
    pub indicator_opacity: f32,
    pub progress_fraction: Option<f32>,
    pub progress_color: Option<u32>,
    pub progress_indeterminate: bool,
    pub progress_offset: f32,
    pub attention_surface: bool,
    pub accessibility_suffix: String,
}

impl TaskVisualState {
    pub fn indicator_width_for(&self, labeled_button: bool, button_width: f32) -> f32 {
        if labeled_button {
            (button_width - 16.0).max(self.indicator_width)
        } else {
            self.indicator_width
        }
    }

    pub fn compose(
        available: bool,
        active: bool,
        minimized: bool,
        group_size: usize,
        overlay: &TaskOverlay,
        high_contrast: bool,
        reduced_motion: bool,
    ) -> Self {
        let (progress_fraction, progress_color, progress_indeterminate, progress_label) =
            match overlay.progress {
                ProgressState::None => (None, None, false, None),
                ProgressState::Indeterminate => (
                    Some(if reduced_motion { 1.0 } else { 0.22 }),
                    Some(if high_contrast { 0x00ffff } else { 0x16a34a }),
                    true,
                    Some("indeterminate progress".to_owned()),
                ),
                ProgressState::Normal(value) => {
                    let value = value.min(1000);
                    (
                        Some(f32::from(value) / 1000.0),
                        Some(if high_contrast { 0x00ffff } else { 0x16a34a }),
                        false,
                        Some(format!("normal progress {} percent", value / 10)),
                    )
                }
                ProgressState::Paused(value) => {
                    let value = value.min(1000);
                    (
                        Some(f32::from(value) / 1000.0),
                        Some(if high_contrast { 0xffff00 } else { 0xf9c74f }),
                        false,
                        Some(format!("paused progress {} percent", value / 10)),
                    )
                }
                ProgressState::Error(value) => {
                    let value = value.min(1000);
                    (
                        Some(f32::from(value) / 1000.0),
                        Some(if high_contrast { 0xff00ff } else { 0xd13438 }),
                        false,
                        Some(format!("error progress {} percent", value / 10)),
                    )
                }
            };
        let attention_surface = overlay.attention
            && (overlay.attention_phase_on || overlay.attention_steady || reduced_motion);
        let mut labels = Vec::new();
        if let Some(label) = progress_label {
            labels.push(label);
        }
        if overlay.attention {
            labels.push("requires attention".to_owned());
        }
        Self {
            indicator_width: if active || group_size > 1 { 16.0 } else { 6.0 },
            indicator_height: 3.0,
            indicator_color: if !available {
                0x7a7a7a
            } else if overlay.attention {
                if high_contrast { 0xffff00 } else { 0xff8c00 }
            } else if active {
                if high_contrast { 0x00ffff } else { 0x0067c0 }
            } else {
                0x5b8db8
            },
            second_indicator: group_size > 1,
            indicator_opacity: if minimized { 0.55 } else { 1.0 },
            progress_fraction,
            progress_color,
            progress_indeterminate,
            progress_offset: if reduced_motion {
                0.0
            } else {
                f32::from(overlay.animation_phase.min(1000)) / 1000.0
            },
            attention_surface,
            accessibility_suffix: labels.join(", "),
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
            .scan(None, |previous, (focus_index, (group, index, command))| {
                let separator = previous.is_some_and(|value| value != group);
                *previous = Some(group);
                Some((focus_index, group, index, command.clone(), separator))
            })
            .collect::<Vec<_>>();
        let tokens = CommandSurfaceTokens::current();
        div()
            .id("task-jump-list")
            .role(gpui::Role::Menu)
            .aria_label("Jump List")
            .tab_index(0)
            .track_focus(&self.focus)
            .absolute()
            .left_0()
            .top_0()
            .size_full()
            .p(px(4.))
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(tokens.border))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(2.))
            .bg(rgb(tokens.background))
            .text_color(rgb(tokens.foreground))
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
                    .map(|(focus_index, group, index, command, separator)| {
                        let invoke = self.invoke.clone();
                        let dismiss = self.dismiss.clone();
                        let enabled = command.enabled;
                        let label = command.label.clone();
                        let invoked_command = command.clone();
                        let risk = command.risk.clone();
                        div()
                            .id(format!("jump-list-{group:?}-{index}"))
                            .role(gpui::Role::MenuItem)
                            .aria_label(label.clone())
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .when(separator, |element| {
                                element.border_t_1().border_color(rgb(tokens.border))
                            })
                            .when(self.focused == focus_index, |element| {
                                element
                                    .bg(rgb(tokens.selected))
                                    .text_color(rgb(tokens.selected_foreground))
                            })
                            .when(!enabled, |element| element.opacity(0.5))
                            .when(
                                risk == shell_provider_protocol::CommandRisk::Destructive,
                                |element| element.text_color(rgb(tokens.destructive)),
                            )
                            .when(enabled, |element| {
                                element.cursor_pointer().on_click(cx.listener(
                                    move |_, _, window, cx| {
                                        invoke(&invoked_command);
                                        dismiss(window, cx);
                                    },
                                ))
                            })
                            .child(div().w(px(16.)).flex_none().child(
                                if risk == shell_provider_protocol::CommandRisk::Destructive {
                                    "×"
                                } else {
                                    "•"
                                },
                            ))
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
    fn hover_preview_generation_rejects_stale_open_and_close_timers() {
        assert_eq!(HOVER_PREVIEW_DELAY_MS, 400);
        assert_eq!(HOVER_PREVIEW_CLOSE_GRACE_MS, 250);
        let mut controller = HoverPreviewController::default();
        let first = controller.enter_task("one");
        assert!(controller.can_open("one", first));
        let leave = controller.leave_task("one");
        assert!(!controller.can_open("one", first));
        assert!(controller.can_close(leave));
        let second = controller.enter_task("two");
        assert!(!controller.can_close(leave));
        assert!(controller.can_open("two", second));
        assert!(!controller.can_open("one", first));
    }

    #[test]
    fn popup_crossing_invalidates_close_and_repeated_cycles_stay_exact() {
        let mut controller = HoverPreviewController::default();
        let open = controller.enter_task("one");
        assert!(controller.can_open("one", open));
        let close = controller.leave_task("one");
        let popup = controller.enter_popup();
        assert!(!controller.can_close(close));
        assert!(!controller.can_close(popup));
        let final_close = controller.leave_popup();
        assert!(controller.can_close(final_close));
        let repeat = controller.enter_task("one");
        assert!(controller.can_open("one", repeat));
        assert!(!controller.can_close(final_close));
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

    #[test]
    fn windows11_indicator_geometry_covers_every_running_state() {
        let overlay = TaskOverlay::default();
        let inactive = TaskVisualState::compose(true, false, false, 1, &overlay, false, false);
        let active = TaskVisualState::compose(true, true, false, 1, &overlay, false, false);
        let grouped = TaskVisualState::compose(true, false, true, 3, &overlay, false, false);
        assert_eq!(
            (inactive.indicator_width, inactive.indicator_height),
            (6.0, 3.0)
        );
        assert_eq!(
            (active.indicator_width, active.indicator_height),
            (16.0, 3.0)
        );
        assert!(grouped.second_indicator);
        assert_eq!(grouped.indicator_opacity, 0.55);
    }

    #[test]
    fn labeled_buttons_use_long_reference_indicator_and_icon_only_stays_short() {
        let visual =
            TaskVisualState::compose(true, false, false, 1, &TaskOverlay::default(), false, false);
        assert_eq!(visual.indicator_width_for(true, 160.0), 144.0);
        assert_eq!(visual.indicator_width_for(false, 44.0), 6.0);
        let grouped =
            TaskVisualState::compose(true, false, false, 3, &TaskOverlay::default(), false, false);
        assert_eq!(grouped.indicator_width_for(true, 160.0), 144.0);
    }

    #[test]
    fn progress_attention_and_accessibility_remain_independent() {
        for (progress, color, label) in [
            (
                ProgressState::Normal(400),
                0x16a34a,
                "normal progress 40 percent",
            ),
            (
                ProgressState::Paused(400),
                0xf9c74f,
                "paused progress 40 percent",
            ),
            (
                ProgressState::Error(400),
                0xd13438,
                "error progress 40 percent",
            ),
        ] {
            let overlay = TaskOverlay {
                progress,
                attention: true,
                attention_phase_on: true,
                ..TaskOverlay::default()
            };
            let visual = TaskVisualState::compose(true, false, false, 1, &overlay, false, false);
            assert_eq!(visual.progress_fraction, Some(0.4));
            assert_eq!(visual.progress_color, Some(color));
            assert!(visual.attention_surface);
            assert!(visual.accessibility_suffix.contains(label));
            assert!(visual.accessibility_suffix.contains("requires attention"));
        }
        let reduced = TaskVisualState::compose(
            true,
            false,
            false,
            1,
            &TaskOverlay {
                progress: ProgressState::Indeterminate,
                ..TaskOverlay::default()
            },
            false,
            true,
        );
        assert!(reduced.progress_indeterminate);
        assert_eq!(reduced.progress_fraction, Some(1.0));
    }

    #[test]
    fn high_contrast_and_unavailable_states_do_not_rely_on_opacity_or_color_alone() {
        let overlay = TaskOverlay {
            progress: ProgressState::Normal(500),
            attention: true,
            attention_steady: true,
            ..TaskOverlay::default()
        };
        let high_contrast = TaskVisualState::compose(true, true, false, 2, &overlay, true, false);
        assert_eq!(high_contrast.progress_color, Some(0x00ffff));
        assert_eq!(high_contrast.indicator_color, 0xffff00);
        assert!(high_contrast.second_indicator);
        assert!(high_contrast.accessibility_suffix.contains("50 percent"));
        let unavailable =
            TaskVisualState::compose(false, false, false, 1, &TaskOverlay::default(), true, false);
        assert_eq!(unavailable.indicator_color, 0x7a7a7a);
        assert_eq!(unavailable.indicator_width, 6.0);
    }
}
