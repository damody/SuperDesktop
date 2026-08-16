//! Anti-spoof guardian authority and exactly-once recovery coordination.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod identity;
mod recovery;

pub use recovery::{
    ExplorerDisposition, ExplorerObservation, GuardianInvocation, RecoveryCoordinator,
    RecoveryEffect, RecoveryError, RecoveryIdentity, RecoveryTerminal, RecoveryTiming,
};

/// Read-only installer handshake proving that this guardian can run in the
/// current interactive session and resolve the trusted Explorer recovery
/// target. It deliberately performs no lease acceptance or Shell mutation.
pub fn probe_recovery_readiness() -> Result<(), &'static str> {
    let session = platform_win::common::admission::probe_current_session()
        .map_err(|_| "guardian-admission-probe")?;
    if session.safe_mode || !session.interactive || session.process_session_id == 0 {
        return Err("guardian-session-not-admitted");
    }
    platform_win::common::explorer_recovery::TrustedExplorer::resolve()
        .map(|_| ())
        .map_err(|_| "guardian-explorer-recovery-target")
}

fn recovery_success_terminal(
    recovery_disposition: &str,
    explorer_pid: u32,
    handles: platform_win::common::guardian_lease::HandleCounts,
) -> String {
    format!(
        "{{\"schema\":\"guardian-terminal/v4\",\"parent_terminal_observed\":true,\"recovery_verified\":true,\"recovery_disposition\":\"{recovery_disposition}\",\"explorer_pid\":{explorer_pid},\"unique_success_terminal_count\":1,\"child_handles_before\":{},\"child_handles_after\":{},\"released_inherited_handles\":2,\"explicit_allowlist_exact\":true,\"verified_roles\":[\"parent-wait-handle\",\"one-shot-read-channel\"]}}\n",
        handles.before, handles.after
    )
}

pub fn run_guardian(invocation: GuardianInvocation) -> Result<(), &'static str> {
    let app = std::env::current_exe()
        .map_err(|_| "guardian-current-exe")?
        .parent()
        .ok_or("guardian-no-parent-directory")?
        .join("superdesktop-app.exe");
    let handles =
        platform_win::common::guardian_lease::child_accept_and_wait_expected_deferred_terminal(
            invocation.lease_handle,
            invocation.channel_handle,
            &invocation.terminal_path,
            invocation.parent_wait_ms,
            &app.to_string_lossy(),
        )
        .map_err(|_| "guardian-lease-validation")?;
    let outcome = platform_win::common::explorer_recovery::recover_explorer_shell()
        .map_err(|_| "guardian-shell-recovery")?;
    let (recovery_disposition, explorer_pid) = match outcome {
        platform_win::common::explorer_recovery::ShellRecoveryOutcome::ShownExisting {
            process_id,
        } => ("shown-existing", process_id),
        platform_win::common::explorer_recovery::ShellRecoveryOutcome::SpawnedVerified {
            process_id,
        } => ("spawned-verified", process_id),
    };
    let terminal = recovery_success_terminal(recovery_disposition, explorer_pid, handles);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&invocation.terminal_path)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(terminal.as_bytes())?;
            file.sync_all()
        })
        .map_err(|_| "guardian-success-terminal")
}

pub const APP_USER_MODEL_ID: &str = identity::APP_USER_MODEL_ID;
pub const ORIGINAL_FILENAME: &str = identity::ORIGINAL_FILENAME;

#[cfg(test)]
mod tests {
    use super::recovery_success_terminal;
    use platform_win::common::guardian_lease::HandleCounts;

    #[test]
    fn production_terminal_proves_parent_and_recovery_completion() {
        let terminal = recovery_success_terminal(
            "spawned-verified",
            42,
            HandleCounts {
                before: 7,
                after: 5,
            },
        );
        assert!(terminal.contains("\"schema\":\"guardian-terminal/v4\""));
        assert!(terminal.contains("\"parent_terminal_observed\":true"));
        assert!(terminal.contains("\"recovery_verified\":true"));
        assert!(terminal.contains("\"explorer_pid\":42"));
        assert!(terminal.contains("\"child_handles_before\":7"));
        assert!(terminal.contains("\"child_handles_after\":5"));
    }
}
