use platform_win::common::{
    admission::probe_current_session,
    explorer_recovery::{ShellRecoveryOutcome, TrustedExplorer, recover_explorer_shell},
    ffi_boundary::{CallbackFence, CallbackResult, FfiFatal},
    monitor_dpi_start::{StartHostProbe, invoke_start_host_controlled, snapshot_real_monitors},
    native_window::resource_snapshot,
    owner_lease::SessionOwnerMutex,
};
use serde_json::json;

fn run() -> Result<serde_json::Value, &'static str> {
    let explorer_count_before = powershell_explorer_count()?;
    let admission = probe_current_session()?;
    let owner = SessionOwnerMutex::acquire()?;
    owner.revalidate()?;
    let owner_identity = owner.identity().clone();
    let trusted = TrustedExplorer::resolve()?;
    let monitors = snapshot_real_monitors()?;
    let start = invoke_start_host_controlled();
    let before = resource_snapshot()?;
    let recovery = recover_explorer_shell()?;
    let explorer_count_after = powershell_explorer_count()?;
    let panic_fence = CallbackFence::default();
    let panic_code = match panic_fence.invoke(|| panic!("lifecycle panic fixture")) {
        CallbackResult::Rejected(value) => -(value as isize),
        CallbackResult::Returned(()) => 0,
    };
    let shutdown_fence = CallbackFence::default();
    shutdown_fence.begin_shutdown();
    let shutdown_code = match shutdown_fence.invoke(|| ()) {
        CallbackResult::Rejected(value) => -(value as isize),
        CallbackResult::Returned(()) => 0,
    };
    owner.release()?;
    let after = resource_snapshot()?;
    Ok(json!({
        "schema": "superdesktop-lifecycle-live-probe/v1",
        "admission": {
            "safe_mode": admission.safe_mode,
            "interactive": admission.interactive,
            "session_id": admission.process_session_id,
            "window_station": admission.window_station
        },
        "owner": {
            "pid": owner_identity.pid,
            "creation_time": owner_identity.creation_time,
            "session_id": owner_identity.session_id,
            "user_sid_bound": !owner_identity.user_sid_hex.is_empty(),
            "authentication_id": owner_identity.authentication_id,
            "executable": owner_identity.executable,
            "file_index": owner_identity.file.file_index,
            "revalidated": true,
            "released_last": true
        },
        "explorer": {
            "application": trusted.application,
            "authenticode_verified": trusted.authenticode_verified,
            "file_index": trusted.file_index,
            "recovery": match recovery {
                ShellRecoveryOutcome::ShownExisting { process_id } => json!({"kind":"shown-existing","process_id":process_id}),
                ShellRecoveryOutcome::SpawnedVerified { process_id } => json!({"kind":"spawned-verified","process_id":process_id}),
            },
            "process_count_before": explorer_count_before,
            "process_count_after": explorer_count_after
        },
        "monitors": monitors.monitors.iter().map(|monitor| json!({
            "device": monitor.device_name,
            "dpi_x": monitor.dpi_x,
            "dpi_y": monitor.dpi_y,
            "work_area": [monitor.work_area.left, monitor.work_area.top, monitor.work_area.right, monitor.work_area.bottom]
        })).collect::<Vec<_>>(),
        "start_available": matches!(start, StartHostProbe::Available { .. }),
        "ffi": {
            "panic_code": panic_code,
            "panic_fatal": format!("{:?}", panic_fence.fatal()),
            "shutdown_code": shutdown_code,
            "shutdown_fatal": format!("{:?}", shutdown_fence.fatal()),
            "expected": [-(FfiFatal::CallbackPanic as isize), -(FfiFatal::ShutdownRace as isize)]
        },
        "resources": {
            "before": {"handles":before.process_handles,"user":before.user_objects,"gdi":before.gdi_objects},
            "after": {"handles":after.process_handles,"user":after.user_objects,"gdi":after.gdi_objects}
        }
    }))
}

fn powershell_explorer_count() -> Result<u32, &'static str> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "@(Get-Process explorer -ErrorAction SilentlyContinue).Count",
        ])
        .output()
        .map_err(|_| "powershell-explorer-count")?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| "explorer-count-parse")
}

fn main() {
    match run() {
        Ok(value) => println!(
            "{}",
            serde_json::to_string(&value).expect("JSON value serializes")
        ),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
