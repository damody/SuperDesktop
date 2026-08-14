use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, linear_color_stop, linear_gradient,
    prelude::FluentBuilder as _, px, rgb, svg,
};

use crate::{AccessibleTask, StatusRegion, TaskbarLayout};

pub struct TaskbarView {
    pub accessible_root_name: String,
    pub layout: TaskbarLayout,
    pub tasks: Vec<AccessibleTask>,
    pub fixed_name: String,
    pub status: StatusRegion,
    pub callbacks: Option<TaskbarCallbacks>,
    pub keyboard_focus: Option<FocusHandle>,
}

#[derive(Clone)]
pub struct TaskbarCallbacks {
    pub start: Rc<dyn Fn()>,
    pub fixed: Rc<dyn Fn()>,
    pub task: Rc<dyn Fn(&str)>,
    pub rendered: Rc<dyn Fn()>,
}

impl Render for TaskbarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let start = self.callbacks.as_ref().map(|value| Rc::clone(&value.start));
        let fixed = self.callbacks.as_ref().map(|value| Rc::clone(&value.fixed));
        let task_callback = self.callbacks.as_ref().map(|value| Rc::clone(&value.task));
        let start_key = start.clone();
        let fixed_key = fixed.clone();
        let root_fixed_key = fixed.clone();
        let keyboard_focus = self.keyboard_focus.clone();
        let high_contrast = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("high-contrast");
        let start_color = if high_contrast {
            rgb(0xffff00)
        } else {
            rgb(0x000000)
        };
        if let Some(rendered) = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.rendered))
        {
            rendered();
        }
        div()
            .id("supertaskbar-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .tab_index(0)
            .when_some(keyboard_focus, |element, focus| element.track_focus(&focus))
            .on_key_down(move |event, _, _| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && let Some(callback) = &root_fixed_key
                {
                    callback();
                }
            })
            .size_full()
            .flex()
            .flex_row()
            .items_stretch()
            .bg(linear_gradient(
                90.,
                linear_color_stop(rgb(0xd8edc0), 0.),
                linear_color_stop(rgb(0xc3efef), 1.),
            ))
            .border_t_1()
            .border_color(rgb(0xe4f4d0))
            .text_color(if high_contrast {
                rgb(0xffffff)
            } else {
                rgb(0x182220)
            })
            .when(high_contrast, |element| element.bg(rgb(0x000000)))
            .text_size(px(14.))
            .child(
                div()
                    .id("start-control")
                    .role(gpui::Role::Button)
                    .aria_label("Start")
                    .tab_index(0)
                    .w(px(48.))
                    .h(px(80.))
                    .flex_none()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(high_contrast, |element| {
                        element.border_2().border_color(rgb(0xffff00))
                    })
                    .when(!high_contrast, |element| {
                        element
                            .child(div().absolute().top_0().left_0().w_full().h(px(40.)).bg(
                                linear_gradient(
                                    90.,
                                    linear_color_stop(rgb(0xd9efbd), 0.),
                                    linear_color_stop(rgb(0xd7edc1), 1.),
                                ),
                            ))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(40.))
                                    .left_0()
                                    .w_full()
                                    .h(px(40.))
                                    .bg(linear_gradient(
                                        90.,
                                        linear_color_stop(rgb(0xd8ecb6), 0.),
                                        linear_color_stop(rgb(0xd8edb8), 1.),
                                    )),
                            )
                    })
                    .on_click(move |_, _, _| {
                        if let Some(callback) = &start {
                            callback();
                        }
                    })
                    .on_key_down(move |event, _, _| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                            && let Some(callback) = &start_key
                        {
                            callback();
                        }
                    })
                    .child(
                        svg()
                            .external_path(concat!(
                                env!("CARGO_MANIFEST_DIR"),
                                "/assets/windows-start.svg"
                            ))
                            .w(px(14.))
                            .h(px(14.))
                            .text_color(start_color),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .content_start()
                    .items_start()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("superexplorer-fixed-entry")
                            .role(gpui::Role::Button)
                            .aria_label(self.fixed_name.clone())
                            .tab_index(0)
                            .w(px(150.))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(rgb(0x0078d4))
                            .on_click(move |_, _, _| {
                                if let Some(callback) = &fixed {
                                    callback();
                                }
                            })
                            .on_key_down(move |event, _, _| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                    && let Some(callback) = &fixed_key
                                {
                                    callback();
                                }
                            })
                            .child(self.fixed_name.clone()),
                    )
                    .children(self.tasks.iter().map(move |task| {
                        let callback = task_callback.clone();
                        let key_callback = callback.clone();
                        let stable_id = task.stable_id.clone();
                        let key_stable_id = stable_id.clone();
                        let available = task.available;
                        let state = if !available {
                            "unavailable".to_owned()
                        } else if task.attention {
                            "attention".to_owned()
                        } else if task.group_size > 1 {
                            format!("group:{}", task.group_size)
                        } else if task.active {
                            "active".to_owned()
                        } else if task.minimized {
                            "minimized".to_owned()
                        } else {
                            "available".to_owned()
                        };
                        let accessible_name = format!("{} [{state}]", task.name);
                        let display_name = if task.group_size > 1 {
                            format!("{} ({})", task.name, task.group_size)
                        } else {
                            task.name.clone()
                        };
                        let underline = if !available {
                            rgb(0x6b6b6b)
                        } else if task.attention {
                            rgb(0xff8c00)
                        } else if task.group_size > 1 {
                            rgb(0x744da9)
                        } else if task.active {
                            rgb(0x0067c0)
                        } else if task.minimized {
                            rgb(0x87949a)
                        } else {
                            rgb(0x1683d8)
                        };
                        div()
                            .id(task.stable_id.clone())
                            .role(gpui::Role::Button)
                            .aria_label(accessible_name)
                            .tab_index(0)
                            .w(px(190.))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .border_b_1()
                            .border_color(underline)
                            .when(!available, |element| element.opacity(0.55))
                            .when(available, move |element| {
                                element
                                    .on_click(move |_, _, _| {
                                        if let Some(callback) = &callback {
                                            callback(&stable_id);
                                        }
                                    })
                                    .on_key_down(move |event, _, _| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                            && let Some(callback) = &key_callback
                                        {
                                            callback(&key_stable_id);
                                        }
                                    })
                            })
                            .child(display_name)
                    })),
            )
            .child(
                div()
                    .ml_auto()
                    .w(px(300.))
                    .h(px(80.))
                    .flex_none()
                    .px_2()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(self.status.time.clone())
                    .child(self.status.date.clone()),
            )
    }
}
