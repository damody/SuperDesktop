use std::{collections::BTreeMap, path::PathBuf, rc::Rc};

use gpui::{
    AppContext, Context, DragMoveEvent, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ObjectFit, ParentElement, Render, StatefulInteractiveElement, Styled, StyledImage, Window, div,
    img, prelude::FluentBuilder as _, px, rgb,
};

use crate::{AccessibleNode, MenuModel};
use shell_provider_protocol::MenuInvocation;

type ItemAction = Rc<dyn Fn(&str)>;
type ItemRecycleAction = Rc<dyn Fn(&str) -> bool>;
type ItemPermanentDeleteAction = Rc<dyn Fn(&str) -> bool>;
type ItemRenameAction = Rc<dyn Fn(&str, &str) -> bool>;
type ItemTransferAction = Rc<dyn Fn(&str, &str) -> bool>;
type ItemRepositionAction = Rc<dyn Fn(&str, f32, f32)>;
type ExternalDropAction = Rc<dyn Fn(&[PathBuf]) -> bool>;
type ContextMenuAction = Rc<dyn Fn(&str) -> Option<MenuModel>>;
type ContextInvokeAction = Rc<dyn Fn(&MenuInvocation) -> Option<String>>;
type ItemPropertiesAction = Rc<dyn Fn(&str)>;
type BackgroundNewAction = Rc<dyn Fn() -> bool>;
type RefreshAction = Rc<dyn Fn() -> Vec<AccessibleNode>>;
type SortAction = Rc<dyn Fn(&str) -> Vec<AccessibleNode>>;
type CancelTransferAction = Rc<dyn Fn()>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesktopTransferStatus {
    pub label: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub cancellable: bool,
}

#[derive(Clone)]
struct DesktopDrag {
    stable_id: String,
    label: String,
}

struct DesktopDragPreview {
    label: String,
}

impl Render for DesktopDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(0x285b8f))
            .text_color(rgb(0xffffff))
            .child(self.label.clone())
    }
}

pub struct DesktopView {
    pub accessible_root_name: String,
    pub items: Vec<AccessibleNode>,
    pub fallback_background: bool,
    fixed_action: Option<Rc<dyn Fn()>>,
    item_action: Option<ItemAction>,
    item_recycle_action: Option<ItemRecycleAction>,
    item_permanent_delete_action: Option<ItemPermanentDeleteAction>,
    item_rename_action: Option<ItemRenameAction>,
    item_transfer_action: Option<ItemTransferAction>,
    item_reposition_action: Option<ItemRepositionAction>,
    external_drop_action: Option<ExternalDropAction>,
    context_menu_action: Option<ContextMenuAction>,
    context_invoke_action: Option<ContextInvokeAction>,
    item_properties_action: Option<ItemPropertiesAction>,
    background_new_action: Option<BackgroundNewAction>,
    refresh_action: Option<RefreshAction>,
    sort_action: Option<SortAction>,
    cancel_transfer_action: Option<CancelTransferAction>,
    rendered_action: Option<Rc<dyn Fn()>>,
    keyboard_focus: Option<FocusHandle>,
    wallpaper: Option<PathBuf>,
    rename_target: Option<String>,
    rename_buffer: String,
    context_menu: Option<MenuModel>,
    context_target: Option<String>,
    item_positions: BTreeMap<String, (f32, f32)>,
    drag_position: Option<(f32, f32)>,
    drag_consumed: bool,
    transfer_status: Option<DesktopTransferStatus>,
}

impl DesktopView {
    pub fn new(items: Vec<AccessibleNode>, fallback_background: bool) -> Self {
        Self {
            accessible_root_name: "SuperDesktop".into(),
            items,
            fallback_background,
            fixed_action: None,
            item_action: None,
            item_recycle_action: None,
            item_permanent_delete_action: None,
            item_rename_action: None,
            item_transfer_action: None,
            item_reposition_action: None,
            external_drop_action: None,
            context_menu_action: None,
            context_invoke_action: None,
            item_properties_action: None,
            background_new_action: None,
            refresh_action: None,
            sort_action: None,
            cancel_transfer_action: None,
            rendered_action: None,
            keyboard_focus: None,
            wallpaper: None,
            rename_target: None,
            rename_buffer: String::new(),
            context_menu: None,
            context_target: None,
            item_positions: BTreeMap::new(),
            drag_position: None,
            drag_consumed: false,
            transfer_status: None,
        }
    }

    pub fn with_fixed_action(mut self, action: Rc<dyn Fn()>) -> Self {
        self.fixed_action = Some(action);
        self
    }

    pub fn with_item_action(mut self, action: ItemAction) -> Self {
        self.item_action = Some(action);
        self
    }

    pub fn with_item_recycle_action(mut self, action: ItemRecycleAction) -> Self {
        self.item_recycle_action = Some(action);
        self
    }

    pub fn with_item_permanent_delete_action(mut self, action: ItemPermanentDeleteAction) -> Self {
        self.item_permanent_delete_action = Some(action);
        self
    }

    pub fn with_item_rename_action(mut self, action: ItemRenameAction) -> Self {
        self.item_rename_action = Some(action);
        self
    }

    pub fn with_item_transfer_action(mut self, action: ItemTransferAction) -> Self {
        self.item_transfer_action = Some(action);
        self
    }

    pub fn with_item_reposition_action(mut self, action: ItemRepositionAction) -> Self {
        self.item_reposition_action = Some(action);
        self
    }

    pub fn with_item_positions(mut self, positions: BTreeMap<String, (f32, f32)>) -> Self {
        self.item_positions = positions;
        self
    }

    pub fn with_external_drop_action(mut self, action: ExternalDropAction) -> Self {
        self.external_drop_action = Some(action);
        self
    }

    pub fn with_context_menu_action(mut self, action: ContextMenuAction) -> Self {
        self.context_menu_action = Some(action);
        self
    }

    pub fn with_context_invoke_action(mut self, action: ContextInvokeAction) -> Self {
        self.context_invoke_action = Some(action);
        self
    }

    pub fn with_item_properties_action(mut self, action: ItemPropertiesAction) -> Self {
        self.item_properties_action = Some(action);
        self
    }

    pub fn with_background_new_action(mut self, action: BackgroundNewAction) -> Self {
        self.background_new_action = Some(action);
        self
    }

    pub fn with_refresh_action(mut self, action: RefreshAction) -> Self {
        self.refresh_action = Some(action);
        self
    }

    pub fn with_sort_action(mut self, action: SortAction) -> Self {
        self.sort_action = Some(action);
        self
    }

    pub fn with_cancel_transfer_action(mut self, action: CancelTransferAction) -> Self {
        self.cancel_transfer_action = Some(action);
        self
    }

    pub fn set_transfer_status(&mut self, status: Option<DesktopTransferStatus>) {
        self.transfer_status = status;
    }

    pub fn refresh_authoritative(&mut self) {
        if let Some(refresh) = self.refresh_action.clone() {
            self.apply_authoritative_refresh(refresh());
        }
    }

    pub fn with_rendered_action(mut self, action: Rc<dyn Fn()>) -> Self {
        self.rendered_action = Some(action);
        self
    }

    pub fn with_wallpaper(mut self, path: PathBuf) -> Self {
        self.wallpaper = Some(path);
        self
    }

    pub fn enable_keyboard_focus(mut self, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.keyboard_focus = Some(focus);
        self
    }

    fn apply_authoritative_refresh(&mut self, mut refreshed: Vec<AccessibleNode>) {
        let selected = self
            .items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.stable_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let focused = self
            .items
            .iter()
            .find(|item| item.focused)
            .map(|item| item.stable_id.as_str());
        for item in &mut refreshed {
            item.selected = selected.contains(item.stable_id.as_str());
            item.focused = focused == Some(item.stable_id.as_str());
        }
        self.items = refreshed;
    }

    fn open_context_menu(&mut self, stable_id: &str) {
        self.context_menu = self
            .context_menu_action
            .as_ref()
            .and_then(|action| action(stable_id));
        self.context_target = self.context_menu.as_ref().map(|_| stable_id.to_owned());
    }

    fn invoke_context(&mut self, index: Option<usize>) {
        let invocation = self.context_menu.as_ref().and_then(|menu| match index {
            Some(index) => menu.invoke(index),
            None => menu.invoke_focused(),
        });
        let command = invocation.as_ref().and_then(|invocation| {
            self.context_invoke_action
                .as_ref()
                .and_then(|action| action(invocation))
        });
        let target = self.context_target.clone();
        self.context_menu = None;
        self.context_target = None;
        let (Some(command), Some(target)) = (command.as_deref(), target.as_deref()) else {
            return;
        };
        match command {
            "open" => {
                if let Some(action) = &self.item_action {
                    action(target);
                }
            }
            "rename" => {
                if let Some(item) = self.items.iter().find(|item| item.stable_id == target) {
                    self.rename_target = Some(target.to_owned());
                    self.rename_buffer.clone_from(&item.name);
                }
            }
            "recycle" => {
                if self
                    .item_recycle_action
                    .as_ref()
                    .is_some_and(|action| action(target))
                    && let Some(refresh) = &self.refresh_action
                {
                    self.apply_authoritative_refresh(refresh());
                }
            }
            "properties" => {
                if let Some(action) = &self.item_properties_action {
                    action(target);
                }
            }
            "refresh" => {
                if let Some(refresh) = &self.refresh_action {
                    self.apply_authoritative_refresh(refresh());
                }
            }
            command if command.starts_with("sort-") => {
                if let Some(sort) = &self.sort_action {
                    self.apply_authoritative_refresh(sort(command));
                }
            }
            "new" => {
                if self
                    .background_new_action
                    .as_ref()
                    .is_some_and(|action| action())
                    && let Some(refresh) = &self.refresh_action
                {
                    self.apply_authoritative_refresh(refresh());
                }
            }
            _ => {}
        }
    }
}

impl Render for DesktopView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fixed_action = self.fixed_action.clone();
        let root_action = fixed_action.clone();
        let root_refresh = self.refresh_action.clone();
        let external_drop_action = self.external_drop_action.clone();
        let item_action = self.item_action.clone();
        let item_recycle_action = self.item_recycle_action.clone();
        let item_permanent_delete_action = self.item_permanent_delete_action.clone();
        let item_rename_action = self.item_rename_action.clone();
        let item_transfer_action = self.item_transfer_action.clone();
        let item_reposition_action = self.item_reposition_action.clone();
        let item_refresh_action = self.refresh_action.clone();
        let rename_target = self.rename_target.clone();
        let rename_buffer = self.rename_buffer.clone();
        let context_nodes = self
            .context_menu
            .as_ref()
            .map(MenuModel::accessible_nodes)
            .unwrap_or_default();
        let keyboard_focus = self.keyboard_focus.clone();
        let wallpaper = self.wallpaper.clone();
        let transfer_status = self.transfer_status.clone();
        let cancel_transfer_action = self.cancel_transfer_action.clone();
        let item_positions = self.item_positions.clone();
        if let Some(action) = &self.rendered_action {
            action();
        }
        let high_contrast = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("high-contrast");
        let background = if self.fallback_background {
            rgb(0x20242b)
        } else {
            rgb(0x101820)
        };
        let has_context_menu = !context_nodes.is_empty();
        let transfer_status_element = transfer_status.map(|status| {
            let percent = status
                .completed_bytes
                .saturating_mul(100)
                .checked_div(status.total_bytes)
                .unwrap_or_default();
            div()
                .id("desktop-transfer-status")
                .absolute()
                .right_4()
                .bottom_4()
                .min_w(px(280.))
                .p_3()
                .rounded_md()
                .bg(rgb(0x20242b))
                .border_1()
                .border_color(rgb(0x66717d))
                .child(format!("{} — {percent}%", status.label))
                .when(status.cancellable, |element| {
                    element.child(
                        div()
                            .id("desktop-transfer-cancel")
                            .mt_2()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(0x8f2d2d))
                            .cursor_pointer()
                            .on_click(cx.listener(move |_, _, _, _| {
                                if let Some(cancel) = &cancel_transfer_action {
                                    cancel();
                                }
                            }))
                            .child("Cancel"),
                    )
                })
        });
        let context_menu_element = div()
            .id("desktop-context-menu")
            .role(gpui::Role::Menu)
            .absolute()
            .right_4()
            .top_4()
            .min_w(px(220.))
            .p_1()
            .rounded_md()
            .bg(rgb(0x20242b))
            .border_1()
            .border_color(rgb(0x66717d))
            .flex()
            .flex_col()
            .children(context_nodes.into_iter().enumerate().map(|(index, node)| {
                div()
                    .id(node.stable_id)
                    .role(gpui::Role::MenuItem)
                    .aria_label(node.name.clone())
                    .tab_index(0)
                    .px_3()
                    .py_2()
                    .rounded_sm()
                    .when(node.focused, |element| element.bg(rgb(0x285b8f)))
                    .when(!node.enabled, |element| element.text_color(rgb(0x8b929a)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.invoke_context(Some(index));
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child(node.name)
            }));
        div()
            .id("superdesktop-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .tab_index(0)
            .when_some(keyboard_focus, |element, focus| element.track_focus(&focus))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.open_context_menu("desktop-background");
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if let Some(menu) = this.context_menu.as_mut() {
                    match event.keystroke.key.as_str() {
                        "down" => menu.move_focus(1),
                        "up" => menu.move_focus(-1),
                        "right" => {
                            menu.enter_submenu();
                        }
                        "left" => {
                            menu.leave_submenu();
                        }
                        "enter" | "space" => this.invoke_context(None),
                        "escape" => {
                            menu.dismiss();
                            this.context_menu = None;
                            this.context_target = None;
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if event.keystroke.key == "f5"
                    && let Some(refresh) = &root_refresh
                {
                    this.apply_authoritative_refresh(refresh());
                    cx.notify();
                    return;
                }
                if event.keystroke.key == "f10" && event.keystroke.modifiers.shift {
                    this.open_context_menu("desktop-background");
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && let Some(action) = &root_action
                {
                    action();
                }
            }))
            .on_drop(cx.listener(move |_, paths: &gpui::ExternalPaths, _, cx| {
                if let Some(drop) = &external_drop_action {
                    let _ = drop(paths.paths());
                    cx.notify();
                }
            }))
            .on_drag_move::<DesktopDrag>(cx.listener(
                |this, event: &DragMoveEvent<DesktopDrag>, _, _| {
                    this.drag_position = Some((
                        f32::from(event.event.position.x - event.bounds.origin.x),
                        f32::from(event.event.position.y - event.bounds.origin.y),
                    ));
                    this.drag_consumed = false;
                },
            ))
            .on_drop(cx.listener(move |this, source: &DesktopDrag, _, cx| {
                if !this.drag_consumed
                    && let Some((x, y)) = this.drag_position.take()
                {
                    this.item_positions.insert(source.stable_id.clone(), (x, y));
                    if let Some(reposition) = &item_reposition_action {
                        reposition(&source.stable_id, x, y);
                    }
                    cx.notify();
                }
                this.drag_consumed = false;
            }))
            .size_full()
            .relative()
            .flex()
            .bg(background)
            .when(high_contrast, |element| element.bg(rgb(0x000000)))
            .text_color(rgb(0xf3f7f8))
            .text_size(px(16.))
            .when_some(wallpaper, |element, path| {
                element.child(
                    img(path)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
            })
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .p_2()
                    .children(self.items.iter().enumerate().map(move |(index, item)| {
                        let fixed_action = fixed_action.clone();
                        let item_action = item_action.clone();
                        let key_item_action = item_action.clone();
                        let recycle_action = item_recycle_action.clone();
                        let permanent_delete_action = item_permanent_delete_action.clone();
                        let rename_action = item_rename_action.clone();
                        let transfer_action = item_transfer_action.clone();
                        let refresh_action = item_refresh_action.clone();
                        let stable_id = item.stable_id.clone();
                        let click_id = stable_id.clone();
                        let key_id = stable_id.clone();
                        let context_id = stable_id.clone();
                        let key_action = fixed_action.clone();
                        let renaming = rename_target.as_deref() == Some(stable_id.as_str());
                        let display_name = if renaming {
                            rename_buffer.clone()
                        } else {
                            item.name.clone()
                        };
                        let accessible_name = if item.selected {
                            format!("{} [selected]", display_name)
                        } else {
                            display_name.clone()
                        };
                        let default_column = index / 8;
                        let default_row = index % 8;
                        let (item_x, item_y) = item_positions.get(&stable_id).copied().unwrap_or((
                            8.0 + default_column as f32 * 104.0,
                            8.0 + default_row as f32 * 112.0,
                        ));
                        div()
                            .id(item.stable_id.clone())
                            .role(gpui::Role::Button)
                            .aria_label(accessible_name)
                            .tab_index(0)
                            .w(px(104.))
                            .h(px(112.))
                            .absolute()
                            .left(px(item_x))
                            .top(px(item_y))
                            .flex_none()
                            .p_2()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .cursor_pointer()
                            .on_drag(
                                DesktopDrag {
                                    stable_id: stable_id.clone(),
                                    label: item.name.clone(),
                                },
                                |data: &DesktopDrag, _, _, cx| {
                                    let label = data.label.clone();
                                    cx.new(|_| DesktopDragPreview { label })
                                },
                            )
                            .on_drop(cx.listener(move |this, source: &DesktopDrag, _, cx| {
                                if source.stable_id != stable_id
                                    && let Some(transfer) = &transfer_action
                                    && transfer(&source.stable_id, &stable_id)
                                {
                                    this.drag_consumed = true;
                                    cx.notify();
                                }
                            }))
                            .when(item.selected, |element| element.bg(rgb(0x285b8f)))
                            .when(item.focused || high_contrast, |element| {
                                element.border_2().border_color(if high_contrast {
                                    rgb(0xffff00)
                                } else {
                                    rgb(0xffffff)
                                })
                            })
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, _, _, cx| {
                                    for candidate in &mut this.items {
                                        candidate.selected = candidate.stable_id == context_id;
                                        candidate.focused = candidate.selected;
                                    }
                                    this.open_context_menu(&context_id);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                                for candidate in &mut this.items {
                                    candidate.selected = candidate.stable_id == click_id;
                                    candidate.focused = candidate.selected;
                                }
                                if this.rename_target.as_deref() != Some(click_id.as_str()) {
                                    this.rename_target = None;
                                    this.rename_buffer.clear();
                                }
                                if event.click_count() >= 2 {
                                    if click_id.contains(":superexplorer") {
                                        if let Some(action) = &fixed_action {
                                            action();
                                        }
                                    } else if let Some(action) = &item_action {
                                        action(&click_id);
                                    }
                                }
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    if this.rename_target.as_deref() == Some(key_id.as_str()) {
                                        match event.keystroke.key.as_str() {
                                            "enter" => {
                                                if let Some(rename) = &rename_action
                                                    && rename(&key_id, &this.rename_buffer)
                                                {
                                                    this.rename_target = None;
                                                    this.rename_buffer.clear();
                                                    if let Some(refresh) = &refresh_action {
                                                        this.apply_authoritative_refresh(refresh());
                                                    }
                                                }
                                            }
                                            "escape" => {
                                                this.rename_target = None;
                                                this.rename_buffer.clear();
                                            }
                                            "backspace" => {
                                                this.rename_buffer.pop();
                                            }
                                            _ if !event.keystroke.modifiers.control
                                                && !event.keystroke.modifiers.alt
                                                && !event.keystroke.modifiers.platform =>
                                            {
                                                if let Some(text) = &event.keystroke.key_char
                                                    && !text.chars().any(char::is_control)
                                                {
                                                    this.rename_buffer.push_str(text);
                                                }
                                            }
                                            _ => {}
                                        }
                                        cx.notify();
                                        return;
                                    }
                                    match event.keystroke.key.as_str() {
                                        "enter" | "space" => {
                                            if key_id.contains(":superexplorer") {
                                                if let Some(action) = &key_action {
                                                    action();
                                                }
                                            } else if let Some(action) = &key_item_action {
                                                action(&key_id);
                                            }
                                        }
                                        "delete" if !event.keystroke.modifiers.shift => {
                                            if let Some(recycle) = &recycle_action
                                                && recycle(&key_id)
                                                && let Some(refresh) = &refresh_action
                                            {
                                                this.apply_authoritative_refresh(refresh());
                                                cx.notify();
                                            }
                                        }
                                        "delete" if event.keystroke.modifiers.shift => {
                                            if let Some(delete) = &permanent_delete_action
                                                && delete(&key_id)
                                                && let Some(refresh) = &refresh_action
                                            {
                                                this.apply_authoritative_refresh(refresh());
                                                cx.notify();
                                            }
                                        }
                                        "f2" => {
                                            if let Some(item) = this
                                                .items
                                                .iter()
                                                .find(|candidate| candidate.stable_id == key_id)
                                            {
                                                this.rename_target = Some(key_id.clone());
                                                this.rename_buffer.clone_from(&item.name);
                                                cx.notify();
                                            }
                                        }
                                        "f10" if event.keystroke.modifiers.shift => {
                                            this.open_context_menu(&key_id);
                                            cx.notify();
                                        }
                                        "f5" => {
                                            if let Some(refresh) = &refresh_action {
                                                this.apply_authoritative_refresh(refresh());
                                                cx.notify();
                                            }
                                        }
                                        _ => {}
                                    }
                                },
                            ))
                            .child(div().text_size(px(32.)).child("▣"))
                            .child(
                                div()
                                    .text_center()
                                    .when(renaming, |element| {
                                        element
                                            .bg(rgb(0xffffff))
                                            .text_color(rgb(0x000000))
                                            .border_2()
                                            .border_color(rgb(0x2878d4))
                                    })
                                    .child(display_name),
                            )
                    })),
            )
            .when_some(transfer_status_element, |root, status| root.child(status))
            .when(has_context_menu, |root| root.child(context_menu_element))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccessibleAction;
    use std::cell::Cell;

    fn node(id: &str) -> AccessibleNode {
        AccessibleNode {
            stable_id: id.into(),
            name: id.into(),
            role: "button",
            selected: false,
            focused: false,
            actions: vec![AccessibleAction::Invoke],
            message_key: None,
        }
    }

    #[test]
    fn authoritative_refresh_restores_only_surviving_selection() {
        let mut selected = node("survives");
        selected.selected = true;
        selected.focused = true;
        let mut view = DesktopView::new(vec![selected, node("removed")], false);
        view.apply_authoritative_refresh(vec![node("new"), node("survives")]);
        assert!(!view.items[0].selected);
        assert!(view.items[1].selected);
        assert!(view.items[1].focused);

        view.apply_authoritative_refresh(vec![node("new")]);
        assert!(
            view.items
                .iter()
                .all(|item| !item.selected && !item.focused)
        );
    }

    #[test]
    fn context_menu_invocation_uses_the_same_typed_command_for_pointer_and_keyboard() {
        use shell_provider_protocol::{CommandDescriptor, CommandId, CommandRisk, MenuEnumeration};
        let invoked = Rc::new(Cell::new(0));
        let invoked_for_action = Rc::clone(&invoked);
        let mut view = DesktopView::new(vec![node("item")], false)
            .with_context_menu_action(Rc::new(|stable_id| {
                MenuModel::new(MenuEnumeration {
                    generation: 1,
                    selection_fingerprint: stable_id.into(),
                    commands: vec![CommandDescriptor {
                        id: CommandId("typed-open".into()),
                        label: "Open".into(),
                        enabled: true,
                        risk: CommandRisk::Normal,
                        children: Vec::new(),
                    }],
                    optional_enrichment_complete: false,
                })
                .ok()
            }))
            .with_context_invoke_action(Rc::new(|invocation| {
                (invocation.token == "typed-open").then(|| "open".into())
            }))
            .with_item_action(Rc::new(move |_| {
                invoked_for_action.set(invoked_for_action.get() + 1)
            }));
        view.open_context_menu("item");
        view.invoke_context(Some(0));
        view.open_context_menu("item");
        view.invoke_context(None);
        assert_eq!(invoked.get(), 2);
    }
}
