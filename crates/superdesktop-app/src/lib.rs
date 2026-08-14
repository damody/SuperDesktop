//! Product composition root and transactional Windows Shell lifecycle.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod composition;
mod identity;
mod lifecycle;

pub use composition::{CompositionRoot, RouteSource, RoutedTerminal};
pub use lifecycle::{
    Admission, EnvironmentFacts, ExecutionRequest, HealthReport, JournalEntry, LeaseIdentity,
    LeaseRegistry, LifecycleError, Mutation, Phase, ShutdownStep, TakeoverCoordinator,
};

pub fn run_product(request: ExecutionRequest) -> Result<(), &'static str> {
    if request.shell {
        let probe = platform_win::common::admission::probe_current_session()?;
        Admission::evaluate(
            &request,
            &EnvironmentFacts {
                safe_mode: probe.safe_mode,
                interactive: probe.interactive,
                session_active: probe.process_session_id != 0,
                capability_go: true,
            },
        )
        .map_err(|_| "shell-admission-rejected")?;
        let owner = platform_win::common::owner_lease::SessionOwnerMutex::acquire()?;
        owner.revalidate()?;
        platform_win::common::explorer_recovery::TrustedExplorer::resolve()?;
        let start_available = (0..3).any(|_| {
            let available = matches!(
                platform_win::common::monitor_dpi_start::invoke_start_host_controlled(),
                platform_win::common::monitor_dpi_start::StartHostProbe::Available { .. }
            );
            if !available {
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            available
        });
        if !start_available {
            return Err("start-host-unavailable");
        }
        // The long-lived GPUI runtime consumes the same typed coordinator. The
        // binary preflight intentionally performs no mutation until those
        // surfaces report ready.
        owner.release()?;
        return Ok(());
    }
    let mut root = CompositionRoot::new(request);
    root.start()
}

/// Evidence-only parent mode for the real restricted guardian topology. The
/// process exits after the guardian accepts, causing its inherited process
/// handle to signal and recovery to run.
pub fn run_guardian_parent_fixture(terminal_path: &str) -> Result<u32, &'static str> {
    let guardian = std::env::current_exe()
        .map_err(|_| "app-current-executable")?
        .parent()
        .ok_or("app-no-binary-directory")?
        .join("superdesktop-guardian.exe");
    let lease = platform_win::common::guardian_lease::spawn_restricted_child(
        &guardian.to_string_lossy(),
        terminal_path,
    )?;
    let pid = lease.child_pid;
    lease.close_owned_handles()?;
    Ok(pid)
}

pub const APP_USER_MODEL_ID: &str = identity::APP_USER_MODEL_ID;
pub const ORIGINAL_FILENAME: &str = identity::ORIGINAL_FILENAME;
