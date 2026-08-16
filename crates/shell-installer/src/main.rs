use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use shell_installer::{
    FileRollbackStore, InstallerAudit, InstallerCommand, InstallerError, MutationAuthority,
    RollbackStore, ShellRegistry, WindowsShellRegistry, build_enable_plan, build_restore_plan,
    execute_plan, failed_audit, validate_mutation_binaries, validate_rollback_record_path,
};

#[derive(Debug)]
struct Arguments {
    command: InstallerCommand,
    app: PathBuf,
    guardian: PathBuf,
    rollback_record: PathBuf,
    authority: MutationAuthority,
}

fn main() -> ExitCode {
    let raw_arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let requested_operation = raw_arguments.first().cloned();
    match run(raw_arguments.into_iter()) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, error, audit)) => {
            let operation = audit
                .as_ref()
                .map(|record| format!("{:?}", record.command).to_ascii_lowercase())
                .or(requested_operation);
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "disposition": "failed",
                    "exit_code": code,
                    "error": format!("{error:?}"),
                    "timestamp_unix_ms": timestamp_unix_ms(),
                    "operation": operation,
                    "plan_fingerprint": audit.as_ref().map(|record| &record.fingerprint),
                    "before": audit.as_ref().and_then(|record| record.before.as_ref()),
                    "desired": audit.as_ref().and_then(|record| record.desired.as_ref()),
                    "after": audit.as_ref().and_then(|record| record.after.as_ref()),
                    "affected_targets": audit.as_ref().map(|record| &record.affected_targets).cloned().unwrap_or_default(),
                    "audit": audit,
                }))
                .expect("error audit serializes")
            );
            ExitCode::from(code)
        }
    }
}

fn run(arguments: impl Iterator<Item = String>) -> Result<(), CliFailure> {
    let arguments = parse(arguments).map_err(|message| {
        failure((
            2,
            InstallerError::PreflightRejected(format!(
                "{message}; usage: shell-installer <install|enable|disable|repair|uninstall> \
                 [--app PATH] [--guardian PATH] [--rollback-record PATH] \
                 [--apply --explicit-opt-in --confirm-plan FINGERPRINT]"
            )),
        ))
    })?;
    let rollback_record_path = validate_rollback_record_path(&arguments.rollback_record)
        .map_err(|error| failure(classify(error)))?;
    let mut registry = WindowsShellRegistry;
    let observed = registry
        .read_shell()
        .map_err(|error| failure(classify(error)))?;
    let mut store = FileRollbackStore::new(rollback_record_path.clone());
    let plan = match arguments.command {
        InstallerCommand::Install | InstallerCommand::Enable | InstallerCommand::Repair => {
            build_enable_plan(
                arguments.command,
                observed,
                &arguments.app,
                &arguments.guardian,
                &rollback_record_path,
            )
            .map_err(|error| failure(classify(error)))?
        }
        InstallerCommand::Disable | InstallerCommand::Uninstall => {
            let record = store
                .load()
                .map_err(|error| failure(classify(error)))?
                .ok_or_else(|| {
                    failure(classify(InstallerError::RollbackStore(
                        "no rollback record exists; refusing an inexact restore".into(),
                    )))
                })?;
            build_restore_plan(
                arguments.command,
                observed,
                &record,
                arguments.app,
                arguments.guardian,
                rollback_record_path,
            )
            .map_err(|error| failure(classify(error)))?
        }
    };
    validate_mutation_binaries(&plan).map_err(|error| failure_with_plan(classify(error), &plan))?;
    let audit = execute_plan(&mut registry, &mut store, &plan, &arguments.authority)
        .map_err(|error| failure_with_plan(classify(error), &plan))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "plan": plan, "audit": audit }))
            .expect("installer audit serializes")
    );
    Ok(())
}

type CliFailure = (u8, InstallerError, Option<Box<InstallerAudit>>);

fn failure((code, error): (u8, InstallerError)) -> CliFailure {
    (code, error, None)
}

fn failure_with_plan(
    (code, error): (u8, InstallerError),
    plan: &shell_installer::InstallerPlan,
) -> CliFailure {
    let audit = failed_audit(plan, &error);
    (code, error, Some(Box::new(audit)))
}

fn timestamp_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn classify(error: InstallerError) -> (u8, InstallerError) {
    let code = match error {
        InstallerError::InvalidBinary(_)
        | InstallerError::Unauthorized
        | InstallerError::PreflightRejected(_) => 2,
        InstallerError::StateDrift => 3,
        _ => 4,
    };
    (code, error)
}

fn parse(arguments: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut arguments = arguments.peekable();
    let command = match arguments.next().as_deref() {
        Some("install") => InstallerCommand::Install,
        Some("enable") => InstallerCommand::Enable,
        Some("disable") => InstallerCommand::Disable,
        Some("repair") => InstallerCommand::Repair,
        Some("uninstall") => InstallerCommand::Uninstall,
        Some(value) => return Err(format!("unknown command {value}")),
        None => return Err("missing command".into()),
    };
    let current_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let install_dir = current_exe
        .parent()
        .ok_or("installer has no parent directory")?;
    let mut app = install_dir.join("superdesktop-app.exe");
    let mut guardian = install_dir.join("superdesktop-guardian.exe");
    let mut rollback_record = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SuperDesktop")
        .join("installer-rollback.json");
    let mut apply = false;
    let mut explicit_opt_in = false;
    let mut confirmed_fingerprint = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--app" => app = next_path(&mut arguments, "--app")?,
            "--guardian" => guardian = next_path(&mut arguments, "--guardian")?,
            "--rollback-record" => {
                rollback_record = next_path(&mut arguments, "--rollback-record")?
            }
            "--confirm-plan" => {
                confirmed_fingerprint = Some(
                    arguments
                        .next()
                        .ok_or("--confirm-plan requires a fingerprint")?,
                )
            }
            "--apply" => apply = true,
            "--explicit-opt-in" => explicit_opt_in = true,
            value => return Err(format!("unknown argument {value}")),
        }
    }
    Ok(Arguments {
        command,
        app,
        guardian,
        rollback_record,
        authority: MutationAuthority {
            apply,
            explicit_opt_in,
            confirmed_fingerprint,
        },
    })
}

fn next_path(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<PathBuf, String> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}
