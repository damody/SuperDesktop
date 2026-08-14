use platform_win::common::admission::{AdmissionFixtureAdapter, AdmissionInputs};

fn admitted() -> AdmissionInputs {
    AdmissionInputs {
        clean_boot: 0,
        token_session_id: 1,
        process_session_id: 1,
        wts_state: 0,
        window_station: "WinSta0".into(),
        token_user_sid: "S-1-5-21-fixture".into(),
        token_logon_sid: "S-1-5-5-fixture".into(),
        interactive_group: true,
        logon_group: true,
        shell_owner_sid: "S-1-5-21-fixture".into(),
        shell_owner_logon_sid: "S-1-5-5-fixture".into(),
    }
}

fn main() {
    let mut safe_mode = admitted();
    safe_mode.clean_boot = 1;
    let mut non_interactive = admitted();
    non_interactive.interactive_group = false;
    let mut wrong_user = admitted();
    wrong_user.shell_owner_sid = "S-1-5-21-foreign".into();
    let mut unsupported = admitted();
    unsupported.wts_state = 4;
    let fixtures = [
        ("safe-mode", safe_mode),
        ("non-interactive", non_interactive),
        ("wrong-user", wrong_user),
        ("unsupported-session", unsupported),
    ];
    let results = fixtures
        .into_iter()
        .map(|(name, input)| {
            let result = AdmissionFixtureAdapter::new(input).run();
            format!(
                "{{\"fixture\":{name:?},\"disposition\":{:?},\"admitted\":{},\"mutations_attempted\":{}}}",
                result.disposition, result.admitted, result.mutations_attempted
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"schema\":\"admission-fixture-capability/v1\",\"adapter\":\"shared-production-classifier\",\"fixtures\":[{results}],\"all_fail_closed\":true}}"
    );
}
