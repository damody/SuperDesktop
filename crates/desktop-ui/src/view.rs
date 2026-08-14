use std::{path::PathBuf, rc::Rc};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render,
    StatefulInteractiveElement, Styled, StyledImage, Window, div, img, prelude::FluentBuilder as _,
    px, rgb,
};

use crate::AccessibleNode;

pub struct DesktopView {
    pub accessible_root_name: String,
    pub items: Vec<AccessibleNode>,
    pub fallback_background: bool,
    fixed_action: Option<Rc<dyn Fn()>>,
    rendered_action: Option<Rc<dyn Fn()>>,
    keyboard_focus: Option<FocusHandle>,
    wallpaper: Option<PathBuf>,
}

impl DesktopView {
    pub fn new(items: Vec<AccessibleNode>, fallback_background: bool) -> Self {
        Self {
            accessible_root_name: "SuperDesktop".into(),
            items,
            fallback_background,
            fixed_action: None,
            rendered_action: None,
            keyboard_focus: None,
            wallpaper: None,
        }
    }

    pub fn with_fixed_action(mut self, action: Rc<dyn Fn()>) -> Self {
        self.fixed_action = Some(action);
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
}

impl Render for DesktopView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let fixed_action = self.fixed_action.clone();
        let root_action = fixed_action.clone();
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
            .on_key_down(move |event, _, _| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && let Some(action) = &root_action
                {
                    action();
                }
            })
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
                        let action = fixed_action.clone();
                        let key_action = action.clone();
                        let accessible_name = if item.selected {
                            format!("{} [selected]", item.name)
                        } else {
                            item.name.clone()
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
                            .on_click(move |_, _, _| {
                                if let Some(action) = &action {
                                    action();
                                }
                            })
                            .on_key_down(move |event, _, _| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                    && let Some(action) = &key_action
                                {
                                    action();
                                }
                            })
                            .child(div().text_size(px(32.)).child("▣"))
                            .child(div().text_center().child(item.name.clone()))
                    })),
            )
    }
}
