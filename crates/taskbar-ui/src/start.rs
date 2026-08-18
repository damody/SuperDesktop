use platform_win::common::monitor_dpi_start::{StartHostProbe, invoke_start_host_controlled};
use shell_provider_protocol::{
    CommandDescriptor, SearchBatch, SearchProvider, SearchProviderState, SearchQuery, SearchResult,
    rank_search_results,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    AppContext, Bounds, Context, ElementInputHandler, EntityInputHandler, FocusHandle,
    InteractiveElement, IntoElement, ObjectFit, ParentElement, Pixels, Render, RenderImage,
    StatefulInteractiveElement, Styled, StyledImage, UTF16Selection, Window, canvas, div, img,
    prelude::FluentBuilder as _, px, rgb,
};
use shell_provider_protocol::{IconData, SearchCategory};

use crate::view::icon_render_image;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StartPage {
    #[default]
    Home,
    AllApps,
}

#[derive(Clone, Debug, Default)]
pub struct StartModel {
    pub open: bool,
    pub query: String,
    pub composition: String,
    pub page: StartPage,
    pub power_open: bool,
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
        self.page = StartPage::Home;
        self.power_open = false;
        self.focused_result = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.composition.clear();
        self.power_open = false;
        self.focused_result = None;
    }

    pub fn show_home(&mut self) {
        self.page = StartPage::Home;
        self.power_open = false;
        self.focused_result = None;
    }

    pub fn show_all_apps(&mut self) {
        self.page = StartPage::AllApps;
        self.power_open = false;
        self.focused_result = (!self.all_apps_results().is_empty()).then_some(0);
    }

    pub fn toggle_power(&mut self) {
        self.power_open = !self.power_open;
    }

    pub fn dismiss_power(&mut self) -> bool {
        std::mem::take(&mut self.power_open)
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
            return match self.page {
                StartPage::Home => self
                    .home_pins()
                    .into_iter()
                    .chain(self.recommendations())
                    .collect(),
                StartPage::AllApps => self.all_apps_results(),
            };
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

    pub fn home_pins(&self) -> Vec<&SearchResult> {
        let mut seen = BTreeSet::new();
        self.pinned
            .iter()
            .filter(|item| seen.insert(item.id.as_str()))
            .take(12)
            .collect()
    }

    pub fn recommendations(&self) -> Vec<&SearchResult> {
        let pinned = self
            .pinned
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        self.recent
            .iter()
            .filter(|item| !pinned.contains(item.id.as_str()))
            .filter(|item| seen.insert(item.id.as_str()))
            .take(6)
            .collect()
    }

    pub fn all_apps_results(&self) -> Vec<&SearchResult> {
        let mut apps = self
            .all_apps
            .iter()
            .filter(|item| item.category == shell_provider_protocol::SearchCategory::Application)
            .collect::<Vec<_>>();
        apps.sort_by(|left, right| {
            left.title
                .to_lowercase()
                .cmp(&right.title.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        apps.truncate(100);
        apps
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StartStrings {
    start: &'static str,
    search_placeholder: &'static str,
    pinned: &'static str,
    all_apps: &'static str,
    recommended: &'static str,
    recent_empty: &'static str,
    back_to_pinned: &'static str,
    search_results: &'static str,
    account_prefix: &'static str,
    settings: &'static str,
    power: &'static str,
    footer_actions: &'static str,
    power_options: &'static str,
    sign_out: &'static str,
    restart: &'static str,
    shut_down: &'static str,
    user_fallback: &'static str,
}

impl StartStrings {
    const ENGLISH: Self = Self {
        start: "Start",
        search_placeholder: "Search apps, settings, and files",
        pinned: "Pinned",
        all_apps: "All apps",
        recommended: "Recommended",
        recent_empty: "Recent apps and files will appear here.",
        back_to_pinned: "Back to pinned",
        search_results: "Search results",
        account_prefix: "User account",
        settings: "Settings",
        power: "Power",
        footer_actions: "Start footer actions",
        power_options: "Power options",
        sign_out: "Sign out",
        restart: "Restart",
        shut_down: "Shut down",
        user_fallback: "User",
    };

    const TRADITIONAL_CHINESE: Self = Self {
        start: "開始",
        search_placeholder: "搜尋應用程式、設定及檔案",
        pinned: "已釘選",
        all_apps: "所有應用程式",
        recommended: "建議",
        recent_empty: "最近使用的應用程式和檔案將顯示在這裡。",
        back_to_pinned: "返回已釘選",
        search_results: "搜尋結果",
        account_prefix: "使用者帳戶",
        settings: "設定",
        power: "電源",
        footer_actions: "開始功能表動作",
        power_options: "電源選項",
        sign_out: "登出",
        restart: "重新啟動",
        shut_down: "關機",
        user_fallback: "使用者",
    };

    fn from_locale(locale: Option<&str>) -> Self {
        if locale.is_some_and(|locale| locale.eq_ignore_ascii_case("zh-TW")) {
            Self::TRADITIONAL_CHINESE
        } else {
            Self::ENGLISH
        }
    }

    fn current() -> Self {
        let locale = std::env::var("SUPERDESKTOP_LOCALE")
            .ok()
            .or_else(platform_win::common::taskbar_status::user_locale_name);
        Self::from_locale(locale.as_deref())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StartVisualTokens {
    panel: u32,
    text: u32,
    secondary_text: u32,
    surface: u32,
    subtle_surface: u32,
    hover: u32,
    pressed: u32,
    border: u32,
    focus: u32,
}

impl StartVisualTokens {
    const fn new(high_contrast: bool) -> Self {
        if high_contrast {
            Self {
                panel: 0x000000,
                text: 0xffffff,
                secondary_text: 0xffffff,
                surface: 0x000000,
                subtle_surface: 0x000000,
                hover: 0x1f1f1f,
                pressed: 0x333333,
                border: 0xffffff,
                focus: 0xffff00,
            }
        } else {
            Self {
                panel: 0xf3f3f3,
                text: 0x202020,
                secondary_text: 0x616161,
                surface: 0xfbfbfb,
                subtle_surface: 0xe9e9e9,
                hover: 0xe5e5e5,
                pressed: 0xdadada,
                border: 0xd2d2d2,
                focus: 0x0067c0,
            }
        }
    }
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
    icon_cache: BTreeMap<String, Option<IconData>>,
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
        let mut default_pins = catalogs
            .iter()
            .filter(|item| item.category == SearchCategory::Application)
            .take(12)
            .cloned()
            .collect::<Vec<_>>();
        if default_pins.len() < 12 {
            let pinned = default_pins
                .iter()
                .map(|item| item.id.clone())
                .collect::<BTreeSet<_>>();
            default_pins.extend(
                catalogs
                    .iter()
                    .filter(|item| !pinned.contains(&item.id))
                    .take(12 - default_pins.len())
                    .cloned(),
            );
        }
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
            icon_cache: BTreeMap::new(),
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

    fn render_icons_for(
        &mut self,
        results: impl IntoIterator<Item = SearchResult>,
    ) -> BTreeMap<String, Option<Arc<RenderImage>>> {
        let results = results.into_iter().collect::<Vec<_>>();
        let live = results
            .iter()
            .map(|result| result.id.clone())
            .collect::<BTreeSet<_>>();
        self.icon_cache.retain(|id, _| live.contains(id));
        results
            .into_iter()
            .map(|result| {
                let icon = self
                    .icon_cache
                    .entry(result.id.clone())
                    .or_insert_with(|| {
                        result_path(&result).and_then(|path| {
                            platform_win::common::icon::shell_icon_for_path(path, 32)
                        })
                    })
                    .as_ref()
                    .and_then(icon_render_image);
                (result.id, icon)
            })
            .collect()
    }

    fn render_windows11(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        window.focus(&self.focus, cx);
        let query = self.model.query.clone();
        let composition = self.model.composition.clone();
        let query_active = !query.trim().is_empty();
        let home_active = !query_active && self.model.page == StartPage::Home;
        let all_apps_active = !query_active && self.model.page == StartPage::AllApps;
        let pins = self
            .model
            .home_pins()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let recommendations = self
            .model
            .recommendations()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let list_results = if query_active {
            self.model
                .results()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        } else if all_apps_active {
            self.model
                .all_apps_results()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let icons = self.render_icons_for(
            pins.iter()
                .chain(&recommendations)
                .chain(&list_results)
                .cloned(),
        );
        let home_icons = icons.clone();
        let list_icons = icons;
        let focused_id = self.model.focused_result().map(|result| result.id);
        let dismiss_for_key = self.dismiss.clone();
        let input_entity = cx.entity();
        let input_focus = self.focus.clone();
        let account_activate = self.activate.clone();
        let settings_activate = self.activate.clone();
        let settings_dismiss = self.dismiss.clone();
        let power_open = self.model.power_open;
        let sign_out = self.power.clone();
        let restart = self.power.clone();
        let shut_down = self.power.clone();
        let strings = StartStrings::current();
        let high_contrast = std::env::var("SUPERDESKTOP_THEME")
            .is_ok_and(|value| value.eq_ignore_ascii_case("high-contrast"));
        let tokens = StartVisualTokens::new(high_contrast);
        let account_name =
            std::env::var("USERNAME").unwrap_or_else(|_| strings.user_fallback.into());
        let account_initial = account_name.chars().next().unwrap_or('U').to_string();

        div()
            .id("windows11-start-surface")
            .role(gpui::Role::Dialog)
            .aria_label(strings.start)
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .relative()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(tokens.border))
            .bg(rgb(tokens.panel))
            .text_color(rgb(tokens.text))
            .shadow_lg()
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => {
                            if !this.model.dismiss_power() {
                                this.model.close();
                                dismiss_for_key(window, cx);
                            }
                        }
                        "down" | "right" => this.model.move_focus(1),
                        "up" | "left" => this.model.move_focus(-1),
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
                    .aria_label(strings.search_placeholder)
                    .h(px(44.))
                    .px_3()
                    .flex_none()
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .bg(rgb(tokens.surface))
                    .border_1()
                    .border_color(rgb(tokens.border))
                    .text_color(rgb(tokens.text))
                    .relative()
                    .hover(move |style| style.bg(rgb(tokens.hover)))
                    .focus_visible(move |style| style.border_2().border_color(rgb(tokens.focus)))
                    .child(div().mr_2().text_size(px(16.)).child("⌕"))
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
                        strings.search_placeholder.to_owned()
                    } else {
                        format!("{query}{composition}")
                    }),
            )
            .when(home_active, |root| {
                root.child(
                    div()
                        .id("start-home")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_size(px(18.)).child(strings.pinned))
                                .child(
                                    div()
                                        .id("start-all-apps")
                                        .role(gpui::Role::Button)
                                        .aria_label(strings.all_apps)
                                        .tab_index(0)
                                        .px_3()
                                        .py_1()
                                        .rounded_md()
                                        .bg(rgb(tokens.subtle_surface))
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(rgb(tokens.hover)))
                                        .active(move |style| style.bg(rgb(tokens.pressed)))
                                        .focus_visible(move |style| {
                                            style.border_2().border_color(rgb(tokens.focus))
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.model.show_all_apps();
                                            cx.notify();
                                        }))
                                        .child(format!("{}  ›", strings.all_apps)),
                                ),
                        )
                        .child(
                            div()
                                .id("start-pinned-grid")
                                .role(gpui::Role::List)
                                .aria_label(strings.pinned)
                                .flex()
                                .flex_wrap()
                                .children(pins.into_iter().map(|result| {
                                    let icon = home_icons.get(&result.id).cloned().flatten();
                                    let focused = focused_id.as_deref() == Some(result.id.as_str());
                                    let activate_result = result.clone();
                                    let dismiss = self.dismiss.clone();
                                    div()
                                        .id(format!("start:pinned:{}", result.id))
                                        .role(gpui::Role::ListItem)
                                        .aria_label(result.title.clone())
                                        .tab_index(0)
                                        .w(px(94.))
                                        .h(px(82.))
                                        .p_1()
                                        .rounded_md()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap_1()
                                        .cursor_pointer()
                                        .when(focused, |element| element.bg(rgb(tokens.hover)))
                                        .hover(move |style| style.bg(rgb(tokens.hover)))
                                        .active(move |style| style.bg(rgb(tokens.pressed)))
                                        .focus_visible(move |style| {
                                            style.border_2().border_color(rgb(tokens.focus))
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.activate_result(activate_result.clone());
                                            this.model.close();
                                            dismiss(window, cx);
                                        }))
                                        .child(start_icon_tile(icon, result.category.clone(), 34.0))
                                        .child(
                                            div()
                                                .w_full()
                                                .text_center()
                                                .text_size(px(12.))
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .child(result.title),
                                        )
                                })),
                        )
                        .child(div().text_size(px(18.)).child(strings.recommended))
                        .child(
                            div()
                                .id("start-recommended")
                                .role(gpui::Role::List)
                                .aria_label(strings.recommended)
                                .flex()
                                .flex_wrap()
                                .when(recommendations.is_empty(), |element| {
                                    element.child(
                                        div()
                                            .py_3()
                                            .text_color(rgb(tokens.secondary_text))
                                            .child(strings.recent_empty),
                                    )
                                })
                                .children(recommendations.into_iter().map(|result| {
                                    let icon = home_icons.get(&result.id).cloned().flatten();
                                    let activate_result = result.clone();
                                    let dismiss = self.dismiss.clone();
                                    div()
                                        .id(format!("start:recommended:{}", result.id))
                                        .role(gpui::Role::ListItem)
                                        .aria_label(result.title.clone())
                                        .tab_index(0)
                                        .w(px(282.))
                                        .h(px(54.))
                                        .px_2()
                                        .rounded_md()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(rgb(tokens.hover)))
                                        .active(move |style| style.bg(rgb(tokens.pressed)))
                                        .focus_visible(move |style| {
                                            style.border_2().border_color(rgb(tokens.focus))
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.activate_result(activate_result.clone());
                                            this.model.close();
                                            dismiss(window, cx);
                                        }))
                                        .child(start_icon_tile(icon, result.category.clone(), 32.0))
                                        .child(
                                            div().min_w_0().flex_1().child(result.title).when_some(
                                                result.subtitle,
                                                |element, subtitle| {
                                                    element.child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(tokens.secondary_text))
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .text_ellipsis()
                                                            .child(subtitle),
                                                    )
                                                },
                                            ),
                                        )
                                })),
                        ),
                )
            })
            .when(!home_active, |root| {
                root.child(
                    div()
                        .id(if query_active {
                            "start-search-results"
                        } else {
                            "start-all-apps-page"
                        })
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .when(all_apps_active, |element| {
                                    element.child(
                                        div()
                                            .id("start-back-home")
                                            .role(gpui::Role::Button)
                                            .aria_label(strings.back_to_pinned)
                                            .tab_index(0)
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(tokens.subtle_surface))
                                            .cursor_pointer()
                                            .hover(move |style| style.bg(rgb(tokens.hover)))
                                            .active(move |style| style.bg(rgb(tokens.pressed)))
                                            .focus_visible(move |style| {
                                                style.border_2().border_color(rgb(tokens.focus))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.model.show_home();
                                                cx.notify();
                                            }))
                                            .child("‹"),
                                    )
                                })
                                .child(div().text_size(px(18.)).child(if query_active {
                                    strings.search_results
                                } else {
                                    strings.all_apps
                                })),
                        )
                        .child(
                            div()
                                .id("start-mode-results")
                                .role(gpui::Role::List)
                                .aria_label(if query_active {
                                    strings.search_results
                                } else {
                                    strings.all_apps
                                })
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .children(list_results.into_iter().map(|result| {
                                    let icon = list_icons.get(&result.id).cloned().flatten();
                                    let focused = focused_id.as_deref() == Some(result.id.as_str());
                                    let activate_result = result.clone();
                                    let dismiss = self.dismiss.clone();
                                    let subtitle =
                                        query_active.then(|| result.subtitle.clone()).flatten();
                                    div()
                                        .id(format!("start:list:{}", result.id))
                                        .role(gpui::Role::ListItem)
                                        .aria_label(result.title.clone())
                                        .tab_index(0)
                                        .min_h(px(50.))
                                        .px_3()
                                        .py_1()
                                        .rounded_md()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .cursor_pointer()
                                        .when(focused, |element| element.bg(rgb(tokens.hover)))
                                        .hover(move |style| style.bg(rgb(tokens.hover)))
                                        .active(move |style| style.bg(rgb(tokens.pressed)))
                                        .focus_visible(move |style| {
                                            style.border_2().border_color(rgb(tokens.focus))
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.activate_result(activate_result.clone());
                                            this.model.close();
                                            dismiss(window, cx);
                                        }))
                                        .child(start_icon_tile(icon, result.category.clone(), 32.0))
                                        .child(
                                            div().min_w_0().flex_1().child(result.title).when_some(
                                                subtitle,
                                                |element, subtitle| {
                                                    element.child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(tokens.secondary_text))
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .text_ellipsis()
                                                            .child(subtitle),
                                                    )
                                                },
                                            ),
                                        )
                                })),
                        ),
                )
            })
            .child(
                div()
                    .id("start-footer")
                    .h(px(52.))
                    .px_2()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(tokens.border))
                    .child(
                        div()
                            .id("start-account")
                            .role(gpui::Role::Button)
                            .aria_label(format!("{} {account_name}", strings.account_prefix))
                            .tab_index(0)
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_2()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(move |_, _, _| {
                                account_activate(&settings_command(
                                    "ms-settings:yourinfo",
                                    strings.account_prefix,
                                ));
                            })
                            .child(
                                div()
                                    .w(px(28.))
                                    .h(px(28.))
                                    .rounded_full()
                                    .bg(rgb(tokens.subtle_surface))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(account_initial),
                            )
                            .child(account_name),
                    )
                    .child(
                        div()
                            .id("start-footer-actions")
                            .role(gpui::Role::Group)
                            .aria_label(strings.footer_actions)
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id("start-settings")
                                    .role(gpui::Role::Button)
                                    .aria_label(strings.settings)
                                    .tab_index(0)
                                    .w(px(40.))
                                    .h(px(40.))
                                    .rounded_md()
                                    .bg(rgb(tokens.subtle_surface))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(move |style| style.bg(rgb(tokens.hover)))
                                    .active(move |style| style.bg(rgb(tokens.pressed)))
                                    .focus_visible(move |style| {
                                        style.border_2().border_color(rgb(tokens.focus))
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        settings_activate(&settings_command(
                                            "ms-settings:",
                                            strings.settings,
                                        ));
                                        this.model.close();
                                        settings_dismiss(window, cx);
                                    }))
                                    .child("⚙"),
                            )
                            .child(
                                div()
                                    .id("start-power")
                                    .role(gpui::Role::Button)
                                    .aria_label(strings.power)
                                    .tab_index(0)
                                    .w(px(40.))
                                    .h(px(40.))
                                    .rounded_md()
                                    .bg(rgb(tokens.subtle_surface))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(move |style| style.bg(rgb(tokens.hover)))
                                    .active(move |style| style.bg(rgb(tokens.pressed)))
                                    .focus_visible(move |style| {
                                        style.border_2().border_color(rgb(tokens.focus))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.model.toggle_power();
                                        cx.notify();
                                    }))
                                    .child("⏻"),
                            ),
                    ),
            )
            .when(power_open, |root| {
                root.child(
                    div()
                        .id("start-power-menu")
                        .role(gpui::Role::Menu)
                        .aria_label(strings.power_options)
                        .absolute()
                        .right(px(16.))
                        .bottom(px(66.))
                        .w(px(190.))
                        .p_1()
                        .rounded_lg()
                        .bg(rgb(tokens.surface))
                        .border_1()
                        .border_color(rgb(tokens.border))
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .child(power_menu_item(
                            "start-sign-out",
                            strings.sign_out,
                            tokens,
                            sign_out,
                            StartPowerAction::SignOut,
                            cx,
                        ))
                        .child(power_menu_item(
                            "start-restart",
                            strings.restart,
                            tokens,
                            restart,
                            StartPowerAction::Restart,
                            cx,
                        ))
                        .child(power_menu_item(
                            "start-shut-down",
                            strings.shut_down,
                            tokens,
                            shut_down,
                            StartPowerAction::ShutDown,
                            cx,
                        )),
                )
            })
    }
}

fn result_path(result: &SearchResult) -> Option<&Path> {
    let path = result.activation.id.0.strip_prefix("open:")?;
    let path = Path::new(path);
    path.is_file().then_some(path)
}

fn start_icon_tile(
    icon: Option<Arc<RenderImage>>,
    category: SearchCategory,
    edge: f32,
) -> impl IntoElement {
    let fallback = match category {
        SearchCategory::Application => "APP",
        SearchCategory::Setting => "SET",
        SearchCategory::File => "FILE",
        SearchCategory::Command => "CMD",
    };
    let missing = icon.is_none();
    div()
        .w(px(edge))
        .h(px(edge))
        .flex_none()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(0xe7eef8))
        .text_color(rgb(0x245a9a))
        .text_size(px(9.))
        .when_some(icon, |element, icon| {
            element.child(
                img(icon)
                    .w(px(edge))
                    .h(px(edge))
                    .object_fit(ObjectFit::Contain),
            )
        })
        .when(missing, |element| element.child(fallback))
}

fn power_menu_item(
    id: &'static str,
    label: &'static str,
    tokens: StartVisualTokens,
    action: PowerAction,
    value: StartPowerAction,
    cx: &mut Context<StartView>,
) -> impl IntoElement {
    div()
        .id(id)
        .role(gpui::Role::MenuItem)
        .aria_label(label)
        .tab_index(0)
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(tokens.hover)))
        .active(move |style| style.bg(rgb(tokens.pressed)))
        .focus_visible(move |style| style.border_2().border_color(rgb(tokens.focus)))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.model.power_open = false;
            action(value);
            cx.notify();
        }))
        .child(label)
}

fn settings_command(uri: &str, label: &str) -> CommandDescriptor {
    CommandDescriptor {
        id: shell_provider_protocol::CommandId(format!("settings:{uri}")),
        label: label.into(),
        enabled: true,
        risk: shell_provider_protocol::CommandRisk::Normal,
        children: Vec::new(),
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
        self.render_windows11(window, cx)
    }
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

    #[test]
    fn home_all_apps_and_power_are_bounded_sorted_and_dismissible() {
        let catalog = (0..20)
            .rev()
            .map(|index| result(&format!("app-{index:02}"), &format!("App {index:02}")))
            .collect::<Vec<_>>();
        let mut start = StartModel::default();
        start.open();
        let recent = (0..8)
            .map(|index| result(&format!("recent-{index}"), &format!("Recent {index}")))
            .collect();
        start.set_catalogs(catalog[..15].to_vec(), recent, catalog.clone());
        assert_eq!(start.home_pins().len(), 12);
        assert_eq!(start.recommendations().len(), 6);
        start.show_all_apps();
        assert_eq!(start.page, StartPage::AllApps);
        let all = start.all_apps_results();
        assert_eq!(all.len(), 20);
        assert!(all.windows(2).all(|pair| pair[0].title <= pair[1].title));
        start.toggle_power();
        assert!(start.power_open);
        assert!(start.dismiss_power());
        assert!(!start.power_open);
        start.show_home();
        assert_eq!(start.page, StartPage::Home);
    }

    #[test]
    fn query_temporarily_overrides_page_without_changing_pins() {
        let catalog = vec![result("alpha", "Alpha"), result("beta", "Beta")];
        let mut start = StartModel::default();
        start.open();
        start.set_catalogs(catalog.clone(), Vec::new(), catalog);
        start.show_all_apps();
        let pins = start.snapshot().pinned_ids;
        let query = start.commit_query("alpha");
        assert!(start.accept_batch(SearchBatch {
            generation: query.generation,
            provider: SearchProvider::Applications,
            state: SearchProviderState::Complete,
            results: vec![result("alpha", "Alpha")],
        }));
        assert_eq!(start.results().len(), 1);
        start.commit_query("");
        assert_eq!(start.page, StartPage::AllApps);
        assert_eq!(start.snapshot().pinned_ids, pins);
    }

    #[test]
    fn windows11_view_contract_has_sections_icons_and_collapsed_power() {
        let source = include_str!("start.rs");
        for required in [
            "windows11-start-surface",
            "start-pinned-grid",
            "start-recommended",
            "start-all-apps-page",
            "start-search-results",
            "start-account",
            "start-settings",
            "start-power",
            "start-power-menu",
            "render_icons_for",
            "StartStrings::current",
            "StartVisualTokens::new",
            ".hover(move |style|",
            ".active(move |style|",
            ".focus_visible(move |style|",
            "high-contrast",
        ] {
            assert!(
                source.contains(required),
                "missing Start contract: {required}"
            );
        }
        assert!(source.contains("return self.render_windows11(window, cx)"));
        let model = StartModel::default();
        assert!(!model.power_open);
    }

    #[test]
    fn start_strings_are_complete_localized_and_fall_back_to_english() {
        let zh = StartStrings::from_locale(Some("zh-TW"));
        assert_eq!(zh.start, "開始");
        assert_eq!(zh.search_placeholder, "搜尋應用程式、設定及檔案");
        assert_eq!(zh.all_apps, "所有應用程式");
        assert_eq!(zh.power_options, "電源選項");
        assert_eq!(zh.shut_down, "關機");
        let english = StartStrings::from_locale(Some("en-US"));
        assert_eq!(english, StartStrings::ENGLISH);
        assert_eq!(
            StartStrings::from_locale(Some("ar-SA")),
            StartStrings::ENGLISH
        );
        for value in [
            zh.start,
            zh.search_placeholder,
            zh.pinned,
            zh.all_apps,
            zh.recommended,
            zh.recent_empty,
            zh.back_to_pinned,
            zh.search_results,
            zh.account_prefix,
            zh.settings,
            zh.power,
            zh.footer_actions,
            zh.power_options,
            zh.sign_out,
            zh.restart,
            zh.shut_down,
        ] {
            assert!(!value.trim().is_empty());
            assert!(value.chars().count() <= 32);
        }
    }

    #[test]
    fn start_visual_tokens_keep_high_contrast_geometry_distinct() {
        let light = StartVisualTokens::new(false);
        let contrast = StartVisualTokens::new(true);
        assert_ne!(light.panel, light.surface);
        assert_ne!(light.hover, light.pressed);
        assert_eq!(contrast.panel, 0x000000);
        assert_eq!(contrast.border, 0xffffff);
        assert_eq!(contrast.focus, 0xffff00);
        assert_ne!(contrast.hover, contrast.pressed);
    }

    #[test]
    fn path_result_is_eligible_for_native_start_icon() {
        let executable = std::env::current_exe().unwrap();
        let mut item = result("fixture", "Fixture");
        item.activation.id.0 = format!("open:{}", executable.display());
        assert_eq!(result_path(&item), Some(executable.as_path()));
    }
}
