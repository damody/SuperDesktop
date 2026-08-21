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
    let mut shell_active = request.shell;
    if shell_active && let Err(error) = ensure_current_user_shell_registration() {
        eprintln!("SuperDesktop error [shell-registration]: {error}");
        shell_active = false;
    }
    if shell_active && let Err(error) = arm_recovery_guardian() {
        eprintln!("SuperDesktop error [guardian-arm]: {error}");
        if let Err(restore_error) = restore_default_explorer_registration() {
            eprintln!("SuperDesktop error [shell-registration-rollback]: {restore_error}");
        }
        shell_active = false;
    }
    if shell_active && let Err(error) = close_explorer_with_uac() {
        eprintln!("SuperDesktop error [explorer-uac]: {error}");
        if let Err(restore_error) = restore_default_explorer_registration() {
            eprintln!("SuperDesktop error [shell-registration-rollback]: {restore_error}");
        }
        shell_active = false;
    }
    let effective_request = ExecutionRequest {
        shell: shell_active,
    };
    let mut root = CompositionRoot::new(effective_request);
    if !shell_active {
        root.start()?;
    }
    surface_runtime::run(shell_active, duration)?;
    if let Some(owner) = owner {
        owner.release()?;
    }
    Ok(())
}

/// Runs the real owned-shell surfaces and hotkey hook for a bounded headful
/// verification without mutating shell registration, arming recovery, or
/// launching the elevated Explorer closer.
pub fn run_owned_hotkey_verification_for(
    duration: std::time::Duration,
) -> Result<(), &'static str> {
    let request = ExecutionRequest { shell: true };
    let owner = admit_shell(&request)?.ok_or("verification-owner-missing")?;
    surface_runtime::run(true, Some(duration))?;
    owner.release()?;
    Ok(())
}

fn close_explorer_with_uac() -> Result<(), &'static str> {
    let executable = std::env::current_exe().map_err(|_| "app-current-executable")?;
    let helper = executable
        .parent()
        .ok_or("app-no-binary-directory")?
        .join("shell-installer.exe");
    let exit_code = platform_win::common::elevation::run_elevated_helper(
        &helper,
        "close-explorer",
        std::time::Duration::from_secs(30),
    )?;
    if exit_code != 0 {
        return Err("elevated-explorer-helper-failed");
    }
    if platform_win::common::explorer_recovery::trusted_explorer_shell_present()? {
        return Err("elevated-explorer-shutdown-not-observed");
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

fn is_default_explorer_registration(observed: Option<&str>) -> bool {
    observed.is_none_or(|value| {
        matches!(
            value.trim(),
            value if value.eq_ignore_ascii_case(shell_installer::DEFAULT_EXPLORER_SHELL)
                || value.eq_ignore_ascii_case(&format!(
                    "\"{}\"",
                    shell_installer::DEFAULT_EXPLORER_SHELL
                ))
        )
    })
}

fn owned_shell_executable(observed: &str) -> Option<&str> {
    observed
        .strip_suffix("\" --shell --explicit-opt-in")?
        .strip_prefix('"')
        .filter(|path| !path.is_empty() && !path.contains('"'))
}

fn is_exact_owned_shell_registration(observed: Option<&str>, app: &std::path::Path) -> bool {
    let Some(candidate) = observed.and_then(owned_shell_executable) else {
        return false;
    };
    platform_win::common::guardian_lease::equivalent_executable_identity(
        candidate,
        &app.to_string_lossy(),
    )
    .unwrap_or(false)
}

fn reconstructed_default_explorer_record(
    observed: Option<&str>,
    app: &std::path::Path,
    fingerprint: &str,
) -> Result<Option<shell_installer::RollbackRecord>, String> {
    if is_default_explorer_registration(observed) {
        return Ok(None);
    }
    if !is_exact_owned_shell_registration(observed, app) {
        return Err("rollback record is unavailable for an unrecognized shell".to_owned());
    }
    Ok(Some(shell_installer::RollbackRecord {
        target: shell_installer::SHELL_TARGET.to_owned(),
        prior: Some(shell_installer::DEFAULT_EXPLORER_SHELL.to_owned()),
        intended: observed.map(str::to_owned),
        plan_fingerprint: fingerprint.to_owned(),
    }))
}

fn is_default_explorer_rollback(record: &shell_installer::RollbackRecord) -> bool {
    record.target == shell_installer::SHELL_TARGET
        && record.prior.as_deref() == Some(shell_installer::DEFAULT_EXPLORER_SHELL)
}

fn rollback_record_matches_owned(
    record: &shell_installer::RollbackRecord,
    observed: Option<&str>,
) -> bool {
    record.target == shell_installer::SHELL_TARGET
        && record.intended.as_deref() == observed
        && record.prior.as_deref() != observed
}

fn ensure_current_user_shell_registration() -> Result<(), String> {
    use shell_installer::{
        FileRollbackStore, InstallerCommand, InstallerDisposition, MutationAuthority,
        RollbackStore, ShellRegistry, WindowsShellRegistry, build_enable_plan, execute_plan,
    };

    let (app, guardian, rollback) = shell_registration_paths()?;
    let mut registry = WindowsShellRegistry;
    let observed = registry
        .read_shell()
        .map_err(|error| format!("read:{error:?}"))?;
    let mut store = FileRollbackStore::new(rollback.clone());
    let existing_record = store
        .load()
        .map_err(|error| format!("rollback-read:{error:?}"))?;
    let command = if existing_record.is_some() {
        InstallerCommand::Repair
    } else {
        InstallerCommand::Enable
    };
    let plan = build_enable_plan(command, observed, &app, &guardian, &rollback)
        .map_err(|error| format!("plan:{error:?}"))?;
    if plan.observed == plan.desired {
        match existing_record {
            Some(record) if rollback_record_matches_owned(&record, plan.observed.as_deref()) => {}
            Some(_) => {
                return Err("rollback record does not match the exact owned shell".to_owned());
            }
            None => {
                let record = reconstructed_default_explorer_record(
                    plan.observed.as_deref(),
                    &app,
                    &plan.fingerprint,
                )?
                .ok_or_else(|| {
                    "owned shell registration unexpectedly resolved as Explorer".to_owned()
                })?;
                store
                    .save(&record)
                    .map_err(|error| format!("rollback-reconstruct:{error:?}"))?;
            }
        }
        return Ok(());
    }
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
        .map_err(|error| format!("rollback-read:{error:?}"))?;
    if is_default_explorer_registration(observed.as_deref()) {
        if let Some(record) = record
            && is_default_explorer_rollback(&record)
        {
            store
                .remove()
                .map_err(|error| format!("rollback-remove:{error:?}"))?;
        }
        return Ok(());
    }
    let record = match record {
        Some(record) => record,
        None => {
            let record = reconstructed_default_explorer_record(
                observed.as_deref(),
                &app,
                "reconstructed-default-explorer",
            )?
            .ok_or_else(|| "rollback reconstruction produced no record".to_owned())?;
            store
                .save(&record)
                .map_err(|error| format!("rollback-reconstruct:{error:?}"))?;
            record
        }
    };
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
    use super::{
        is_default_explorer_registration, is_default_explorer_rollback,
        is_exact_owned_shell_registration, reconstructed_default_explorer_record,
        rollback_record_matches_owned,
    };

    #[test]
    fn shell_registration_is_verified_before_guardian_and_explorer_shutdown() {
        let lib = include_str!("lib.rs");
        let runtime = include_str!("surface_runtime.rs");
        let registration = lib
            .find("ensure_current_user_shell_registration()")
            .expect("shell registration");
        let guardian = lib
            .find("if shell_active && let Err(error) = arm_recovery_guardian()")
            .expect("guardian arm");
        let uac = lib
            .find("if shell_active && let Err(error) = close_explorer_with_uac()")
            .expect("UAC close");
        assert!(registration < guardian);
        assert!(guardian < uac);
        assert!(lib.contains("run_elevated_helper("));
        assert!(lib.contains("\"close-explorer\""));
        assert!(lib.contains("elevated-explorer-shutdown-not-observed"));
        assert!(lib.contains("audit.after != plan.desired"));
        assert!(lib.contains("shell_active = false"));
        assert!(lib.contains("restore_default_explorer_registration"));
        assert!(!runtime.contains("superdesktop-explorer-suppression"));
        assert!(!runtime.contains("EXPLORER_SUPPRESSION_ENABLED"));
    }

    #[test]
    fn owned_hotkey_verification_keeps_admission_but_skips_shell_mutation() {
        let source = include_str!("lib.rs");
        let body = source
            .split("pub fn run_owned_hotkey_verification_for")
            .nth(1)
            .and_then(|tail| tail.split("fn close_explorer_with_uac").next())
            .expect("owned hotkey verification body");
        for required in [
            "admit_shell(&request)",
            "surface_runtime::run(true",
            "owner.release()",
        ] {
            assert!(
                body.contains(required),
                "missing verification safety: {required}"
            );
        }
        for forbidden in [
            "ensure_current_user_shell_registration",
            "arm_recovery_guardian",
            "close_explorer_with_uac",
        ] {
            assert!(
                !body.contains(forbidden),
                "verification mutates shell: {forbidden}"
            );
        }
    }

    #[test]
    fn exact_owned_shell_recognizer_requires_exact_arguments_and_file_identity() {
        let app = std::env::current_exe().expect("current test executable");
        let owned = format!("\"{}\" --shell --explicit-opt-in", app.display());
        assert!(is_exact_owned_shell_registration(Some(&owned), &app));
        assert!(!is_exact_owned_shell_registration(
            Some(&format!("\"{}\" --shell", app.display())),
            &app
        ));
        assert!(!is_exact_owned_shell_registration(
            Some("\"C:\\Other\\superdesktop-app.exe\" --shell --explicit-opt-in"),
            &app
        ));
    }

    #[test]
    fn missing_record_reconstructs_only_owned_shell_and_is_idempotent_for_explorer() {
        let app = std::env::current_exe().expect("current test executable");
        let owned = format!("\"{}\" --shell --explicit-opt-in", app.display());
        let record = reconstructed_default_explorer_record(Some(&owned), &app, "fixture")
            .expect("owned reconstruction")
            .expect("owned rollback record");
        assert_eq!(
            record.prior.as_deref(),
            Some(shell_installer::DEFAULT_EXPLORER_SHELL)
        );
        assert_eq!(record.intended.as_deref(), Some(owned.as_str()));
        assert!(is_default_explorer_rollback(&record));
        assert!(rollback_record_matches_owned(&record, Some(&owned)));
        let mut unrelated = record.clone();
        unrelated.target = "HKCU\\ThirdParty\\Shell".to_owned();
        assert!(!is_default_explorer_rollback(&unrelated));
        assert!(!rollback_record_matches_owned(&unrelated, Some(&owned)));
        assert_eq!(
            reconstructed_default_explorer_record(
                Some(shell_installer::DEFAULT_EXPLORER_SHELL),
                &app,
                "fixture"
            ),
            Ok(None)
        );
        let quoted_uppercase_default = format!(
            "\"{}\"",
            shell_installer::DEFAULT_EXPLORER_SHELL.to_ascii_uppercase()
        );
        assert!(is_default_explorer_registration(Some(
            &quoted_uppercase_default
        )));
        assert!(is_default_explorer_registration(None));
        assert!(
            reconstructed_default_explorer_record(Some("third-party-shell.exe"), &app, "fixture")
                .is_err()
        );
    }
}
