//! Injectable fail-closed admission classifier shared by live and fixture probes.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionInputs {
    pub clean_boot: i32,
    pub token_session_id: u32,
    pub process_session_id: u32,
    pub wts_state: i32,
    pub window_station: String,
    pub token_user_sid: String,
    pub token_logon_sid: String,
    pub interactive_group: bool,
    pub logon_group: bool,
    pub shell_owner_sid: String,
    pub shell_owner_logon_sid: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionResult {
    pub disposition: &'static str,
    pub admitted: bool,
    pub mutations_attempted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveAdmissionProbe {
    pub safe_mode: bool,
    pub process_session_id: u32,
    pub window_station: String,
    pub interactive: bool,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetProcessWindowStation() -> isize;
    fn GetUserObjectInformationW(
        object: isize,
        index: i32,
        info: *mut u16,
        length: u32,
        needed: *mut u32,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
}

/// Read-only live preflight. Token/SID equality remains enforced by the owner
/// lease; this probe rejects Safe Mode, Session 0 and non-WinSta0 stations.
pub fn probe_current_session() -> Result<LiveAdmissionProbe, &'static str> {
    let mut session = 0;
    // SAFETY: process identity is read-only and session points to writable storage.
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) } == 0 {
        return Err("process-session");
    }
    // SAFETY: obtains a borrowed handle owned by the process.
    let station = unsafe { GetProcessWindowStation() };
    if station == 0 {
        return Err("window-station");
    }
    let mut needed = 0;
    // SAFETY: documented size query for UOI_NAME (2).
    let _ = unsafe { GetUserObjectInformationW(station, 2, std::ptr::null_mut(), 0, &mut needed) };
    if needed < 2 {
        return Err("window-station-name-size");
    }
    let mut name = vec![0u16; needed as usize / 2];
    // SAFETY: buffer has exactly the byte size returned by the size query.
    if unsafe { GetUserObjectInformationW(station, 2, name.as_mut_ptr(), needed, &mut needed) } == 0
    {
        return Err("window-station-name");
    }
    let end = name
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(name.len());
    let window_station = String::from_utf16_lossy(&name[..end]);
    // SAFETY: SM_CLEANBOOT (67) is a read-only system metric.
    let safe_mode = unsafe { GetSystemMetrics(67) } != 0;
    Ok(LiveAdmissionProbe {
        safe_mode,
        process_session_id: session,
        interactive: session != 0 && window_station.eq_ignore_ascii_case("WinSta0"),
        window_station,
    })
}

pub fn classify(input: &AdmissionInputs) -> (&'static str, bool) {
    if input.clean_boot != 0 {
        ("safe-mode", false)
    } else if input.token_session_id != input.process_session_id {
        ("token-session-mismatch", false)
    } else if matches!(
        input.token_user_sid.as_str(),
        "S-1-5-18" | "S-1-5-6" | "S-1-5-7" | "S-1-5-19" | "S-1-5-20"
    ) {
        ("service-system-or-anonymous-token", false)
    } else if !input.interactive_group || !input.logon_group {
        ("non-interactive-or-foreign-token", false)
    } else if input.token_user_sid != input.shell_owner_sid
        || input.token_logon_sid != input.shell_owner_logon_sid
    {
        ("shell-owner-token-mismatch", false)
    } else if input.wts_state != 0 {
        ("session-not-active", false)
    } else if input.window_station != "WinSta0" {
        ("non-interactive-window-station", false)
    } else {
        ("admitted", true)
    }
}

/// Fixture injection cannot reach a mutation adapter: classification is the
/// only operation exposed by this type.
pub struct AdmissionFixtureAdapter {
    inputs: AdmissionInputs,
}

impl AdmissionFixtureAdapter {
    pub fn new(inputs: AdmissionInputs) -> Self {
        Self { inputs }
    }

    pub fn run(self) -> AdmissionResult {
        let (disposition, admitted) = classify(&self.inputs);
        AdmissionResult {
            disposition,
            admitted,
            mutations_attempted: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AdmissionFixtureAdapter, AdmissionInputs, probe_current_session};

    fn admitted() -> AdmissionInputs {
        AdmissionInputs {
            clean_boot: 0,
            token_session_id: 1,
            process_session_id: 1,
            wts_state: 0,
            window_station: "WinSta0".into(),
            token_user_sid: "S-1-5-21-1".into(),
            token_logon_sid: "S-1-5-5-1-2".into(),
            interactive_group: true,
            logon_group: true,
            shell_owner_sid: "S-1-5-21-1".into(),
            shell_owner_logon_sid: "S-1-5-5-1-2".into(),
        }
    }

    #[test]
    fn negative_fixtures_are_fail_closed_and_mutation_incapable() {
        let mut safe_mode = admitted();
        safe_mode.clean_boot = 1;
        let mut wrong_user = admitted();
        wrong_user.shell_owner_sid = "S-1-5-21-foreign".into();
        let mut unsupported = admitted();
        unsupported.wts_state = 4;
        for input in [safe_mode, wrong_user, unsupported] {
            let result = AdmissionFixtureAdapter::new(input).run();
            assert!(!result.admitted);
            assert!(!result.mutations_attempted);
        }
    }

    #[test]
    fn live_probe_reports_this_interactive_test_session() {
        let probe = probe_current_session().unwrap();
        assert!(!probe.safe_mode);
        assert!(probe.interactive);
        assert_eq!(probe.window_station, "WinSta0");
    }
}
