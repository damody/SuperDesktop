use std::{path::PathBuf, rc::Rc};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render,
    StatefulInteractiveElement, Styled, StyledImage, Window, div, img, prelude::FluentBuilder as _,
    px, rgb,
};

use crate::AccessibleNode;

type ItemAction = Rc<dyn Fn(&str)>;
type ItemRecycleAction = Rc<dyn Fn(&str) -> bool>;
type ItemPermanentDeleteAction = Rc<dyn Fn(&str) -> bool>;
type ItemRenameAction = Rc<dyn Fn(&str, &str) -> bool>;
type RefreshAction = Rc<dyn Fn() -> Vec<AccessibleNode>>;

pub struct DesktopView {
    pub accessible_root_name: String,
    pub items: Vec<AccessibleNode>,
    pub fallback_background: bool,
    fixed_action: Option<Rc<dyn Fn()>>,
    item_action: Option<ItemAction>,
    item_recycle_action: Option<ItemRecycleAction>,
    item_permanent_delete_action: Option<ItemPermanentDeleteAction>,
    item_rename_action: Option<ItemRenameAction>,
    refresh_action: Option<RefreshAction>,
    rendered_action: Option<Rc<dyn Fn()>>,
    keyboard_focus: Option<FocusHandle>,
    wallpaper: Option<PathBuf>,
    rename_target: Option<String>,
    rename_buffer: String,
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
            refresh_action: None,
            rendered_action: None,
            keyboard_focus: None,
            wallpaper: None,
            rename_target: None,
            rename_buffer: String::new(),
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

    pub fn with_refresh_action(mut self, action: RefreshAction) -> Self {
        self.refresh_action = Some(action);
        self
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
}

impl Render for DesktopView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fixed_action = self.fixed_action.clone();
        let root_action = fixed_action.clone();
        let root_refresh = self.refresh_action.clone();
        let item_action = self.item_action.clone();
        let item_recycle_action = self.item_recycle_action.clone();
        let item_permanent_delete_action = self.item_permanent_delete_action.clone();
        let item_rename_action = self.item_rename_action.clone();
        let item_refresh_action = self.refresh_action.clone();
        let rename_target = self.rename_target.clone();
        let rename_buffer = self.rename_buffer.clone();
        let keyboard_focus = self.keyboard_focus.clone();
        let wallpaper = self.wallpaper.clone();
        if let Some(action) = &self.rendered_action {
            action();
        }
        let high_contrast = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("high-contrast");
        let background = if self.fallback_background {
            rgb(0x20242b)
        } else {
            rgb(0x101820)
        };
        div()
            .id("superdesktop-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .tab_index(0)
            .when_some(keyboard_focus, |element, focus| element.track_focus(&focus))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "f5"
                    && let Some(refresh) = &root_refresh
                {
                    this.apply_authoritative_refresh(refresh());
                    cx.notify();
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && let Some(action) = &root_action
                {
                    action();
                }
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
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .content_start()
                    .items_start()
                    .children(self.items.iter().map(move |item| {
                        let fixed_action = fixed_action.clone();
                        let item_action = item_action.clone();
                        let key_item_action = item_action.clone();
                        let recycle_action = item_recycle_action.clone();
                        let permanent_delete_action = item_permanent_delete_action.clone();
                        let rename_action = item_rename_action.clone();
                        let refresh_action = item_refresh_action.clone();
                        let stable_id = item.stable_id.clone();
                        let click_id = stable_id.clone();
                        let key_id = stable_id.clone();
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
                        div()
                            .id(item.stable_id.clone())
                            .role(gpui::Role::Button)
                            .aria_label(accessible_name)
                            .tab_index(0)
                            .w(px(104.))
                            .h(px(112.))
                            .flex_none()
                            .p_2()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .cursor_pointer()
                            .when(item.selected, |element| element.bg(rgb(0x285b8f)))
                            .when(item.focused || high_contrast, |element| {
                                element.border_2().border_color(if high_contrast {
                                    rgb(0xffff00)
                                } else {
                                    rgb(0xffffff)
                                })
                            })
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccessibleAction;

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
}
