use std::collections::BTreeMap;

use shell_core::{
    ApplicationId, Generation, MonitorId, MonitorState, ShellEffect, ShellItemId, ShellState,
    WindowId, WindowState,
};

#[derive(Clone, Debug, Default)]
pub struct FakeEffectAdapter {
    effects: Vec<ShellEffect>,
}

impl FakeEffectAdapter {
    pub fn apply(&mut self, effects: impl IntoIterator<Item = ShellEffect>) {
        self.effects.extend(effects);
    }

    pub fn effects(&self) -> &[ShellEffect] {
        &self.effects
    }

    pub fn take(&mut self) -> Vec<ShellEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ShellFixtureBuilder {
    state: ShellState,
}

impl ShellFixtureBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generation(mut self, generation: u64) -> Self {
        self.state.generation = Generation(generation);
        self
    }

    pub fn monitor(mut self, id: &str, dpi: u32, primary: bool) -> Self {
        let id = MonitorId::new(id).expect("fixture monitor identity must be valid");
        self.state.monitors.insert(
            id.clone(),
            MonitorState {
                id,
                dpi_x: dpi,
                dpi_y: dpi,
                primary,
            },
        );
        self
    }

    pub fn desktop_item(mut self, id: &str, selected: bool) -> Self {
        let id = ShellItemId::new(id).expect("fixture item identity must be valid");
        self.state.desktop_items.insert(id.clone());
        if selected {
            self.state.selection.selected.insert(id);
        }
        self
    }

    pub fn window(mut self, id: &str, application_id: &str, order: u64, active: bool) -> Self {
        let id = WindowId::new(id).expect("fixture window identity must be valid");
        let application_id =
            ApplicationId::new(application_id).expect("fixture application identity must be valid");
        self.state.windows.insert(
            id.clone(),
            WindowState {
                id,
                application_id,
                title: String::new(),
                order,
                active,
                minimized: false,
            },
        );
        self
    }

    pub fn build(self) -> ShellState {
        self.state
    }

    pub fn windows(&self) -> BTreeMap<WindowId, WindowState> {
        self.state.windows.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_is_deterministic() {
        let build = || {
            ShellFixtureBuilder::new()
                .generation(4)
                .monitor("monitor:a", 144, true)
                .desktop_item("item:a", true)
                .window("window:a", "app:a", 1, true)
                .build()
        };
        assert_eq!(build(), build());
    }
}
