use std::collections::BTreeMap;

use shell_core::{BridgeTerminal, CorrelationId, Generation, RequestId};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MonotonicMillis(pub u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionTerminal {
    Launched,
    ValidationFailed,
    SpawnFailed,
    Cancelled,
    TimedOut,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionDiagnostic {
    LateTerminal,
    DuplicateCorrelation,
    ShutdownRundown,
}

#[derive(Clone, Debug)]
struct Pending {
    request_id: RequestId,
    generation: Generation,
    deadline: MonotonicMillis,
    terminal: Option<AdmissionTerminal>,
}
#[derive(Clone, Debug, Default)]
pub struct AdmissionDispatcher {
    pending: BTreeMap<CorrelationId, Pending>,
    diagnostics: Vec<(CorrelationId, AdmissionDiagnostic)>,
}

impl AdmissionDispatcher {
    pub const DEADLINE_MS: u64 = 5_000;
    pub fn begin(
        &mut self,
        request_id: RequestId,
        correlation_id: CorrelationId,
        generation: Generation,
        t0: MonotonicMillis,
    ) -> bool {
        if self.pending.contains_key(&correlation_id) {
            self.diagnostics
                .push((correlation_id, AdmissionDiagnostic::DuplicateCorrelation));
            return false;
        }
        self.pending.insert(
            correlation_id,
            Pending {
                request_id,
                generation,
                deadline: MonotonicMillis(t0.0.saturating_add(Self::DEADLINE_MS)),
                terminal: None,
            },
        );
        true
    }
    pub fn complete(&mut self, correlation_id: CorrelationId, terminal: AdmissionTerminal) -> bool {
        self.set_terminal(correlation_id, terminal)
    }
    pub fn cancel(&mut self, correlation_id: CorrelationId) -> bool {
        self.set_terminal(correlation_id, AdmissionTerminal::Cancelled)
    }
    pub fn tick(&mut self, now: MonotonicMillis) -> Vec<CorrelationId> {
        let due = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.terminal.is_none() && now >= pending.deadline)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in &due {
            self.set_terminal(*id, AdmissionTerminal::TimedOut);
        }
        due
    }
    pub fn shutdown(&mut self) {
        let ids = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.terminal.is_none())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            self.set_terminal(id, AdmissionTerminal::Cancelled);
            self.diagnostics
                .push((id, AdmissionDiagnostic::ShutdownRundown));
        }
    }
    pub fn terminal(&self, id: CorrelationId) -> Option<AdmissionTerminal> {
        self.pending.get(&id).and_then(|pending| pending.terminal)
    }
    pub fn context(&self, id: CorrelationId) -> Option<(RequestId, Generation, MonotonicMillis)> {
        self.pending
            .get(&id)
            .map(|pending| (pending.request_id, pending.generation, pending.deadline))
    }
    pub fn diagnostics(&self) -> &[(CorrelationId, AdmissionDiagnostic)] {
        &self.diagnostics
    }
    fn set_terminal(&mut self, id: CorrelationId, terminal: AdmissionTerminal) -> bool {
        let Some(pending) = self.pending.get_mut(&id) else {
            return false;
        };
        if pending.terminal.is_some() {
            self.diagnostics
                .push((id, AdmissionDiagnostic::LateTerminal));
            return false;
        }
        pending.terminal = Some(terminal);
        true
    }
}
impl From<AdmissionTerminal> for BridgeTerminal {
    fn from(value: AdmissionTerminal) -> Self {
        match value {
            AdmissionTerminal::Launched => BridgeTerminal::Launched,
            AdmissionTerminal::ValidationFailed => BridgeTerminal::ResolverUnavailable,
            AdmissionTerminal::SpawnFailed => BridgeTerminal::SpawnRejected,
            AdmissionTerminal::Cancelled => BridgeTerminal::Cancelled,
            AdmissionTerminal::TimedOut => BridgeTerminal::TimedOut,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn start() -> (AdmissionDispatcher, CorrelationId) {
        let mut d = AdmissionDispatcher::default();
        let id = CorrelationId(7);
        assert!(d.begin(RequestId(3), id, Generation(2), MonotonicMillis(100)));
        (d, id)
    }
    #[test]
    fn deadline_is_five_seconds_and_timeout_beats_late_success() {
        let (mut d, id) = start();
        assert_eq!(d.context(id).unwrap().2, MonotonicMillis(5100));
        assert!(d.tick(MonotonicMillis(5099)).is_empty());
        assert_eq!(d.tick(MonotonicMillis(5100)), vec![id]);
        assert!(!d.complete(id, AdmissionTerminal::Launched));
        assert_eq!(d.terminal(id), Some(AdmissionTerminal::TimedOut))
    }
    #[test]
    fn cancel_beats_success_and_duplicate_callback_is_diagnostic() {
        let (mut d, id) = start();
        assert!(d.cancel(id));
        assert!(!d.complete(id, AdmissionTerminal::Launched));
        assert_eq!(d.terminal(id), Some(AdmissionTerminal::Cancelled));
        assert!(
            d.diagnostics()
                .iter()
                .any(|(_, kind)| *kind == AdmissionDiagnostic::LateTerminal)
        )
    }
    #[test]
    fn duplicate_correlation_and_shutdown_rundown_are_bounded() {
        let (mut d, id) = start();
        assert!(!d.begin(RequestId(4), id, Generation(2), MonotonicMillis(101)));
        d.shutdown();
        assert_eq!(d.terminal(id), Some(AdmissionTerminal::Cancelled));
        assert!(
            d.diagnostics()
                .iter()
                .any(|(_, kind)| *kind == AdmissionDiagnostic::ShutdownRundown)
        )
    }
}
