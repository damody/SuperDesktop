use platform_win::common::monitor_dpi_start::{StartHostProbe, invoke_start_host_controlled};
use shell_provider_protocol::{
    CommandDescriptor, SearchBatch, SearchProvider, SearchProviderState, SearchQuery, SearchResult,
    rank_search_results,
};
use std::collections::{BTreeMap, BTreeSet};

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
            return self
                .pinned
                .iter()
                .chain(&self.recent)
                .chain(&self.all_apps)
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

    pub fn snapshot(&self) -> StartSnapshot {
        StartSnapshot {
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
        start.close();
        assert!(!start.open);
    }
}
