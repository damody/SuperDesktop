use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, prelude::FluentBuilder as _, px, rgb,
};

use crate::AccessibleNode;

pub struct DesktopView {
    pub accessible_root_name: String,
    pub items: Vec<AccessibleNode>,
    pub fallback_background: bool,
    fixed_action: Option<Rc<dyn Fn()>>,
}

impl DesktopView {
    pub fn new(items: Vec<AccessibleNode>, fallback_background: bool) -> Self {
        Self {
            accessible_root_name: "SuperDesktop".into(),
            items,
            fallback_background,
            fixed_action: None,
        }
    }

    pub fn with_fixed_action(mut self, action: Rc<dyn Fn()>) -> Self {
        self.fixed_action = Some(action);
        self
    }
}

impl Render for DesktopView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let fixed_action = self.fixed_action.clone();
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
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .when(high_contrast, |element| element.bg(rgb(0x000000)))
            .text_color(rgb(0xf3f7f8))
            .text_size(px(16.))
            .child(self.accessible_root_name.clone())
            .children(self.items.iter().map(move |item| {
                let action = fixed_action.clone();
                div()
                    .id(item.stable_id.clone())
                    .role(gpui::Role::Button)
                    .aria_label(item.name.clone())
                    .aria_selected(item.selected)
                    .tab_index(0)
                    .p_2()
                    .cursor_pointer()
                    .when(high_contrast, |element| {
                        element.border_2().border_color(rgb(0xffff00))
                    })
                    .on_click(move |_, _, _| {
                        if let Some(action) = &action {
                            action();
                        }
                    })
                    .child(item.name.clone())
            }))
    }
}
