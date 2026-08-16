use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px, rgb,
};

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

pub type TaskViewDismissAction = Rc<dyn Fn(&mut Window, &mut gpui::App)>;
pub type TaskViewAction = Rc<dyn Fn(TaskViewEffect) -> bool>;

pub struct TaskViewSurface {
    pub model: TaskViewModel,
    dismiss: TaskViewDismissAction,
    action: TaskViewAction,
    focus: FocusHandle,
    selected_window: Option<WindowId>,
}

impl TaskViewSurface {
    pub fn new(
        mut model: TaskViewModel,
        action: TaskViewAction,
        dismiss: TaskViewDismissAction,
        cx: &mut Context<Self>,
    ) -> Self {
        model.open();
        Self {
            model,
            dismiss,
            action,
            focus: cx.focus_handle(),
            selected_window: None,
        }
    }

    fn select_window(&mut self, delta: i32) {
        let Some(desktop) = self
            .model
            .focused
            .and_then(|index| self.model.desktops.get(index))
        else {
            self.selected_window = None;
            return;
        };
        if desktop.windows.is_empty() {
            self.selected_window = None;
            return;
        }
        let current = self
            .selected_window
            .as_ref()
            .and_then(|window| desktop.windows.iter().position(|item| item == window))
            .unwrap_or(0) as i32;
        self.selected_window = Some(
            desktop.windows[(current + delta).rem_euclid(desktop.windows.len() as i32) as usize]
                .clone(),
        );
    }

    fn move_selected_to(&mut self, desktop_id: u128) -> bool {
        let Some(window_id) = self.selected_window.clone() else {
            return false;
        };
        let Some(effect) = self.model.move_window(window_id.clone(), desktop_id) else {
            return false;
        };
        if !(self.action)(effect) {
            self.model.unavailable_reason = Some("virtual-desktop-move-failed".into());
            return false;
        }
        self.model.reconcile_successful_move(&window_id, desktop_id);
        self.selected_window = None;
        true
    }
}

impl Render for TaskViewSurface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let nodes = self.model.accessibility_nodes();
        let cards = self.model.desktops.clone();
        let dismiss = self.dismiss.clone();
        let action_for_key = self.action.clone();
        let selected_window = self.selected_window.clone();
        let switch_available = self.model.capabilities.switch;
        let unavailable = (!switch_available).then_some(
            "Desktop switching is unavailable through the documented Windows API; window membership remains observable.",
        );
        div()
            .id("task-view-surface")
            .role(gpui::Role::Dialog)
            .aria_label("Task View")
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x182028))
            .text_color(rgb(0xf4f7fa))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "left" => {
                            let length = this.model.desktops.len();
                            if length > 0 {
                                let current = this.model.focused.unwrap_or(0);
                                this.model.focused = Some((current + length - 1) % length);
                            }
                        }
                        "right" => {
                            let length = this.model.desktops.len();
                            if length > 0 {
                                this.model.focused =
                                    Some((this.model.focused.unwrap_or(0) + 1) % length);
                            }
                        }
                        "up" => this.select_window(-1),
                        "down" => this.select_window(1),
                        "m" => {
                            if let Some(desktop_id) = this
                                .model
                                .focused
                                .and_then(|index| this.model.desktops.get(index))
                                .map(|desktop| desktop.id)
                            {
                                this.move_selected_to(desktop_id);
                            }
                        }
                        "enter" => {
                            if let Some(index) = this.model.focused
                                && let Some(desktop) = this.model.desktops.get(index)
                                && let Some(effect) = this.model.switch(desktop.id)
                            {
                                action_for_key(effect);
                            }
                        }
                        "escape" => {
                            this.model.dismiss();
                            dismiss(window, cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(div().text_size(px(24.)).child("Task View"))
            .when_some(unavailable, |root, message| {
                root.child(div().text_color(rgb(0xffcc66)).child(message))
            })
            .child(
                div()
                    .id("virtual-desktop-list")
                    .role(gpui::Role::TabList)
                    .aria_label("Virtual desktops")
                    .flex_1()
                    .flex()
                    .gap_3()
                    .children(nodes.into_iter().zip(cards).enumerate().map(
                        |(index, (node, card))| {
                            let switch_action = self.action.clone();
                            let move_available = self.model.capabilities.move_window;
                            let selected_for_card = selected_window.clone();
                            let selected_for_windows = selected_for_card.clone();
                            let has_selected = selected_for_card.is_some();
                            let destination_id = card.id;
                            let desktop_name = card.name.clone();
                            div()
                                .id(node.stable_id)
                                .role(gpui::Role::Tab)
                                .aria_label(format!(
                                    "{}; {} windows{}",
                                    node.name,
                                    card.windows.len(),
                                    if node.available {
                                        ""
                                    } else {
                                        "; switching unavailable"
                                    }
                                ))
                                .tab_index(0)
                                .w(px(280.))
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x2a343e))
                                .when(node.focused, |element| {
                                    element.border_2().border_color(rgb(0x4aa3ff))
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.model.focused = Some(index);
                                    if let Some(desktop) = this.model.desktops.get(index)
                                        && let Some(effect) = this.model.switch(desktop.id)
                                    {
                                        switch_action(effect);
                                    }
                                    cx.notify();
                                }))
                                .child(node.name)
                                .child(format!("{} windows", card.windows.len()))
                                .children(card.windows.into_iter().map(|window_id| {
                                    let selected =
                                        selected_for_windows.as_ref() == Some(&window_id);
                                    let display_id = window_id.to_string();
                                    div()
                                        .id(format!("task-view-window-{window_id}"))
                                        .role(gpui::Role::Button)
                                        .aria_label(format!("Select window {window_id}"))
                                        .tab_index(0)
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .when(selected, |element| element.bg(rgb(0x285b8f)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected_window = Some(window_id.clone());
                                            cx.stop_propagation();
                                            cx.notify();
                                        }))
                                        .child(display_id)
                                }))
                                .when(move_available && has_selected, |element| {
                                    element.child(
                                        div()
                                            .id(format!("task-view-move-to-{destination_id:032x}"))
                                            .role(gpui::Role::Button)
                                            .aria_label(format!(
                                                "Move selected window to {desktop_name}"
                                            ))
                                            .tab_index(0)
                                            .mt_2()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(0x285b8f))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.move_selected_to(destination_id);
                                                cx.stop_propagation();
                                                cx.notify();
                                            }))
                                            .child("Move selected here"),
                                    )
                                })
                        },
                    )),
            )
    }
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

    pub fn reconcile_successful_move(&mut self, window_id: &WindowId, desktop_id: u128) {
        for desktop in &mut self.desktops {
            desktop.windows.retain(|window| window != window_id);
            if desktop.id == desktop_id && !desktop.windows.contains(window_id) {
                desktop.windows.push(window_id.clone());
                desktop.windows.sort();
            }
        }
        self.unavailable_reason = None;
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
            [(1, vec![WindowId::new("window").unwrap()]), (2, Vec::new())]
                .into_iter()
                .collect(),
        );
        model.open();
        assert!(model.switch(1).is_none());
        assert!(model.unavailable_reason.is_some());
        assert!(matches!(
            model.move_window(WindowId::new("window").unwrap(), 2),
            Some(TaskViewEffect::MoveWindow { .. })
        ));
        model.reconcile_successful_move(&WindowId::new("window").unwrap(), 2);
        assert!(model.desktops[0].windows.is_empty());
        assert_eq!(
            model.desktops[1].windows,
            vec![WindowId::new("window").unwrap()]
        );
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
