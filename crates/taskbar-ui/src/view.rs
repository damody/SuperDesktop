use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};

use crate::{AccessibleTask, StatusRegion, TaskbarLayout};

pub struct TaskbarView {
    pub accessible_root_name: String,
    pub layout: TaskbarLayout,
    pub tasks: Vec<AccessibleTask>,
    pub fixed_name: String,
    pub status: StatusRegion,
    pub callbacks: Option<TaskbarCallbacks>,
}

#[derive(Clone)]
pub struct TaskbarCallbacks {
    pub start: Rc<dyn Fn()>,
    pub fixed: Rc<dyn Fn()>,
    pub task: Rc<dyn Fn(&str)>,
}

impl Render for TaskbarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let start = self.callbacks.as_ref().map(|value| Rc::clone(&value.start));
        let fixed = self.callbacks.as_ref().map(|value| Rc::clone(&value.fixed));
        let task_callback = self.callbacks.as_ref().map(|value| Rc::clone(&value.task));
        div()
            .id("supertaskbar-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .size_full()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .bg(rgb(0xe8f5f2))
            .text_color(rgb(0x182220))
            .text_size(px(14.))
            .child(
                div()
                    .id("start-control")
                    .role(gpui::Role::Button)
                    .aria_label("Start")
                    .tab_index(0)
                    .w(px(48.))
                    .h(px(40.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(move |_, _, _| {
                        if let Some(callback) = &start {
                            callback();
                        }
                    })
                    .child("⊞"),
            )
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
                    .on_click(move |_, _, _| {
                        if let Some(callback) = &fixed {
                            callback();
                        }
                    })
                    .child(self.fixed_name.clone()),
            )
            .children(self.tasks.iter().map(move |task| {
                let callback = task_callback.clone();
                let stable_id = task.stable_id.clone();
                let underline = if task.active {
                    rgb(0x0067c0)
                } else if task.minimized {
                    rgb(0x87949a)
                } else {
                    rgb(0x1683d8)
                };
                div()
                    .id(task.stable_id.clone())
                    .role(gpui::Role::Button)
                    .aria_label(task.name.clone())
                    .aria_selected(task.active)
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
                    .on_click(move |_, _, _| {
                        if let Some(callback) = &callback {
                            callback(&stable_id);
                        }
                    })
                    .child(task.name.clone())
            }))
            .child(
                div()
                    .ml_auto()
                    .w(px(180.))
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
