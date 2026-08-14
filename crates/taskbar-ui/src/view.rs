use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, rgb,
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
            .bg(rgb(0x202020))
            .child(
                div()
                    .id("start-control")
                    .role(gpui::Role::Button)
                    .aria_label("Start")
                    .tab_index(0)
                    .on_click(|_, _, _| {})
                    .child("Start"),
            )
            .child(
                div()
                    .id("superexplorer-fixed-entry")
                    .role(gpui::Role::Button)
                    .aria_label(self.fixed_name.clone())
                    .tab_index(0)
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
                    .on_click(|_, _, _| {})
                    .child(task.name.clone())
            }))
            .child(format!("{} {}", self.status.time, self.status.date))
    }
}
