use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::{
    ActiveRequest, ApplicationId, ApplicationState, BridgeTerminal, Diagnostic, DiagnosticKind,
    ExecutionMode, Generation, LifecyclePhase, RequestId, RequestKind, ShellEffect, ShellEvent,
    ShellItemId, ShellState, TerminalKind, WindowId, WindowState,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub state: ShellState,
    pub effects: Vec<ShellEffect>,
}

impl Transition {
    fn unchanged(state: &ShellState) -> Self {
        Self {
            state: state.clone(),
            effects: Vec::new(),
        }
    }
}

pub fn reduce(current: &ShellState, event: &ShellEvent) -> Transition {
    let mut transition = Transition::unchanged(current);
    match event {
        ShellEvent::Command(command) => reduce_command(&mut transition, command),
        ShellEvent::RequestStarted {
            request_id,
            generation,
        } => {
            if *generation != current.generation || current.requests.contains_key(request_id) {
                reject(
                    &mut transition,
                    DiagnosticKind::RejectedTransition,
                    Some(*request_id),
                );
            } else {
                transition.state.requests.insert(
                    *request_id,
                    ActiveRequest {
                        request_id: *request_id,
                        generation: *generation,
                        kind: RequestKind::Bridge,
                        cancelled: false,
                        terminal: None,
                    },
                );
            }
        }
        ShellEvent::RequestTerminal {
            request_id,
            correlation_id,
            generation,
            terminal,
        } => apply_terminal(
            &mut transition,
            *request_id,
            *correlation_id,
            *generation,
            *terminal,
        ),
        ShellEvent::BridgeTerminal {
            request_id,
            correlation_id,
            generation,
            terminal,
        } => apply_terminal(
            &mut transition,
            *request_id,
            *correlation_id,
            *generation,
            bridge_terminal_kind(*terminal),
        ),
        ShellEvent::DesktopItemsChanged {
            request_id,
            generation,
            items,
        } => apply_desktop_snapshot(&mut transition, *request_id, *generation, items),
        ShellEvent::WindowsChanged {
            request_id,
            generation,
            windows,
        } => apply_window_snapshot(&mut transition, *request_id, *generation, windows),
        ShellEvent::DesktopOverflow => {
            request_refresh(&mut transition, RequestKind::DesktopRefresh)
        }
        ShellEvent::WindowOverflow => request_refresh(&mut transition, RequestKind::WindowRefresh),
    }
    transition
}

fn reduce_command(transition: &mut Transition, command: &crate::ShellCommand) {
    match command {
        crate::ShellCommand::StartPreview => {
            if transition.state.lifecycle != LifecyclePhase::Stopped {
                reject(transition, DiagnosticKind::RejectedTransition, None);
                return;
            }
            transition.state.mode = ExecutionMode::Preview;
            transition.state.lifecycle = LifecyclePhase::Preview;
            transition.effects.push(ShellEffect::PreviewReady);
        }
        crate::ShellCommand::StartShell { explicit_opt_in } => {
            if !explicit_opt_in || transition.state.lifecycle != LifecyclePhase::Stopped {
                reject(transition, DiagnosticKind::RejectedTransition, None);
                return;
            }
            transition.state.mode = ExecutionMode::Shell;
            transition.state.lifecycle = LifecyclePhase::StartingShell;
            transition
                .effects
                .push(ShellEffect::ProbeShellPrerequisites);
        }
        crate::ShellCommand::Stop => {
            if transition.state.lifecycle == LifecyclePhase::Stopped {
                reject(transition, DiagnosticKind::RejectedTransition, None);
                return;
            }
            transition.state.lifecycle = LifecyclePhase::ShuttingDown;
            transition.effects.push(ShellEffect::BeginShutdown);
        }
        crate::ShellCommand::CancelRequest(request_id) => {
            let Some(request) = transition.state.requests.get_mut(request_id) else {
                reject(
                    transition,
                    DiagnosticKind::RejectedTransition,
                    Some(*request_id),
                );
                return;
            };
            if request.terminal.is_some() || request.cancelled {
                reject(
                    transition,
                    DiagnosticKind::DuplicateTerminal,
                    Some(*request_id),
                );
                return;
            }
            request.cancelled = true;
            request.terminal = Some(TerminalKind::Cancelled);
            transition
                .effects
                .push(ShellEffect::CancelWorker(*request_id));
        }
        crate::ShellCommand::LaunchBridge(request) => {
            if transition.state.requests.contains_key(&request.request_id) {
                reject(
                    transition,
                    DiagnosticKind::RejectedTransition,
                    Some(request.request_id),
                );
                return;
            }
            transition.state.requests.insert(
                request.request_id,
                ActiveRequest {
                    request_id: request.request_id,
                    generation: transition.state.generation,
                    kind: RequestKind::Bridge,
                    cancelled: false,
                    terminal: None,
                },
            );
            transition
                .effects
                .push(ShellEffect::LaunchBridge(request.clone()));
        }
    }
}

fn apply_terminal(
    transition: &mut Transition,
    request_id: RequestId,
    correlation_id: crate::CorrelationId,
    generation: Generation,
    terminal: TerminalKind,
) {
    if generation != transition.state.generation {
        reject(transition, DiagnosticKind::StaleResult, Some(request_id));
        return;
    }
    if transition.state.terminals.contains_key(&correlation_id) {
        transition.state.diagnostics.push(Diagnostic {
            kind: DiagnosticKind::DuplicateTerminal,
            request_id: Some(request_id),
            correlation_id: Some(correlation_id),
            message_key: None,
        });
        return;
    }
    let Some(request) = transition.state.requests.get_mut(&request_id) else {
        reject(
            transition,
            DiagnosticKind::RejectedTransition,
            Some(request_id),
        );
        return;
    };
    if request.cancelled || request.terminal.is_some() {
        transition.state.diagnostics.push(Diagnostic {
            kind: if request.cancelled {
                DiagnosticKind::CancelledResult
            } else {
                DiagnosticKind::DuplicateTerminal
            },
            request_id: Some(request_id),
            correlation_id: Some(correlation_id),
            message_key: None,
        });
        return;
    }
    request.terminal = Some(terminal);
    transition.state.terminals.insert(correlation_id, terminal);
}

fn bridge_terminal_kind(terminal: BridgeTerminal) -> TerminalKind {
    match terminal {
        BridgeTerminal::Launched => TerminalKind::Succeeded,
        BridgeTerminal::TimedOut => TerminalKind::TimedOut,
        BridgeTerminal::Cancelled => TerminalKind::Cancelled,
        BridgeTerminal::ResolverUnavailable
        | BridgeTerminal::SpawnRejected
        | BridgeTerminal::AdmissionFailed => TerminalKind::Failed,
    }
}

fn request_refresh(transition: &mut Transition, kind: RequestKind) {
    if transition
        .state
        .requests
        .values()
        .any(|request| request.kind == kind && request.terminal.is_none() && !request.cancelled)
    {
        transition.state.diagnostics.push(Diagnostic {
            kind: DiagnosticKind::QueueOverflow,
            request_id: None,
            correlation_id: None,
            message_key: None,
        });
        return;
    }
    transition.state.generation = transition.state.generation.next();
    let request_id = RequestId(transition.state.next_request_id);
    transition.state.next_request_id = transition.state.next_request_id.saturating_add(1);
    let generation = transition.state.generation;
    transition.state.requests.insert(
        request_id,
        ActiveRequest {
            request_id,
            generation,
            kind,
            cancelled: false,
            terminal: None,
        },
    );
    transition.state.diagnostics.push(Diagnostic {
        kind: DiagnosticKind::QueueOverflow,
        request_id: Some(request_id),
        correlation_id: None,
        message_key: None,
    });
    transition.effects.push(match kind {
        RequestKind::DesktopRefresh => ShellEffect::RequestDesktopSnapshot {
            request_id,
            generation,
        },
        RequestKind::WindowRefresh => ShellEffect::RequestWindowSnapshot {
            request_id,
            generation,
        },
        RequestKind::Bridge => unreachable!("bridge is not a reconciliation refresh"),
    });
}

fn valid_refresh(
    transition: &mut Transition,
    request_id: RequestId,
    generation: Generation,
    kind: RequestKind,
) -> bool {
    if generation != transition.state.generation {
        reject(transition, DiagnosticKind::StaleResult, Some(request_id));
        return false;
    }
    let Some(request) = transition.state.requests.get(&request_id) else {
        reject(
            transition,
            DiagnosticKind::RejectedTransition,
            Some(request_id),
        );
        return false;
    };
    if request.kind != kind || request.generation != generation || request.terminal.is_some() {
        reject(transition, DiagnosticKind::LateDelta, Some(request_id));
        return false;
    }
    true
}

fn apply_desktop_snapshot(
    transition: &mut Transition,
    request_id: RequestId,
    generation: Generation,
    items: &BTreeSet<ShellItemId>,
) {
    if !valid_refresh(
        transition,
        request_id,
        generation,
        RequestKind::DesktopRefresh,
    ) {
        return;
    }
    transition.state.desktop_items = items.clone();
    transition
        .state
        .selection
        .selected
        .retain(|id| items.contains(id));
    if transition
        .state
        .selection
        .focused
        .as_ref()
        .is_some_and(|id| !items.contains(id))
    {
        transition.state.selection.focused = None;
    }
    transition
        .state
        .requests
        .get_mut(&request_id)
        .unwrap()
        .terminal = Some(TerminalKind::Succeeded);
}

fn apply_window_snapshot(
    transition: &mut Transition,
    request_id: RequestId,
    generation: Generation,
    windows: &BTreeMap<WindowId, WindowState>,
) {
    if !valid_refresh(
        transition,
        request_id,
        generation,
        RequestKind::WindowRefresh,
    ) {
        return;
    }
    transition.state.windows = windows.clone();
    rebuild_applications(&mut transition.state);
    transition
        .state
        .requests
        .get_mut(&request_id)
        .unwrap()
        .terminal = Some(TerminalKind::Succeeded);
}

fn rebuild_applications(state: &mut ShellState) {
    let old = state.applications.clone();
    let mut next_order = old
        .values()
        .map(|application| application.order)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut grouped: BTreeMap<ApplicationId, Vec<WindowId>> = BTreeMap::new();
    for window in state.windows.values() {
        grouped
            .entry(window.application_id.clone())
            .or_default()
            .push(window.id.clone());
    }
    state.applications = grouped
        .into_iter()
        .map(|(id, mut window_ids)| {
            window_ids.sort_by_key(|window_id| {
                state
                    .windows
                    .get(window_id)
                    .map_or(u64::MAX, |window| window.order)
            });
            let previous = old.get(&id);
            let order = previous.map_or_else(
                || {
                    let assigned = next_order;
                    next_order = next_order.saturating_add(1);
                    assigned
                },
                |application| application.order,
            );
            (
                id.clone(),
                ApplicationState {
                    id,
                    window_ids,
                    pinned: previous.is_some_and(|application| application.pinned),
                    order,
                },
            )
        })
        .collect();
}

fn reject(transition: &mut Transition, kind: DiagnosticKind, request_id: Option<RequestId>) {
    transition.state.diagnostics.push(Diagnostic {
        kind,
        request_id,
        correlation_id: None,
        message_key: None,
    });
    transition.effects.push(ShellEffect::Rejected(kind));
}

pub fn stable_state_hash(state: &ShellState) -> u64 {
    let mut canonical = String::new();
    write!(
        canonical,
        "{:?}|{:?}|{:?}|{}|{}|{}|",
        state.mode,
        state.lifecycle,
        state.recovery,
        state.generation.0,
        state.settings_revision,
        state.next_request_id
    )
    .expect("writing to a string cannot fail");
    for id in state.monitors.keys() {
        write!(canonical, "m:{id}|").unwrap();
    }
    for id in &state.desktop_items {
        write!(canonical, "d:{id}|").unwrap();
    }
    for (id, window) in &state.windows {
        write!(
            canonical,
            "w:{id}:{}:{}:{}:{}|",
            window.application_id, window.order, window.active, window.minimized
        )
        .unwrap();
    }
    for (id, request) in &state.requests {
        write!(
            canonical,
            "r:{id}:{:?}:{}:{:?}:{}|",
            request.kind, request.generation.0, request.terminal, request.cancelled
        )
        .unwrap();
    }
    for (id, terminal) in &state.terminals {
        write!(canonical, "t:{id}:{terminal:?}|").unwrap();
    }
    for diagnostic in &state.diagnostics {
        write!(canonical, "x:{:?}|", diagnostic.kind).unwrap();
    }
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    canonical.bytes().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BridgeLaunchRequest, BridgeLaunchSource, CorrelationId, ShellCommand};

    fn launch_request() -> BridgeLaunchRequest {
        BridgeLaunchRequest::default_location(
            RequestId(7),
            CorrelationId(70),
            BridgeLaunchSource::DesktopFixedEntry,
        )
    }

    #[test]
    fn shell_requires_explicit_opt_in_and_rejection_preserves_authority() {
        let initial = ShellState::default();
        let rejected = reduce(
            &initial,
            &ShellEvent::Command(ShellCommand::StartShell {
                explicit_opt_in: false,
            }),
        );
        assert_eq!(rejected.state.lifecycle, initial.lifecycle);
        assert_eq!(rejected.state.mode, ExecutionMode::Preview);
        assert_eq!(
            rejected.effects,
            vec![ShellEffect::Rejected(DiagnosticKind::RejectedTransition)]
        );
    }

    #[test]
    fn stale_success_after_refresh_cannot_modify_state() {
        let first = reduce(&ShellState::default(), &ShellEvent::DesktopOverflow);
        let first_request = match first.effects[0] {
            ShellEffect::RequestDesktopSnapshot {
                request_id,
                generation,
            } => (request_id, generation),
            _ => panic!("expected desktop refresh"),
        };
        let mut ended = first.state.clone();
        ended.requests.get_mut(&first_request.0).unwrap().terminal = Some(TerminalKind::Succeeded);
        let second = reduce(&ended, &ShellEvent::DesktopOverflow);
        let before = second.state.desktop_items.clone();
        let late = reduce(
            &second.state,
            &ShellEvent::DesktopItemsChanged {
                request_id: first_request.0,
                generation: first_request.1,
                items: [ShellItemId::new("late").unwrap()].into_iter().collect(),
            },
        );
        assert_eq!(late.state.desktop_items, before);
        assert_eq!(
            late.state.diagnostics.last().unwrap().kind,
            DiagnosticKind::StaleResult
        );
    }

    #[test]
    fn cancel_wins_over_late_success() {
        let launched = reduce(
            &ShellState::default(),
            &ShellEvent::Command(ShellCommand::LaunchBridge(launch_request())),
        );
        let cancelled = reduce(
            &launched.state,
            &ShellEvent::Command(ShellCommand::CancelRequest(RequestId(7))),
        );
        let late = reduce(
            &cancelled.state,
            &ShellEvent::BridgeTerminal {
                request_id: RequestId(7),
                correlation_id: CorrelationId(70),
                generation: Generation(0),
                terminal: BridgeTerminal::Launched,
            },
        );
        assert!(late.state.terminals.is_empty());
        assert_eq!(
            late.state.diagnostics.last().unwrap().kind,
            DiagnosticKind::CancelledResult
        );
    }

    #[test]
    fn only_first_terminal_changes_state() {
        let launched = reduce(
            &ShellState::default(),
            &ShellEvent::Command(ShellCommand::LaunchBridge(launch_request())),
        );
        let first = reduce(
            &launched.state,
            &ShellEvent::BridgeTerminal {
                request_id: RequestId(7),
                correlation_id: CorrelationId(70),
                generation: Generation(0),
                terminal: BridgeTerminal::Launched,
            },
        );
        let duplicate = reduce(
            &first.state,
            &ShellEvent::BridgeTerminal {
                request_id: RequestId(7),
                correlation_id: CorrelationId(70),
                generation: Generation(0),
                terminal: BridgeTerminal::SpawnRejected,
            },
        );
        assert_eq!(
            duplicate.state.terminals[&CorrelationId(70)],
            TerminalKind::Succeeded
        );
        assert_eq!(
            duplicate.state.diagnostics.last().unwrap().kind,
            DiagnosticKind::DuplicateTerminal
        );
    }

    #[test]
    fn replay_is_deterministic_and_hash_stable() {
        let events = [
            ShellEvent::Command(ShellCommand::StartPreview),
            ShellEvent::Command(ShellCommand::LaunchBridge(launch_request())),
            ShellEvent::BridgeTerminal {
                request_id: RequestId(7),
                correlation_id: CorrelationId(70),
                generation: Generation(0),
                terminal: BridgeTerminal::Launched,
            },
        ];
        let replay = || {
            events.iter().fold(ShellState::default(), |state, event| {
                reduce(&state, event).state
            })
        };
        let left = replay();
        let right = replay();
        assert_eq!(left, right);
        assert_eq!(stable_state_hash(&left), stable_state_hash(&right));
        assert_ne!(stable_state_hash(&left), 0);
    }

    #[test]
    fn watcher_overflow_restores_selection_by_stable_identity() {
        let keep = ShellItemId::new("item:keep").unwrap();
        let removed = ShellItemId::new("item:removed").unwrap();
        let mut initial = ShellState::default();
        initial.selection.selected = [keep.clone(), removed.clone()].into_iter().collect();
        initial.selection.focused = Some(removed);
        let requested = reduce(&initial, &ShellEvent::DesktopOverflow);
        let (request_id, generation) = match requested.effects[0] {
            ShellEffect::RequestDesktopSnapshot {
                request_id,
                generation,
            } => (request_id, generation),
            _ => panic!("expected desktop refresh"),
        };
        let applied = reduce(
            &requested.state,
            &ShellEvent::DesktopItemsChanged {
                request_id,
                generation,
                items: [keep.clone(), ShellItemId::new("item:new").unwrap()]
                    .into_iter()
                    .collect(),
            },
        );
        assert_eq!(
            applied.state.selection.selected,
            [keep].into_iter().collect()
        );
        assert_eq!(applied.state.selection.focused, None);
    }

    #[test]
    fn window_overflow_preserves_existing_group_order() {
        let app = ApplicationId::new("app:a").unwrap();
        let mut initial = ShellState::default();
        initial.applications.insert(
            app.clone(),
            ApplicationState {
                id: app.clone(),
                window_ids: Vec::new(),
                pinned: true,
                order: 9,
            },
        );
        let requested = reduce(&initial, &ShellEvent::WindowOverflow);
        let (request_id, generation) = match requested.effects[0] {
            ShellEffect::RequestWindowSnapshot {
                request_id,
                generation,
            } => (request_id, generation),
            _ => panic!("expected window refresh"),
        };
        let window_id = WindowId::new("window:1").unwrap();
        let window = WindowState {
            id: window_id.clone(),
            application_id: app.clone(),
            title: "A".into(),
            order: 3,
            active: false,
            minimized: false,
        };
        let applied = reduce(
            &requested.state,
            &ShellEvent::WindowsChanged {
                request_id,
                generation,
                windows: [(window_id.clone(), window)].into_iter().collect(),
            },
        );
        assert_eq!(applied.state.applications[&app].order, 9);
        assert!(applied.state.applications[&app].pinned);
        assert_eq!(applied.state.applications[&app].window_ids, vec![window_id]);
    }

    #[test]
    fn repeated_overflow_has_only_one_active_refresh() {
        let first = reduce(&ShellState::default(), &ShellEvent::DesktopOverflow);
        let second = reduce(&first.state, &ShellEvent::DesktopOverflow);
        assert!(second.effects.is_empty());
        assert_eq!(second.state.requests.len(), 1);
        assert_eq!(second.state.generation, first.state.generation);
    }
}
