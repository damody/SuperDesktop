use platform_win::common::monitor_dpi_start::{StartHostProbe, invoke_start_host_controlled};
use shell_provider_protocol::{
    CommandDescriptor, SearchBatch, SearchProvider, SearchProviderState, SearchQuery, SearchResult,
    rank_search_results,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    AppContext, Bounds, Context, ElementInputHandler, EntityInputHandler, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, Pixels, Render, StatefulInteractiveElement,
    Styled, UTF16Selection, Window, canvas, div, prelude::FluentBuilder as _, px, rgb,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartFailure {
    Missing,
    Refused,
    StaleHost,
    UntrustedHost,
    ShellModeDeferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartAvailability {
    Available {
        host_pid: u32,
        host_executable: String,
    },
    Unavailable(StartFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartEffect {
    Invoked { input_events: u32, restored: bool },
    OwnedOpened,
    Unavailable(StartFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartSource {
    Pointer,
    Keyboard,
    Accessibility,
}

#[derive(Clone, Debug, Default)]
pub struct StartControl {
    last_identity: Option<(u32, String)>,
}

impl StartControl {
    pub const fn route(_source: StartSource) -> &'static str {
        "controlled-start-probe"
    }
    pub fn preview_probe_and_invoke(&mut self) -> (StartAvailability, StartEffect) {
        match invoke_start_host_controlled() {
            StartHostProbe::Available {
                host_pid,
                host_executable,
                input_events_sent,
                restored,
                ..
            } => {
                self.last_identity = Some((host_pid, host_executable.clone()));
                (
                    StartAvailability::Available {
                        host_pid,
                        host_executable,
                    },
                    StartEffect::Invoked {
                        input_events: input_events_sent,
                        restored,
                    },
                )
            }
            StartHostProbe::Unavailable { reason, .. } => {
                let failure = map_reason(reason);
                (
                    StartAvailability::Unavailable(failure),
                    StartEffect::Unavailable(failure),
                )
            }
        }
    }
    pub fn revalidate_observation(
        &mut self,
        pid: u32,
        executable: &str,
        trusted: bool,
    ) -> StartAvailability {
        if !trusted {
            return StartAvailability::Unavailable(StartFailure::UntrustedHost);
        }
        if let Some((old_pid, old_executable)) = &self.last_identity
            && (*old_pid != pid || old_executable != executable)
        {
            return StartAvailability::Unavailable(StartFailure::StaleHost);
        }
        self.last_identity = Some((pid, executable.into()));
        StartAvailability::Available {
            host_pid: pid,
            host_executable: executable.into(),
        }
    }
    pub const fn shell_mode_fixture() -> StartEffect {
        StartEffect::OwnedOpened
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartAccessibilityNode {
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub focused: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StartSnapshot {
    pub initialized: bool,
    pub pinned_ids: Vec<String>,
    pub recent_ids: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct StartModel {
    pub open: bool,
    pub query: String,
    pub composition: String,
    pub generation: u64,
    pub focused_result: Option<usize>,
    pinned: Vec<SearchResult>,
    recent: Vec<SearchResult>,
    all_apps: Vec<SearchResult>,
    batches: BTreeMap<SearchProvider, Vec<SearchResult>>,
    states: BTreeMap<SearchProvider, SearchProviderState>,
    cancelled_generations: BTreeSet<u64>,
    pending_query: Option<(u64, String)>,
}

impl StartModel {
    pub fn open(&mut self) {
        self.open = true;
        self.focused_result = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.composition.clear();
        self.focused_result = None;
    }

    pub fn set_catalogs(
        &mut self,
        pinned: Vec<SearchResult>,
        recent: Vec<SearchResult>,
        all_apps: Vec<SearchResult>,
    ) {
        self.pinned = pinned;
        self.recent = recent;
        self.all_apps = all_apps;
    }

    pub fn composition_changed(&mut self, value: impl Into<String>) {
        self.composition = value.into();
    }

    pub fn commit_text(&mut self, value: impl Into<String>, now_ms: u64) {
        self.composition.clear();
        self.pending_query = Some((now_ms.saturating_add(50), value.into()));
    }

    pub fn take_debounced_query(&mut self, now_ms: u64) -> Option<SearchQuery> {
        let (due, _) = self.pending_query.as_ref()?;
        if now_ms < *due {
            return None;
        }
        let (_, value) = self.pending_query.take()?;
        Some(self.commit_query(value))
    }

    pub fn commit_query(&mut self, value: impl Into<String>) -> SearchQuery {
        if self.generation > 0 {
            self.cancelled_generations.insert(self.generation);
        }
        self.generation = self.generation.saturating_add(1);
        self.query = value.into();
        self.composition.clear();
        self.batches.clear();
        self.states = [
            (SearchProvider::Applications, SearchProviderState::Pending),
            (SearchProvider::Files, SearchProviderState::Pending),
            (SearchProvider::Settings, SearchProviderState::Pending),
        ]
        .into_iter()
        .collect();
        self.focused_result = None;
        SearchQuery {
            generation: self.generation,
            text: self.query.clone(),
            max_results: 100,
            providers: vec![
                SearchProvider::Applications,
                SearchProvider::Files,
                SearchProvider::Settings,
            ],
        }
    }

    pub fn accept_batch(&mut self, mut batch: SearchBatch) -> bool {
        if batch.generation != self.generation
            || self.cancelled_generations.contains(&batch.generation)
        {
            return false;
        }
        rank_search_results(&self.query, &mut batch.results, &BTreeMap::new());
        self.states.insert(batch.provider, batch.state);
        self.batches
            .entry(batch.provider)
            .or_default()
            .extend(batch.results);
        if self.focused_result.is_none() && !self.results().is_empty() {
            self.focused_result = Some(0);
        }
        true
    }

    pub fn results(&self) -> Vec<&SearchResult> {
        if self.query.trim().is_empty() {
            let mut seen = BTreeSet::new();
            return self
                .pinned
                .iter()
                .chain(&self.recent)
                .chain(&self.all_apps)
                .filter(|item| seen.insert(item.id.as_str()))
                .collect();
        }
        let mut output: Vec<_> = self.batches.values().flatten().collect();
        output.sort_by(|left, right| {
            right
                .score_milli
                .cmp(&left.score_milli)
                .then_with(|| left.id.cmp(&right.id))
        });
        output.truncate(100);
        output
    }

    pub fn move_focus(&mut self, delta: i32) {
        let length = self.results().len();
        if length == 0 {
            self.focused_result = None;
        } else {
            let current = self.focused_result.unwrap_or(0) as i32;
            self.focused_result = Some((current + delta).rem_euclid(length as i32) as usize);
        }
    }

    pub fn activate_focused(&self) -> Option<CommandDescriptor> {
        self.results()
            .get(self.focused_result?)
            .map(|result| result.activation.clone())
    }

    pub fn focused_result(&self) -> Option<SearchResult> {
        self.results()
            .get(self.focused_result?)
            .map(|item| (*item).clone())
    }

    pub fn record_recent(&mut self, item: SearchResult) {
        self.recent.retain(|existing| existing.id != item.id);
        self.recent.insert(0, item);
        self.recent.truncate(20);
    }

    pub fn toggle_pin(&mut self, item: SearchResult) -> bool {
        if let Some(index) = self
            .pinned
            .iter()
            .position(|existing| existing.id == item.id)
        {
            self.pinned.remove(index);
            false
        } else {
            self.pinned.push(item);
            true
        }
    }

    pub fn snapshot(&self) -> StartSnapshot {
        StartSnapshot {
            initialized: true,
            pinned_ids: self.pinned.iter().map(|item| item.id.clone()).collect(),
            recent_ids: self.recent.iter().map(|item| item.id.clone()).collect(),
        }
    }

    pub fn restore_snapshot(&mut self, snapshot: &StartSnapshot, catalog: &[SearchResult]) {
        let by_id: BTreeMap<_, _> = catalog
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();
        self.pinned = snapshot
            .pinned_ids
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).map(|item| (*item).clone()))
            .collect();
        self.recent = snapshot
            .recent_ids
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).map(|item| (*item).clone()))
            .collect();
    }

    pub fn accessibility_nodes(&self) -> Vec<StartAccessibilityNode> {
        self.results()
            .iter()
            .enumerate()
            .map(|(index, result)| StartAccessibilityNode {
                stable_id: format!("start:result:{}", result.id),
                name: result.title.clone(),
                role: "listitem",
                focused: self.focused_result == Some(index),
            })
            .collect()
    }
}

pub type SearchAction = Rc<dyn Fn(SearchQuery) -> Vec<SearchBatch>>;
pub type ActivationAction = Rc<dyn Fn(&CommandDescriptor)>;
pub type DismissAction = Rc<dyn Fn(&mut Window, &mut gpui::App)>;
pub type PersistStartAction = Rc<dyn Fn(&StartSnapshot)>;
pub type PowerAction = Rc<dyn Fn(StartPowerAction)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartPowerAction {
    SignOut,
    Restart,
    ShutDown,
}

pub struct StartActions {
    pub search: SearchAction,
    pub activate: ActivationAction,
    pub dismiss: DismissAction,
    pub persist: PersistStartAction,
    pub power: PowerAction,
}

pub struct StartView {
    pub model: StartModel,
    search: SearchAction,
    activate: ActivationAction,
    dismiss: DismissAction,
    persist: PersistStartAction,
    power: PowerAction,
    focus: FocusHandle,
}

impl StartView {
    pub fn new(
        catalogs: Vec<SearchResult>,
        snapshot: StartSnapshot,
        actions: StartActions,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut model = StartModel::default();
        model.open();
        let default_pins = catalogs.iter().take(6).cloned().collect();
        model.set_catalogs(default_pins, Vec::new(), catalogs.clone());
        if snapshot.initialized {
            model.restore_snapshot(&snapshot, &catalogs);
        }
        Self {
            model,
            search: actions.search,
            activate: actions.activate,
            dismiss: actions.dismiss,
            persist: actions.persist,
            power: actions.power,
            focus: cx.focus_handle(),
        }
    }

    fn dispatch_search(&mut self, query: SearchQuery) {
        for batch in (self.search)(query) {
            self.model.accept_batch(batch);
        }
    }

    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        self.model
            .commit_text(self.model.query.clone(), unix_time_ms());
        cx.spawn(async move |this, cx| {
            cx.background_spawn(async {
                std::thread::sleep(Duration::from_millis(50));
            })
            .await;
            this.update(cx, |this, cx| {
                if let Some(query) = this.model.take_debounced_query(unix_time_ms()) {
                    this.dispatch_search(query);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn activate_result(&mut self, result: SearchResult) {
        (self.activate)(&result.activation);
        self.model.record_recent(result);
        (self.persist)(&self.model.snapshot());
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn utf16_to_byte(text: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units >= offset {
            return byte;
        }
        units += ch.len_utf16();
        if units > offset {
            return byte;
        }
    }
    text.len()
}

fn replace_utf16(text: &mut String, range: Range<usize>, replacement: &str) {
    let start = utf16_to_byte(text, range.start);
    let end = utf16_to_byte(text, range.end);
    text.replace_range(start..end, replacement);
}

impl EntityInputHandler for StartView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = format!("{}{}", self.model.query, self.model.composition);
        let maximum = text.encode_utf16().count();
        let range = range.start.min(maximum)..range.end.min(maximum);
        *adjusted_range = Some(range.clone());
        Some(text[utf16_to_byte(&text, range.start)..utf16_to_byte(&text, range.end)].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let cursor =
            self.model.query.encode_utf16().count() + self.model.composition.encode_utf16().count();
        Some(UTF16Selection {
            range: cursor..cursor,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        if self.model.composition.is_empty() {
            None
        } else {
            let start = self.model.query.encode_utf16().count();
            Some(start..start + self.model.composition.encode_utf16().count())
        }
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.composition.is_empty() {
            self.model
                .query
                .push_str(&std::mem::take(&mut self.model.composition));
            self.schedule_search(cx);
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.model.composition.clear();
        let maximum = self.model.query.encode_utf16().count();
        replace_utf16(
            &mut self.model.query,
            range.unwrap_or(maximum..maximum),
            text,
        );
        self.schedule_search(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(range) = range {
            let query_max = self.model.query.encode_utf16().count();
            if range.start < query_max {
                replace_utf16(
                    &mut self.model.query,
                    range.start..range.end.min(query_max),
                    "",
                );
            }
        }
        self.model.composition_changed(new_text);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.model.query.encode_utf16().count())
    }
}

impl Render for StartView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let nodes = self.model.accessibility_nodes();
        let query = self.model.query.clone();
        let composition = self.model.composition.clone();
        let dismiss_for_key = self.dismiss.clone();
        let settings_activate = self.activate.clone();
        let input_entity = cx.entity();
        let input_focus = self.focus.clone();
        let sign_out = self.power.clone();
        let restart = self.power.clone();
        let shut_down = self.power.clone();
        div()
            .id("superdesktop-start")
            .role(gpui::Role::Dialog)
            .aria_label("Start")
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0x182028))
            .text_color(rgb(0xf4f7fa))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => {
                            this.model.close();
                            dismiss_for_key(window, cx);
                        }
                        "down" => this.model.move_focus(1),
                        "up" => this.model.move_focus(-1),
                        "enter" => {
                            if let Some(result) = this.model.focused_result() {
                                this.activate_result(result);
                                this.model.close();
                                dismiss_for_key(window, cx);
                            }
                        }
                        "p" if event.keystroke.modifiers.control => {
                            if let Some(result) = this.model.focused_result() {
                                this.model.toggle_pin(result);
                                (this.persist)(&this.model.snapshot());
                            }
                        }
                        "backspace" => {
                            this.model.query.pop();
                            this.schedule_search(cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("start-search-input")
                    .role(gpui::Role::TextInput)
                    .aria_label("Search apps, settings, and files")
                    .h(px(44.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .bg(rgb(0xffffff))
                    .text_color(rgb(0x111111))
                    .relative()
                    .child(
                        canvas(
                            |bounds, _, _| bounds,
                            move |bounds, _, window, cx| {
                                window.handle_input(
                                    &input_focus,
                                    ElementInputHandler::new(bounds, input_entity),
                                    cx,
                                );
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(if query.is_empty() && composition.is_empty() {
                        "Search apps, settings, and files".to_owned()
                    } else {
                        format!("{query}{composition}")
                    }),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(0xaab4be))
                    .child(if query.is_empty() {
                        "Pinned · Recent · All apps"
                    } else {
                        "Search results"
                    }),
            )
            .child(
                div()
                    .id("start-results")
                    .role(gpui::Role::List)
                    .aria_label("Start results")
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(nodes.into_iter().enumerate().map(|(index, node)| {
                        let dismiss = self.dismiss.clone();
                        div()
                            .id(node.stable_id)
                            .role(gpui::Role::ListItem)
                            .aria_label(node.name.clone())
                            .tab_index(0)
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .when(node.focused, |element| element.bg(rgb(0x285b8f)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.model.focused_result = Some(index);
                                if let Some(result) =
                                    this.model.results().get(index).map(|item| (*item).clone())
                                {
                                    this.activate_result(result);
                                }
                                this.model.close();
                                dismiss(window, cx);
                                cx.notify();
                            }))
                            .child(node.name)
                    })),
            )
            .child(
                div()
                    .h(px(42.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(0x3d4852))
                    .child(
                        div()
                            .id("start-settings")
                            .role(gpui::Role::Button)
                            .aria_label("Settings")
                            .tab_index(0)
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .on_click(move |_, _, _| {
                                settings_activate(&CommandDescriptor {
                                    id: shell_provider_protocol::CommandId(
                                        "settings:ms-settings:".into(),
                                    ),
                                    label: "Settings".into(),
                                    enabled: true,
                                    risk: shell_provider_protocol::CommandRisk::Normal,
                                    children: Vec::new(),
                                });
                            })
                            .child("Settings"),
                    )
                    .child(
                        div()
                            .id("start-power-actions")
                            .role(gpui::Role::Group)
                            .aria_label("Power")
                            .flex()
                            .gap_1()
                            .child(power_button(
                                "start-sign-out",
                                "Sign out",
                                sign_out,
                                StartPowerAction::SignOut,
                            ))
                            .child(power_button(
                                "start-restart",
                                "Restart",
                                restart,
                                StartPowerAction::Restart,
                            ))
                            .child(power_button(
                                "start-shut-down",
                                "Shut down",
                                shut_down,
                                StartPowerAction::ShutDown,
                            )),
                    ),
            )
    }
}

fn power_button(
    id: &'static str,
    label: &'static str,
    action: PowerAction,
    value: StartPowerAction,
) -> impl IntoElement {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .tab_index(0)
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .on_click(move |_, _, _| action(value))
        .child(label)
}
fn map_reason(reason: &str) -> StartFailure {
    if reason.contains("refus") || reason.contains("input") {
        StartFailure::Refused
    } else if reason.contains("trust") || reason.contains("identity") {
        StartFailure::UntrustedHost
    } else {
        StartFailure::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_refused_and_stale_are_truthful() {
        assert_eq!(
            StartControl::route(StartSource::Pointer),
            StartControl::route(StartSource::Keyboard)
        );
        assert_eq!(
            StartControl::route(StartSource::Keyboard),
            StartControl::route(StartSource::Accessibility)
        );
        assert_eq!(map_reason("start-input-failed"), StartFailure::Refused);
        assert_eq!(map_reason("missing"), StartFailure::Missing);
        let mut control = StartControl::default();
        assert!(matches!(
            control.revalidate_observation(1, "trusted.exe", true),
            StartAvailability::Available { .. }
        ));
        assert_eq!(
            control.revalidate_observation(2, "trusted.exe", true),
            StartAvailability::Unavailable(StartFailure::StaleHost)
        );
        assert_eq!(StartControl::shell_mode_fixture(), StartEffect::OwnedOpened)
    }

    fn result(id: &str, title: &str) -> SearchResult {
        use shell_provider_protocol::{CommandId, CommandRisk, SearchCategory};
        SearchResult {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            category: SearchCategory::Application,
            score_milli: 0,
            activation: CommandDescriptor {
                id: CommandId(format!("open:{id}")),
                label: "Open".into(),
                enabled: true,
                risk: CommandRisk::Normal,
                children: Vec::new(),
            },
        }
    }

    #[test]
    fn owned_start_ime_generation_merge_navigation_and_accessibility() {
        let mut start = StartModel::default();
        start.open();
        start.composition_changed("設");
        assert_eq!(start.generation, 0);
        start.commit_text("pending", 100);
        assert!(start.take_debounced_query(149).is_none());
        assert!(start.take_debounced_query(150).is_some());
        let first = start.commit_query("term");
        let second = start.commit_query("terminal");
        assert!(second.generation > first.generation);
        assert!(!start.accept_batch(SearchBatch {
            generation: first.generation,
            provider: SearchProvider::Applications,
            state: SearchProviderState::Complete,
            results: vec![result("old", "old")]
        }));
        assert!(start.accept_batch(SearchBatch {
            generation: second.generation,
            provider: SearchProvider::Applications,
            state: SearchProviderState::Complete,
            results: vec![result("terminal", "Terminal")]
        }));
        assert_eq!(start.results().len(), 1);
        assert!(start.activate_focused().is_some());
        assert_eq!(start.accessibility_nodes()[0].role, "listitem");
        let focused = start.focused_result().unwrap();
        start.record_recent(focused.clone());
        assert!(start.toggle_pin(focused.clone()));
        assert!(!start.toggle_pin(focused));
        let snapshot = start.snapshot();
        assert!(snapshot.initialized);
        assert_eq!(snapshot.recent_ids, vec!["terminal"]);
        start.close();
        assert!(!start.open);
    }
}
