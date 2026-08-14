use desktop_ui::{ActivationController, ActivationEffect, ActivationSource};
use explorer_bridge::{AdmissionDispatcher, AdmissionTerminal, MonotonicMillis};
use settings_store::{RuntimeMode, SettingsV1};
use shell_core::{BridgeLaunchRequest, BridgeTerminal, Generation, ShellState};
use taskbar_ui::{AppBarMode, AppBarRegistry, TaskEffect, TaskInteraction, TaskSource};

use crate::{Admission, EnvironmentFacts, ExecutionRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteSource {
    DesktopPointer,
    DesktopKeyboard,
    DesktopUia,
    TaskbarPointer,
    TaskbarKeyboard,
    TaskbarUia,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutedTerminal {
    Launched,
    Failed,
    TimedOut,
    Cancelled,
}

/// The only crate that owns all product-domain adapters. Cross-domain launches
/// are represented by the shell-core `BridgeLaunchRequest` contract.
pub struct CompositionRoot {
    request: ExecutionRequest,
    pub core: ShellState,
    pub settings: SettingsV1,
    pub desktop: ActivationController,
    pub taskbar: TaskInteraction,
    pub appbars: AppBarRegistry,
    pub bridge: AdmissionDispatcher,
}

impl CompositionRoot {
    pub fn new(request: ExecutionRequest) -> Self {
        Self {
            request,
            core: ShellState::default(),
            settings: SettingsV1::default(),
            desktop: ActivationController::default(),
            taskbar: TaskInteraction::default(),
            appbars: AppBarRegistry::new(AppBarMode::Preview),
            bridge: AdmissionDispatcher::default(),
        }
    }

    pub fn start(&mut self) -> Result<(), &'static str> {
        let mode = if self.request.shell {
            RuntimeMode::Shell
        } else {
            RuntimeMode::Preview
        };
        if mode == RuntimeMode::Shell {
            return Err("shell-mode-requires-transaction-coordinator");
        }
        // Preview deliberately constructs no platform mutation adapter.
        let facts = EnvironmentFacts::supported_fixture();
        Admission::evaluate(&self.request, &facts)
            .map(|_| ())
            .map_err(|_| "preview-admission-rejected")
    }

    pub fn route_fixed(&mut self, source: RouteSource, now_ms: u64) -> BridgeLaunchRequest {
        let request = match source {
            RouteSource::DesktopPointer => self.desktop_request(ActivationSource::Pointer),
            RouteSource::DesktopKeyboard => self.desktop_request(ActivationSource::Keyboard),
            RouteSource::DesktopUia => self.desktop_request(ActivationSource::Accessibility),
            RouteSource::TaskbarPointer => self.taskbar_request(TaskSource::Pointer),
            RouteSource::TaskbarKeyboard => self.taskbar_request(TaskSource::Keyboard),
            RouteSource::TaskbarUia => self.taskbar_request(TaskSource::Accessibility),
        };
        assert!(self.bridge.begin(
            request.request_id,
            request.correlation_id,
            Generation(1),
            MonotonicMillis(now_ms),
        ));
        request
    }

    pub fn complete(&mut self, request: &BridgeLaunchRequest, terminal: RoutedTerminal) -> bool {
        let terminal = match terminal {
            RoutedTerminal::Launched => AdmissionTerminal::Launched,
            RoutedTerminal::Failed => AdmissionTerminal::SpawnFailed,
            RoutedTerminal::TimedOut => AdmissionTerminal::TimedOut,
            RoutedTerminal::Cancelled => AdmissionTerminal::Cancelled,
        };
        if !self.bridge.complete(request.correlation_id, terminal) {
            return false;
        }
        let typed: BridgeTerminal = terminal.into();
        match request.source {
            shell_core::BridgeLaunchSource::DesktopFixedEntry => self
                .desktop
                .apply_bridge_terminal(request.correlation_id, typed),
            shell_core::BridgeLaunchSource::TaskbarFixedEntry => self
                .taskbar
                .apply_fixed_terminal(request.correlation_id, typed),
            _ => true,
        }
    }

    fn desktop_request(&mut self, source: ActivationSource) -> BridgeLaunchRequest {
        let ActivationEffect::Bridge(request) = self.desktop.activate_fixed_entry(source) else {
            unreachable!()
        };
        request
    }

    fn taskbar_request(&mut self, source: TaskSource) -> BridgeLaunchRequest {
        let TaskEffect::LaunchBridge(request) = self.taskbar.activate_fixed(source) else {
            unreachable!()
        };
        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_composes_every_domain_without_shell_mutation() {
        let mut root = CompositionRoot::new(ExecutionRequest::default());
        assert_eq!(root.start(), Ok(()));
        assert!(root.appbars.bars().is_empty());
        assert_eq!(root.core, ShellState::default());
    }

    #[test]
    fn all_fixed_entry_inputs_share_typed_exactly_once_bridge_route() {
        for source in [
            RouteSource::DesktopPointer,
            RouteSource::DesktopKeyboard,
            RouteSource::DesktopUia,
            RouteSource::TaskbarPointer,
            RouteSource::TaskbarKeyboard,
            RouteSource::TaskbarUia,
        ] {
            let mut root = CompositionRoot::new(ExecutionRequest::default());
            let request = root.route_fixed(source, 10);
            assert!(root.complete(&request, RoutedTerminal::Launched));
            assert!(!root.complete(&request, RoutedTerminal::Failed));
        }
    }
}
