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

pub fn run_guardian(invocation: GuardianInvocation) -> Result<(), &'static str> {
    let app = std::env::current_exe()
        .map_err(|_| "guardian-current-exe")?
        .parent()
        .ok_or("guardian-no-parent-directory")?
        .join("superdesktop-app.exe");
    platform_win::common::guardian_lease::child_accept_and_wait_expected(
        invocation.lease_handle,
        invocation.channel_handle,
        &invocation.terminal_path,
        invocation.deadline_ms,
        &app.to_string_lossy(),
    )
    .map_err(|_| "guardian-lease-validation")?;
    platform_win::common::explorer_recovery::recover_explorer_shell()
        .map(|_| ())
        .map_err(|_| "guardian-shell-recovery")
}

pub const APP_USER_MODEL_ID: &str = identity::APP_USER_MODEL_ID;
pub const ORIGINAL_FILENAME: &str = identity::ORIGINAL_FILENAME;
