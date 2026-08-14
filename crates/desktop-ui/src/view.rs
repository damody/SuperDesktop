use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, rgb,
};

use crate::AccessibleNode;

pub struct DesktopView {
    pub accessible_root_name: String,
    pub items: Vec<AccessibleNode>,
    pub fallback_background: bool,
}

impl DesktopView {
    pub fn new(items: Vec<AccessibleNode>, fallback_background: bool) -> Self {
        Self {
            accessible_root_name: "SuperDesktop".into(),
            items,
            fallback_background,
        }
    }
}

impl Render for DesktopView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let background = if self.fallback_background {
            rgb(0x20242b)
        } else {
            rgb(0x101820)
        };
        div()
            .id("superdesktop-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .size_full()
            .bg(background)
            .child(self.accessible_root_name.clone())
            .children(self.items.iter().map(|item| {
                div()
                    .id(item.stable_id.clone())
                    .role(gpui::Role::Button)
                    .aria_label(item.name.clone())
                    .aria_selected(item.selected)
                    .tab_index(0)
                    .on_click(|_, _, _| {})
                    .child(item.name.clone())
            }))
    }
}
