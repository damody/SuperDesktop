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
}

impl Render for TaskbarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(|_, _, _| {})
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
                    .px_2()
                    .flex()
                    .items_center()
                    .on_click(|_, _, _| {})
                    .child(self.fixed_name.clone()),
            )
            .children(self.tasks.iter().map(|task| {
                div()
                    .id(task.stable_id.clone())
                    .role(gpui::Role::Button)
                    .aria_label(task.name.clone())
                    .aria_selected(task.active)
                    .tab_index(0)
                    .w(px(220.))
                    .h(px(40.))
                    .px_2()
                    .flex()
                    .items_center()
                    .on_click(|_, _, _| {})
                    .child(task.name.clone())
            }))
            .child(
                div()
                    .ml_auto()
                    .h(px(40.))
                    .px_2()
                    .flex()
                    .items_center()
                    .child(format!("{} {}", self.status.time, self.status.date)),
            )
    }
}
