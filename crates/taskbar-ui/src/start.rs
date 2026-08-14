use platform_win::common::monitor_dpi_start::{StartHostProbe, invoke_start_host_controlled};

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
        StartEffect::Unavailable(StartFailure::ShellModeDeferred)
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
        assert_eq!(
            StartControl::shell_mode_fixture(),
            StartEffect::Unavailable(StartFailure::ShellModeDeferred)
        )
    }
}
