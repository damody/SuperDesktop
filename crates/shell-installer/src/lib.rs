//! Transactional, explicit-opt-in installer contracts for the per-user shell.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

mod windows_registry;
pub use windows_registry::WindowsShellRegistry;

pub const SHELL_TARGET: &str = r"HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerCommand {
    Install,
    Enable,
    Disable,
    Repair,
    Uninstall,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MutationAuthority {
    pub apply: bool,
    pub explicit_opt_in: bool,
    pub confirmed_fingerprint: Option<String>,
}

impl MutationAuthority {
    pub fn authorizes(&self, fingerprint: &str) -> bool {
        self.apply
            && self.explicit_opt_in
            && self.confirmed_fingerprint.as_deref() == Some(fingerprint)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallerPlan {
    pub command: InstallerCommand,
    pub target: String,
    pub observed: Option<String>,
    pub desired: Option<String>,
    pub app_path: PathBuf,
    pub guardian_path: PathBuf,
    pub rollback_record_path: PathBuf,
    pub app_binary_fingerprint: Option<String>,
    pub guardian_binary_fingerprint: Option<String>,
    pub preflight: Option<EnablePreflight>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnablePreflight {
    pub session_name: String,
    pub session_supported: bool,
    pub policy_allows_per_user_shell: bool,
    pub guardian_recovery_admitted: bool,
}

impl EnablePreflight {
    pub fn current() -> Self {
        let session_name = std::env::var("SESSIONNAME").unwrap_or_else(|_| "Console".into());
        let session_supported =
            !session_name.eq_ignore_ascii_case("services") && !session_name.trim().is_empty();
        Self {
            session_name,
            session_supported,
            policy_allows_per_user_shell: std::env::var_os("SUPERDESKTOP_DISABLE_SHELL_INSTALL")
                .is_none(),
            guardian_recovery_admitted: true,
        }
    }

    fn admitted(&self) -> bool {
        self.session_supported
            && self.policy_allows_per_user_shell
            && self.guardian_recovery_admitted
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RollbackRecord {
    pub target: String,
    pub prior: Option<String>,
    pub intended: Option<String>,
    pub plan_fingerprint: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerDisposition {
    DryRun,
    Applied,
    RolledBack,
    Rejected,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallerAudit {
    pub timestamp_unix_ms: u128,
    pub command: InstallerCommand,
    pub target: String,
    pub affected_targets: Vec<String>,
    pub fingerprint: String,
    pub before: Option<String>,
    pub desired: Option<String>,
    pub after: Option<String>,
    pub disposition: InstallerDisposition,
    pub message: String,
}

#[derive(Debug)]
pub enum InstallerError {
    InvalidBinary(&'static str),
    StateDrift,
    Unauthorized,
    Registry(String),
    RollbackStore(String),
    VerificationFailed,
    RollbackFailed,
    PreflightRejected(String),
}

pub trait ShellRegistry {
    fn read_shell(&mut self) -> Result<Option<String>, InstallerError>;
    fn write_shell(&mut self, value: &str) -> Result<(), InstallerError>;
    fn delete_shell(&mut self) -> Result<(), InstallerError>;
}

pub trait RollbackStore {
    fn save(&mut self, record: &RollbackRecord) -> Result<(), InstallerError>;
    fn load(&mut self) -> Result<Option<RollbackRecord>, InstallerError>;
    fn remove(&mut self) -> Result<(), InstallerError>;
}

pub fn build_enable_plan(
    command: InstallerCommand,
    observed: Option<String>,
    app_path: &Path,
    guardian_path: &Path,
    rollback_record_path: &Path,
) -> Result<InstallerPlan, InstallerError> {
    build_enable_plan_with_preflight(
        command,
        observed,
        app_path,
        guardian_path,
        rollback_record_path,
        EnablePreflight::current(),
    )
}

pub fn build_enable_plan_with_preflight(
    command: InstallerCommand,
    observed: Option<String>,
    app_path: &Path,
    guardian_path: &Path,
    rollback_record_path: &Path,
    preflight: EnablePreflight,
) -> Result<InstallerPlan, InstallerError> {
    let app_path = admitted_binary(app_path, "app")?;
    let guardian_path = admitted_binary(guardian_path, "guardian")?;
    let rollback_record_path = validate_rollback_record_path(rollback_record_path)?;
    if app_path.parent() != guardian_path.parent() {
        return Err(InstallerError::PreflightRejected(
            "app and guardian must share an installation directory".into(),
        ));
    }
    if !preflight.admitted() {
        return Err(InstallerError::PreflightRejected(
            "session, policy, or guardian recovery admission rejected".into(),
        ));
    }
    let desired = Some(format!(
        "\"{}\" --shell --explicit-opt-in",
        app_path.display()
    ));
    let app_binary_fingerprint = file_fingerprint(&app_path, "app")?;
    let guardian_binary_fingerprint = file_fingerprint(&guardian_path, "guardian")?;
    Ok(finish_plan_with_preflight(
        command,
        observed,
        desired,
        app_path,
        guardian_path,
        rollback_record_path,
        Some((
            app_binary_fingerprint,
            guardian_binary_fingerprint,
            preflight,
        )),
    ))
}

pub fn build_restore_plan(
    command: InstallerCommand,
    observed: Option<String>,
    record: &RollbackRecord,
    app_path: PathBuf,
    guardian_path: PathBuf,
    rollback_record_path: PathBuf,
) -> Result<InstallerPlan, InstallerError> {
    let rollback_record_path = validate_rollback_record_path(&rollback_record_path)?;
    Ok(finish_plan_with_preflight(
        command,
        observed,
        record.prior.clone(),
        app_path,
        guardian_path,
        rollback_record_path,
        None,
    ))
}

#[cfg(test)]
fn finish_plan(
    command: InstallerCommand,
    observed: Option<String>,
    desired: Option<String>,
    app_path: PathBuf,
    guardian_path: PathBuf,
) -> InstallerPlan {
    finish_plan_with_preflight(
        command,
        observed,
        desired,
        app_path,
        guardian_path,
        PathBuf::from(r"C:\SuperDesktop\installer-rollback.json"),
        None,
    )
}

fn finish_plan_with_preflight(
    command: InstallerCommand,
    observed: Option<String>,
    desired: Option<String>,
    app_path: PathBuf,
    guardian_path: PathBuf,
    rollback_record_path: PathBuf,
    enable_admission: Option<(String, String, EnablePreflight)>,
) -> InstallerPlan {
    let (app_binary_fingerprint, guardian_binary_fingerprint, preflight) = enable_admission
        .map(|(app, guardian, preflight)| (Some(app), Some(guardian), Some(preflight)))
        .unwrap_or((None, None, None));
    let material = format!(
        "{command:?}|{SHELL_TARGET}|{observed:?}|{desired:?}|{}|{}|{}|{app_binary_fingerprint:?}|{guardian_binary_fingerprint:?}|{preflight:?}",
        app_path.display(),
        guardian_path.display(),
        rollback_record_path.display()
    );
    InstallerPlan {
        command,
        target: SHELL_TARGET.into(),
        observed,
        desired,
        app_path,
        guardian_path,
        rollback_record_path,
        app_binary_fingerprint,
        guardian_binary_fingerprint,
        preflight,
        fingerprint: stable_fingerprint(material.as_bytes()),
    }
}

pub fn validate_rollback_record_path(path: &Path) -> Result<PathBuf, InstallerError> {
    use std::path::Component;

    if !path.is_absolute()
        || path.file_name().is_none()
        || path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err(InstallerError::PreflightRejected(
            "rollback record path must be an absolute, lexically normalized file path".into(),
        ));
    }
    Ok(path.to_path_buf())
}

fn file_fingerprint(path: &Path, field: &'static str) -> Result<String, InstallerError> {
    let mut file = fs::File::open(path).map_err(|_| InstallerError::InvalidBinary(field))?;
    let mut hash = 0xcbf29ce484222325u64;
    let mut length = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| InstallerError::InvalidBinary(field))?;
        if count == 0 {
            break;
        }
        length = length
            .checked_add(count as u64)
            .ok_or(InstallerError::InvalidBinary(field))?;
        for byte in &buffer[..count] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("fnv1a64:{hash:016x}:len:{length}"))
}

/// Revalidates binary content and Windows product identity immediately before
/// an authorized enable/install/repair mutation. Restore operations never
/// depend on the continued availability of product binaries.
pub fn validate_mutation_binaries(plan: &InstallerPlan) -> Result<(), InstallerError> {
    if matches!(
        plan.command,
        InstallerCommand::Disable | InstallerCommand::Uninstall
    ) {
        return Ok(());
    }
    let app_path = admitted_binary(&plan.app_path, "app")?;
    let guardian_path = admitted_binary(&plan.guardian_path, "guardian")?;
    if app_path != plan.app_path
        || guardian_path != plan.guardian_path
        || plan.app_binary_fingerprint.as_deref()
            != Some(file_fingerprint(&app_path, "app")?.as_str())
        || plan.guardian_binary_fingerprint.as_deref()
            != Some(file_fingerprint(&guardian_path, "guardian")?.as_str())
    {
        return Err(InstallerError::StateDrift);
    }
    windows_registry::verify_product_identity(&app_path, &guardian_path)
}

fn admitted_binary(path: &Path, field: &'static str) -> Result<PathBuf, InstallerError> {
    if !path.is_absolute() {
        return Err(InstallerError::InvalidBinary(field));
    }
    let submitted = fs::symlink_metadata(path).map_err(|_| InstallerError::InvalidBinary(field))?;
    if !submitted.is_file() || submitted.file_type().is_symlink() || is_reparse_point(&submitted) {
        return Err(InstallerError::InvalidBinary(field));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| InstallerError::InvalidBinary(field))?;
    let metadata =
        fs::symlink_metadata(&canonical).map_err(|_| InstallerError::InvalidBinary(field))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(InstallerError::InvalidBinary(field));
    }
    Ok(canonical)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

pub fn stable_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

pub fn execute_plan<R: ShellRegistry, S: RollbackStore>(
    registry: &mut R,
    store: &mut S,
    plan: &InstallerPlan,
    authority: &MutationAuthority,
) -> Result<InstallerAudit, InstallerError> {
    if !authority.authorizes(&plan.fingerprint) {
        return Ok(audit(
            plan,
            plan.observed.clone(),
            InstallerDisposition::DryRun,
            "mutation authority absent",
        ));
    }
    let current = registry.read_shell()?;
    if current != plan.observed {
        return Err(InstallerError::StateDrift);
    }
    let restoring = matches!(
        plan.command,
        InstallerCommand::Disable | InstallerCommand::Uninstall
    );
    if restoring {
        let record = store.load()?.ok_or_else(|| {
            InstallerError::RollbackStore("restore requires an existing rollback record".into())
        })?;
        if record.target != plan.target
            || record.prior != plan.desired
            || record.intended != plan.observed
        {
            return Err(InstallerError::StateDrift);
        }
    } else if plan.command == InstallerCommand::Repair {
        match store.load()? {
            Some(record) => {
                if record.target != plan.target || record.intended != plan.desired {
                    return Err(InstallerError::PreflightRejected(
                        "repair cannot silently replace the installed binary path; disable then enable"
                            .into(),
                    ));
                }
            }
            None => {
                store.save(&RollbackRecord {
                    target: plan.target.clone(),
                    prior: plan.observed.clone(),
                    intended: plan.desired.clone(),
                    plan_fingerprint: plan.fingerprint.clone(),
                })?;
            }
        }
    } else {
        let record = RollbackRecord {
            target: plan.target.clone(),
            prior: plan.observed.clone(),
            intended: plan.desired.clone(),
            plan_fingerprint: plan.fingerprint.clone(),
        };
        store.save(&record)?;
    }
    let write_result = apply_value(registry, plan.desired.as_deref());
    let after_result = registry.read_shell();
    let verified = write_result.is_ok()
        && after_result
            .as_ref()
            .is_ok_and(|after| *after == plan.desired);
    if !verified {
        restore_and_verify(registry, plan.observed.as_deref())?;
        return Ok(audit(
            plan,
            plan.observed.clone(),
            InstallerDisposition::RolledBack,
            if write_result.is_err() {
                "write failed; prior state restored"
            } else {
                "verification failed; prior state restored"
            },
        ));
    }
    let after = after_result.expect("verified result is successful");
    if restoring {
        store.remove()?;
    }
    Ok(audit(
        plan,
        after,
        InstallerDisposition::Applied,
        "transaction verified",
    ))
}

fn restore_and_verify(
    registry: &mut impl ShellRegistry,
    prior: Option<&str>,
) -> Result<(), InstallerError> {
    if apply_value(registry, prior).is_err() {
        return Err(InstallerError::RollbackFailed);
    }
    let restored = registry
        .read_shell()
        .map_err(|_| InstallerError::RollbackFailed)?;
    if restored.as_deref() != prior {
        return Err(InstallerError::RollbackFailed);
    }
    Ok(())
}

fn apply_value(
    registry: &mut impl ShellRegistry,
    value: Option<&str>,
) -> Result<(), InstallerError> {
    match value {
        Some(value) => registry.write_shell(value),
        None => registry.delete_shell(),
    }
}

fn audit(
    plan: &InstallerPlan,
    after: Option<String>,
    disposition: InstallerDisposition,
    message: &str,
) -> InstallerAudit {
    InstallerAudit {
        timestamp_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        command: plan.command,
        target: plan.target.clone(),
        affected_targets: vec![
            plan.target.clone(),
            format!("rollback_record:{}", plan.rollback_record_path.display()),
        ],
        fingerprint: plan.fingerprint.clone(),
        before: plan.observed.clone(),
        desired: plan.desired.clone(),
        after,
        disposition,
        message: message.into(),
    }
}

#[derive(Clone, Debug)]
pub struct FileRollbackStore {
    path: PathBuf,
}

impl FileRollbackStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl RollbackStore for FileRollbackStore {
    fn save(&mut self, record: &RollbackRecord) -> Result<(), InstallerError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(store_error)?;
        }
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|error| InstallerError::RollbackStore(error.to_string()))?;
        let temporary = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(store_error)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(store_error)?;
        drop(file);
        // Hard-link publication is atomic and refuses an existing destination,
        // preserving the first recovery record as immutable.
        if let Err(error) = fs::hard_link(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(store_error(error));
        }
        fs::remove_file(&temporary).map_err(store_error)?;
        Ok(())
    }
    fn load(&mut self) -> Result<Option<RollbackRecord>, InstallerError> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| InstallerError::RollbackStore(error.to_string())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(store_error(error)),
        }
    }
    fn remove(&mut self) -> Result<(), InstallerError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(store_error(error)),
        }
    }
}

fn store_error(error: io::Error) -> InstallerError {
    InstallerError::RollbackStore(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryRegistry {
        value: Option<String>,
        reads: usize,
        writes: usize,
        fail_read_at: Option<usize>,
        fail_write_at: Option<usize>,
    }
    impl ShellRegistry for MemoryRegistry {
        fn read_shell(&mut self) -> Result<Option<String>, InstallerError> {
            self.reads += 1;
            if self.fail_read_at == Some(self.reads) {
                Ok(Some("drifted-after-write".into()))
            } else {
                Ok(self.value.clone())
            }
        }
        fn write_shell(&mut self, value: &str) -> Result<(), InstallerError> {
            self.writes += 1;
            if self.fail_write_at == Some(self.writes) {
                return Err(InstallerError::Registry("injected-write".into()));
            }
            self.value = Some(value.into());
            Ok(())
        }
        fn delete_shell(&mut self) -> Result<(), InstallerError> {
            self.writes += 1;
            if self.fail_write_at == Some(self.writes) {
                return Err(InstallerError::Registry("injected-delete".into()));
            }
            self.value = None;
            Ok(())
        }
    }
    #[derive(Default)]
    struct MemoryStore {
        record: Option<RollbackRecord>,
        fail_save: bool,
    }
    impl RollbackStore for MemoryStore {
        fn save(&mut self, record: &RollbackRecord) -> Result<(), InstallerError> {
            if self.fail_save {
                Err(InstallerError::RollbackStore("injected".into()))
            } else {
                self.record = Some(record.clone());
                Ok(())
            }
        }
        fn load(&mut self) -> Result<Option<RollbackRecord>, InstallerError> {
            Ok(self.record.clone())
        }
        fn remove(&mut self) -> Result<(), InstallerError> {
            self.record = None;
            Ok(())
        }
    }

    fn test_plan(before: Option<&str>, desired: Option<&str>) -> InstallerPlan {
        finish_plan(
            InstallerCommand::Enable,
            before.map(str::to_owned),
            desired.map(str::to_owned),
            "C:\\app.exe".into(),
            "C:\\guardian.exe".into(),
        )
    }
    fn authority(plan: &InstallerPlan) -> MutationAuthority {
        MutationAuthority {
            apply: true,
            explicit_opt_in: true,
            confirmed_fingerprint: Some(plan.fingerprint.clone()),
        }
    }

    #[test]
    fn dry_run_drift_write_before_mutate_and_exact_absence_restore() {
        let plan = test_plan(Some("explorer.exe"), Some("superdesktop.exe"));
        let alternate_metadata_target = finish_plan_with_preflight(
            InstallerCommand::Enable,
            Some("explorer.exe".into()),
            Some("superdesktop.exe".into()),
            "C:\\app.exe".into(),
            "C:\\guardian.exe".into(),
            PathBuf::from(r"C:\Other\installer-rollback.json"),
            None,
        );
        assert_ne!(plan.fingerprint, alternate_metadata_target.fingerprint);
        assert!(validate_rollback_record_path(Path::new("relative-rollback.json")).is_err());
        let mut registry = MemoryRegistry {
            value: Some("explorer.exe".into()),
            ..Default::default()
        };
        let mut store = MemoryStore::default();
        let audit = execute_plan(
            &mut registry,
            &mut store,
            &plan,
            &MutationAuthority {
                apply: false,
                explicit_opt_in: false,
                confirmed_fingerprint: None,
            },
        )
        .unwrap();
        assert_eq!(audit.disposition, InstallerDisposition::DryRun);
        assert_eq!(
            audit.affected_targets,
            vec![
                SHELL_TARGET.to_owned(),
                r"rollback_record:C:\SuperDesktop\installer-rollback.json".to_owned()
            ]
        );
        assert_eq!(registry.value.as_deref(), Some("explorer.exe"));
        registry.value = Some("external.exe".into());
        assert!(matches!(
            execute_plan(&mut registry, &mut store, &plan, &authority(&plan)),
            Err(InstallerError::StateDrift)
        ));
        let absent = test_plan(None, Some("superdesktop.exe"));
        registry.value = None;
        store.fail_save = true;
        assert!(execute_plan(&mut registry, &mut store, &absent, &authority(&absent)).is_err());
        assert_eq!(registry.value, None);
    }

    #[test]
    fn verification_failure_rolls_back_exact_prior_value() {
        let plan = test_plan(Some("explorer.exe"), Some("superdesktop.exe"));
        let mut registry = MemoryRegistry {
            value: Some("explorer.exe".into()),
            fail_read_at: Some(2),
            ..Default::default()
        };
        let mut store = MemoryStore::default();
        let audit = execute_plan(&mut registry, &mut store, &plan, &authority(&plan)).unwrap();
        assert_eq!(audit.disposition, InstallerDisposition::RolledBack);
        assert_eq!(registry.value.as_deref(), Some("explorer.exe"));
        assert_eq!(
            store.record.as_ref().unwrap().prior.as_deref(),
            Some("explorer.exe")
        );
    }

    #[test]
    fn rollback_failure_is_terminal_and_preserves_recovery_record() {
        let plan = test_plan(Some("explorer.exe"), Some("superdesktop.exe"));
        let mut registry = MemoryRegistry {
            value: Some("explorer.exe".into()),
            fail_read_at: Some(2),
            fail_write_at: Some(2),
            ..Default::default()
        };
        let mut store = MemoryStore::default();
        assert!(matches!(
            execute_plan(&mut registry, &mut store, &plan, &authority(&plan)),
            Err(InstallerError::RollbackFailed)
        ));
        assert!(store.record.is_some());
    }

    #[test]
    fn restore_reuses_original_record_and_removes_it_only_after_verification() {
        let original = RollbackRecord {
            target: SHELL_TARGET.into(),
            prior: None,
            intended: Some("superdesktop.exe".into()),
            plan_fingerprint: "original".into(),
        };
        let plan = build_restore_plan(
            InstallerCommand::Uninstall,
            original.intended.clone(),
            &original,
            "C:\\app.exe".into(),
            "C:\\guardian.exe".into(),
            PathBuf::from(r"C:\SuperDesktop\installer-rollback.json"),
        )
        .unwrap();
        let mut registry = MemoryRegistry {
            value: original.intended.clone(),
            ..Default::default()
        };
        let mut store = MemoryStore {
            record: Some(original),
            fail_save: false,
        };
        let audit = execute_plan(&mut registry, &mut store, &plan, &authority(&plan)).unwrap();
        assert_eq!(audit.disposition, InstallerDisposition::Applied);
        assert_eq!(registry.value, None);
        assert!(store.record.is_none());
    }

    #[test]
    fn file_rollback_store_is_immutable_and_round_trips_exact_absence() {
        let path = std::env::temp_dir().join(format!(
            "superdesktop-rollback-store-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let record = RollbackRecord {
            target: SHELL_TARGET.into(),
            prior: None,
            intended: Some("owned.exe".into()),
            plan_fingerprint: "test".into(),
        };
        let mut store = FileRollbackStore::new(path.clone());
        store.save(&record).unwrap();
        assert_eq!(store.load().unwrap(), Some(record.clone()));
        assert!(store.save(&record).is_err());
        assert_eq!(store.load().unwrap(), Some(record));
        store.remove().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn mutation_binary_drift_and_product_identity_fail_closed_but_restore_stays_reachable() {
        let directory = std::env::temp_dir().join(format!(
            "superdesktop-installer-binaries-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let app = directory.join("superdesktop-app.exe");
        let guardian = directory.join("superdesktop-guardian.exe");
        fs::write(&app, b"fixture-app").unwrap();
        fs::write(&guardian, b"fixture-guardian").unwrap();
        let plan = build_enable_plan_with_preflight(
            InstallerCommand::Enable,
            None,
            &app,
            &guardian,
            &directory.join("installer-rollback.json"),
            EnablePreflight::current(),
        )
        .unwrap();
        assert!(matches!(
            validate_mutation_binaries(&plan),
            Err(InstallerError::PreflightRejected(_))
        ));
        fs::write(&guardian, b"fixture-guardian-drifted").unwrap();
        assert!(matches!(
            validate_mutation_binaries(&plan),
            Err(InstallerError::StateDrift)
        ));

        let restore = build_restore_plan(
            InstallerCommand::Disable,
            Some("owned-shell".into()),
            &RollbackRecord {
                target: SHELL_TARGET.into(),
                prior: None,
                intended: Some("owned-shell".into()),
                plan_fingerprint: "original".into(),
            },
            directory.join("missing-app.exe"),
            directory.join("missing-guardian.exe"),
            directory.join("installer-rollback.json"),
        )
        .unwrap();
        assert!(validate_mutation_binaries(&restore).is_ok());
        fs::remove_file(app).unwrap();
        fs::remove_file(guardian).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn repair_reuses_immutable_record_and_restores_each_transaction_pre_state() {
        let original = RollbackRecord {
            target: SHELL_TARGET.into(),
            prior: Some("explorer.exe".into()),
            intended: Some("superdesktop.exe".into()),
            plan_fingerprint: "original-install".into(),
        };
        let repair = finish_plan(
            InstallerCommand::Repair,
            Some("damaged-shell.exe".into()),
            original.intended.clone(),
            "C:\\app.exe".into(),
            "C:\\guardian.exe".into(),
        );
        let mut registry = MemoryRegistry {
            value: repair.observed.clone(),
            ..Default::default()
        };
        let mut store = MemoryStore {
            record: Some(original.clone()),
            fail_save: true,
        };
        let audit = execute_plan(&mut registry, &mut store, &repair, &authority(&repair)).unwrap();
        assert_eq!(audit.disposition, InstallerDisposition::Applied);
        assert_eq!(registry.value, original.intended);
        assert_eq!(store.record, Some(original.clone()));

        let changed_path = finish_plan(
            InstallerCommand::Repair,
            original.intended.clone(),
            Some("different-superdesktop.exe".into()),
            "C:\\app.exe".into(),
            "C:\\guardian.exe".into(),
        );
        registry.value = changed_path.observed.clone();
        assert!(matches!(
            execute_plan(
                &mut registry,
                &mut store,
                &changed_path,
                &authority(&changed_path)
            ),
            Err(InstallerError::PreflightRejected(_))
        ));
        assert_eq!(registry.value, changed_path.observed);

        registry.value = repair.observed.clone();
        registry.reads = 0;
        registry.writes = 0;
        registry.fail_read_at = Some(2);
        let audit = execute_plan(&mut registry, &mut store, &repair, &authority(&repair)).unwrap();
        assert_eq!(audit.disposition, InstallerDisposition::RolledBack);
        assert_eq!(registry.value, repair.observed);
        assert_eq!(store.record, Some(original));
    }
}
