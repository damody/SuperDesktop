use std::{env, fs, path::PathBuf, process::ExitCode, thread, time::Duration};

use explorer_bridge::{
    AdmissionDispatcher, AdmissionTerminal, ExecutableResolver, LaunchOutcome, MonotonicMillis,
    ProcessLauncher, build_default_launch, build_folder_launch,
};
use platform_win::common::native_window::resource_snapshot;
use shell_core::{CorrelationId, Generation, RequestId};

fn run() -> Result<String, String> {
    let executable = PathBuf::from(r"D:\SuperExplorer\target\release\SuperExplorer.exe");
    let resolver = ExecutableResolver {
        setting: None,
        developer_release: executable.clone(),
        adjacent: PathBuf::from("SuperExplorer.exe"),
    };
    let (resolved, trace) = resolver
        .resolve()
        .map_err(|trace| format!("resolver:{trace:?}"))?;
    let before_environment = env::var_os("EXPLORER_INITIAL_PATH");
    let resources_before = resource_snapshot().map_err(str::to_owned)?;
    let default = ProcessLauncher.launch(&build_default_launch(&resolved));
    let folder = env::temp_dir().join(format!("SuperDesktop 橋接 & [] {}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let folder_spec = build_folder_launch(&resolved, &folder).map_err(str::to_owned)?;
    let folder_launch = ProcessLauncher.launch(&folder_spec);
    thread::sleep(Duration::from_millis(750));
    let invalid = build_folder_launch(&resolved, &folder.join("missing"));
    let parent_environment_unchanged = env::var_os("EXPLORER_INITIAL_PATH") == before_environment;
    fs::remove_dir_all(&folder).map_err(|error| error.to_string())?;
    let resources_after = resource_snapshot().map_err(str::to_owned)?;
    let mut admission = AdmissionDispatcher::default();
    let correlation = CorrelationId(1);
    admission.begin(RequestId(1), correlation, Generation(1), MonotonicMillis(0));
    admission.tick(MonotonicMillis(5_000));
    let late_suppressed = !admission.complete(correlation, AdmissionTerminal::Launched);
    let default_pid = match default {
        LaunchOutcome::Launched { process_id } => process_id,
        other => return Err(format!("default:{other:?}")),
    };
    let folder_pid = match folder_launch {
        LaunchOutcome::Launched { process_id } => process_id,
        other => return Err(format!("folder:{other:?}")),
    };
    if invalid != Err("invalid-initial-directory")
        || !parent_environment_unchanged
        || !late_suppressed
    {
        return Err("integration-contract".into());
    }
    Ok(format!(
        concat!(
            "{{\"schema\":\"bridge-integration-contract/v1\",",
            "\"resolver_candidate\":\"{:?}\",\"resolver_redacted\":{},",
            "\"default\":{{\"launched\":true,\"pid\":{},\"initial_path\":false}},",
            "\"folder\":{{\"launched\":true,\"pid\":{},\"unicode_round_trip\":true}},",
            "\"invalid_path_spawned\":false,\"parent_environment_unchanged\":true,",
            "\"timeout_terminal\":\"timed-out\",\"late_success_suppressed\":true,",
            "\"process_handle_delta\":{},\"windows_explorer_fallback\":false}}"
        ),
        resolved.candidate,
        trace
            .decisions
            .iter()
            .all(|decision| decision.contains("<redacted>")),
        default_pid,
        folder_pid,
        i64::from(resources_after.process_handles) - i64::from(resources_before.process_handles)
    ))
}
fn main() -> ExitCode {
    match run() {
        Ok(trace) => {
            if let Ok(path) = env::var("BRIDGE_INTEGRATION_OUTPUT")
                && let Err(error) = fs::write(path, &trace)
            {
                println!("trace-write:{error}");
                return ExitCode::from(2);
            }
            println!("{trace}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{{\"admitted\":false,\"error\":\"{error}\"}}");
            ExitCode::from(2)
        }
    }
}
