//! Three-mode, controlled guardian lease capability runner.
//!
//! `--controller` owns evidence observation. `--lease-parent` creates the
//! restricted child and exits. `--guardian-child` accepts authority only from
//! its inherited parent handle and sealed pipe claim.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use platform_win::common::{
    guardian_lease::{
        child_accept_and_wait, current_resources, launch_uninherited_fixture,
        production_negative_fixtures, spawn_restricted_child,
    },
    native_window::ResourceSnapshot,
};

fn argument(name: &str) -> Result<String, String> {
    let mut values = std::env::args().skip(1);
    while let Some(value) = values.next() {
        if value == name {
            return values.next().ok_or_else(|| format!("missing-{name}"));
        }
    }
    Err(format!("missing-{name}"))
}
fn has_flag(name: &str) -> bool {
    std::env::args().any(|value| value == name)
}
fn resources(value: ResourceSnapshot) -> String {
    format!(
        "{{\"process_handles\":{},\"user_objects\":{},\"gdi_objects\":{}}}",
        value.process_handles, value.user_objects, value.gdi_objects
    )
}

fn guardian_child() -> Result<(), String> {
    let parent = argument("--lease-handle")?
        .parse::<isize>()
        .map_err(|_| "lease-handle-parse")?;
    let channel = argument("--channel-handle")?
        .parse::<isize>()
        .map_err(|_| "channel-handle-parse")?;
    child_accept_and_wait(parent, channel, &argument("--terminal-path")?, 10_000)
        .map(|_| ())
        .map_err(|error| format!("child-reject:{error:?}"))
}

fn lease_parent() -> Result<(), String> {
    let terminal = argument("--terminal-path")?;
    let report = argument("--result-path")?;
    let executable = match argument("--child-exe") {
        Ok(value) => value,
        Err(_) => std::env::current_exe()
            .map_err(|_| "current-exe")?
            .to_string_lossy()
            .into_owned(),
    };
    let before = current_resources().map_err(str::to_owned)?;
    let lease = spawn_restricted_child(&executable, &terminal).map_err(str::to_owned)?;
    let child_pid = lease.child_pid;
    // This non-authority report only lets the controller observe a concrete child
    // PID. The child has not accepted merely because this file exists.
    std::fs::write(
        &report,
        format!(
            "{{\"child_pid\":{},\"allowlist_count\":{},\"ready_claimed\":false}}",
            lease.child_pid, lease.explicit_allowlist_count
        ),
    )
    .map_err(|_| "parent-report-write")?;
    let closed_owned_handles = lease.close_owned_handles().map_err(str::to_owned)?;
    // Closing the parent process handle here is not the terminal signal; the
    // guardian remains bound to the underlying parent process object until this
    // lease-parent process exits below.
    let after = current_resources().map_err(str::to_owned)?;
    std::fs::write(
        &report,
        format!(
            "{{\"child_pid\":{},\"allowlist_count\":2,\"ready_claimed\":false,\"parent_handles_before\":{},\"parent_handles_after\":{},\"owned_process_and_thread_handles_closed\":{}}}",
            child_pid,
            before.process_handles,
            after.process_handles,
            closed_owned_handles
        ),
    )
    .map_err(|_| "parent-final-report-write")?;
    Ok(())
}

fn controller() -> Result<String, String> {
    let work = argument("--work-dir")?;
    let work = Path::new(&work);
    std::fs::create_dir_all(work).map_err(|_| "work-dir")?;
    let terminal = work.join("guardian-terminal.json");
    let acknowledgement = work.join("guardian-terminal.json.accepted");
    let report = work.join("guardian-parent-result.json");
    let _ = std::fs::remove_file(&terminal);
    let _ = std::fs::remove_file(&report);
    let _ = std::fs::remove_file(&acknowledgement);
    let executable = std::env::current_exe().map_err(|_| "current-exe")?;
    let case = argument("--case").unwrap_or_else(|_| "valid".into());
    {
        // Exclude one-time process-launch/runtime initialization from every
        // measured controller handle baseline.
        let warm_terminal = work.join("warmup-terminal.json");
        let warm_report = work.join("warmup-parent-result.json");
        let warm_args = vec![
            "--lease-parent".into(),
            "--terminal-path".into(),
            warm_terminal.to_string_lossy().into_owned(),
            "--result-path".into(),
            warm_report.to_string_lossy().into_owned(),
        ];
        if launch_uninherited_fixture(&executable.to_string_lossy(), &warm_args, 5_000)
            .map_err(str::to_owned)?
            != 0
        {
            return Err("warmup-parent-failed".into());
        }
        let deadline = Instant::now() + Duration::from_secs(12);
        while !warm_terminal.is_file() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        if !warm_terminal.is_file() {
            return Err("warmup-terminal-timeout".into());
        }
    }
    let before = current_resources().map_err(str::to_owned)?;
    let parent_executable = if case == "wrong-executable" {
        let copied = work.join("untrusted-lease-parent.exe");
        std::fs::copy(&executable, &copied).map_err(|_| "copy-wrong-executable-fixture")?;
        copied
    } else {
        executable.clone()
    };
    let mut parent_args = vec![
        "--lease-parent".into(),
        "--terminal-path".into(),
        terminal.to_string_lossy().into_owned(),
        "--result-path".into(),
        report.to_string_lossy().into_owned(),
    ];
    if case == "wrong-executable" {
        parent_args.extend([
            "--child-exe".into(),
            executable.to_string_lossy().into_owned(),
        ]);
    }
    let parent_exit =
        launch_uninherited_fixture(&parent_executable.to_string_lossy(), &parent_args, 5_000)
            .map_err(str::to_owned)?;
    if case == "wrong-executable" {
        let rejected = parent_exit != 0 && !acknowledgement.is_file() && !terminal.is_file();
        let after = current_resources().map_err(str::to_owned)?;
        let _ = std::fs::remove_file(&parent_executable);
        if before != after {
            return Err(format!(
                "wrong-executable-controller-resource-drift:{before:?}->{after:?}"
            ));
        }
        if !rejected {
            return Err("wrong-executable-was-not-rejected".into());
        }
        return Ok(format!(
            "{{\"schema\":\"guardian-lease-trace/v2\",\"case\":\"wrong-executable\",\"typed_reject\":\"WrongExecutable|FileIdentityMismatch\",\"terminal_written\":false,\"controller_resources_before\":{},\"controller_resources_after\":{}}}",
            resources(before),
            resources(after)
        ));
    }
    if parent_exit != 0 || !report.is_file() || !acknowledgement.is_file() {
        return Err("lease-parent-not-terminal".into());
    }
    let deadline = Instant::now() + Duration::from_secs(12);
    while !terminal.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    if !terminal.is_file() {
        return Err("child-terminal-timeout".into());
    }
    let terminal_trace = std::fs::read_to_string(&terminal).map_err(|_| "terminal-read")?;
    if !terminal_trace.contains("\"parent_terminal_observed\":true") {
        return Err("terminal-not-observed".into());
    }
    let after = current_resources().map_err(str::to_owned)?;
    if before != after {
        return Err(format!("controller-resource-drift:{before:?}->{after:?}"));
    }
    let negatives = production_negative_fixtures().map_err(str::to_owned)?;
    let negatives = negatives
        .iter()
        .map(|fixture| {
            format!(
                "{{\"case\":{:?},\"typed_reject\":{:?}}}",
                fixture.case,
                format!("{:?}", fixture.rejection)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"schema\":\"guardian-lease-trace/v3\",\"mode\":\"controlled-child-preview\",\"controller_resources_before\":{},\"controller_resources_after\":{},\"parent_exit_success\":true,\"parent_report\":{},\"child_terminal\":{},\"negative_fixtures\":[{}],\"negative_fixture_mutations_attempted\":false,\"explorer_mutations\":false,\"shell_takeover\":false}}",
        resources(before),
        resources(after),
        std::fs::read_to_string(report).map_err(|_| "report-read")?,
        terminal_trace.trim(),
        negatives
    ))
}

fn main() -> std::process::ExitCode {
    let result = if has_flag("--guardian-child") {
        guardian_child()
    } else if has_flag("--lease-parent") {
        lease_parent()
    } else if has_flag("--controller") {
        controller().map(|trace| {
            println!("{trace}");
        })
    } else {
        Err("mode-required".into())
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(3)
        }
    }
}
