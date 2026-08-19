//! Product composition root and transactional Windows Shell lifecycle.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod composition;
mod identity;
mod lifecycle;
mod notification_client;
mod provider_client;
mod status_client;
mod surface_runtime;
mod taskbar_state_client;

pub use composition::{CompositionRoot, RouteSource, RoutedTerminal};
pub use lifecycle::{
    Admission, EnvironmentFacts, ExecutionRequest, HealthReport, JournalEntry, LeaseIdentity,
    LeaseRegistry, LifecycleError, Mutation, Phase, ShutdownStep, TakeoverCoordinator,
};

fn admit_shell(
    request: &ExecutionRequest,
) -> Result<Option<platform_win::common::owner_lease::SessionOwnerMutex>, &'static str> {
    platform_win::common::monitor_dpi_start::enable_per_monitor_v2()?;
    if request.shell {
        let probe = platform_win::common::admission::probe_current_session()?;
        Admission::evaluate(
            request,
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
        return Ok(Some(owner));
    }
    Ok(None)
}

pub fn run_preflight(request: ExecutionRequest) -> Result<(), &'static str> {
    let owner = admit_shell(&request)?;
    let mut root = CompositionRoot::new(request);
    if !request.shell {
        root.start()?;
    }
    if let Some(owner) = owner {
        owner.release()?;
    }
    Ok(())
}

pub fn run_product(request: ExecutionRequest) -> Result<(), &'static str> {
    run_product_for(request, None)
}

pub fn run_product_for(
    request: ExecutionRequest,
    duration: Option<std::time::Duration>,
) -> Result<(), &'static str> {
    let owner = admit_shell(&request)?;
    if request.shell {
        if let Err(error) = ensure_current_user_shell_registration() {
            eprintln!("SuperDesktop error [shell-registration]: {error}");
            return Err("shell-registration-failed");
        }
        arm_recovery_guardian()?;
    }
    let mut root = CompositionRoot::new(request);
    if !request.shell {
        root.start()?;
    }
    surface_runtime::run(request.shell, duration)?;
    if let Some(owner) = owner {
        owner.release()?;
    }
    Ok(())
}

fn shell_registration_paths()
-> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf), String> {
    let app = std::env::current_exe().map_err(|error| format!("current-exe:{error}"))?;
    let directory = app
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_owned())?;
    let guardian = directory.join("superdesktop-guardian.exe");
    let rollback = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is unavailable".to_owned())?
        .join("SuperDesktop")
        .join("installer-rollback.json");
    Ok((app, guardian, rollback))
}

fn ensure_current_user_shell_registration() -> Result<(), String> {
    use shell_installer::{
        FileRollbackStore, InstallerCommand, InstallerDisposition, MutationAuthority,
        ShellRegistry, WindowsShellRegistry, build_enable_plan, execute_plan,
    };

    let (app, guardian, rollback) = shell_registration_paths()?;
    let mut registry = WindowsShellRegistry;
    let observed = registry
        .read_shell()
        .map_err(|error| format!("read:{error:?}"))?;
    let plan = build_enable_plan(
        InstallerCommand::Enable,
        observed,
        &app,
        &guardian,
        &rollback,
    )
    .map_err(|error| format!("plan:{error:?}"))?;
    if plan.observed == plan.desired {
        return Ok(());
    }
    let authority = MutationAuthority {
        apply: true,
        explicit_opt_in: true,
        confirmed_fingerprint: Some(plan.fingerprint.clone()),
    };
    let mut store = FileRollbackStore::new(rollback);
    let audit = execute_plan(&mut registry, &mut store, &plan, &authority)
        .map_err(|error| format!("apply:{error:?}"))?;
    if audit.disposition != InstallerDisposition::Applied || audit.after != plan.desired {
        return Err(format!("verification:{:?}", audit.disposition));
    }
    Ok(())
}

pub(crate) fn restore_default_explorer_registration() -> Result<(), String> {
    use shell_installer::{
        FileRollbackStore, InstallerCommand, InstallerDisposition, MutationAuthority,
        RollbackStore, ShellRegistry, WindowsShellRegistry, build_restore_plan, execute_plan,
    };

    let (app, guardian, rollback) = shell_registration_paths()?;
    let mut registry = WindowsShellRegistry;
    let observed = registry
        .read_shell()
        .map_err(|error| format!("read:{error:?}"))?;
    let mut store = FileRollbackStore::new(rollback.clone());
    let record = store
        .load()
        .map_err(|error| format!("rollback-read:{error:?}"))?
        .ok_or_else(|| "rollback record is unavailable".to_owned())?;
    let plan = build_restore_plan(
        InstallerCommand::Disable,
        observed,
        &record,
        app,
        guardian,
        rollback,
    )
    .map_err(|error| format!("plan:{error:?}"))?;
    let authority = MutationAuthority {
        apply: true,
        explicit_opt_in: true,
        confirmed_fingerprint: Some(plan.fingerprint.clone()),
    };
    let audit = execute_plan(&mut registry, &mut store, &plan, &authority)
        .map_err(|error| format!("apply:{error:?}"))?;
    if audit.disposition != InstallerDisposition::Applied || audit.after != plan.desired {
        return Err(format!("verification:{:?}", audit.disposition));
    }
    Ok(())
}

fn arm_recovery_guardian() -> Result<u32, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "app-current-executable")?;
    let install_directory = executable.parent().ok_or("app-no-binary-directory")?;
    let guardian = install_directory.join("superdesktop-guardian.exe");
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or("guardian-local-app-data-unavailable")?;
    if !local_app_data.is_absolute() {
        return Err("guardian-local-app-data-not-absolute");
    }
    let recovery_directory = local_app_data.join("SuperDesktop").join("guardian");
    std::fs::create_dir_all(&recovery_directory).map_err(|_| "guardian-record-directory")?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "guardian-record-clock")?
        .as_nanos();
    let terminal =
        recovery_directory.join(format!("recovery-{}-{nonce:032x}.json", std::process::id()));
    let terminal = terminal.to_str().ok_or("guardian-record-path-encoding")?;
    let lease = platform_win::common::guardian_lease::spawn_restricted_child(
        guardian.to_str().ok_or("guardian-path-encoding")?,
        terminal,
    )?;
    let child_pid = lease.child_pid;
    lease.close_owned_handles()?;
    std::fs::remove_file(format!("{terminal}.accepted"))
        .map_err(|_| "guardian-acceptance-cleanup")?;
    Ok(child_pid)
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

#[cfg(test)]
mod shell_registration_tests {
    #[test]
    fn shell_registration_is_verified_before_guardian_and_explorer_shutdown() {
        let lib = include_str!("lib.rs");
        let runtime = include_str!("surface_runtime.rs");
        let registration = lib
            .find("ensure_current_user_shell_registration()")
            .expect("shell registration");
        let guardian = lib.find("arm_recovery_guardian()?").expect("guardian arm");
        assert!(registration < guardian);
        assert!(lib.contains("audit.after != plan.desired"));
        assert!(lib.contains("restore_default_explorer_registration"));
        assert!(!runtime.contains("superdesktop-explorer-suppression"));
        assert!(!runtime.contains("EXPLORER_SUPPRESSION_ENABLED"));
    }
}
