use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{SettingsCorrection, SettingsError, SettingsV1};

pub trait AtomicSettingsFileSystem {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>>;
    fn write_temp_synced(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn atomic_replace(&mut self, temporary: &Path, target: &Path) -> io::Result<()>;
    fn sync_parent(&mut self, parent: &Path) -> io::Result<()>;
    fn quarantine(&mut self, source: &Path, destination: &Path) -> io::Result<()>;
}

#[derive(Clone, Debug)]
pub struct FixtureRootGuard {
    canonical_root: PathBuf,
}

impl FixtureRootGuard {
    pub fn new(root: &Path) -> Result<Self, StoreError> {
        let canonical_root = root.canonicalize().map_err(StoreError::Io)?;
        if !canonical_root.is_dir() || is_filesystem_root(&canonical_root) {
            return Err(StoreError::UnsafeFixtureRoot(canonical_root));
        }
        Ok(Self { canonical_root })
    }

    pub fn validate_target(&self, target: &Path) -> Result<(), StoreError> {
        if target == self.canonical_root || target.file_name().is_none() {
            return Err(StoreError::TargetEscapesFixture(target.to_path_buf()));
        }
        let parent = target
            .parent()
            .ok_or_else(|| StoreError::TargetEscapesFixture(target.to_path_buf()))?;
        let canonical_parent = parent.canonicalize().map_err(StoreError::Io)?;
        if !canonical_parent.starts_with(&self.canonical_root) {
            return Err(StoreError::TargetEscapesFixture(target.to_path_buf()));
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.canonical_root
    }
}

fn is_filesystem_root(path: &Path) -> bool {
    let components: Vec<_> = path.components().collect();
    components.is_empty()
        || components
            .iter()
            .all(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
}

pub struct SettingsStore<F> {
    file_system: F,
    guard: FixtureRootGuard,
}

impl<F: AtomicSettingsFileSystem> SettingsStore<F> {
    pub fn new(file_system: F, guard: FixtureRootGuard) -> Self {
        Self { file_system, guard }
    }

    pub fn save(&mut self, target: &Path, settings: &SettingsV1) -> Result<SettingsV1, StoreError> {
        self.guard.validate_target(target)?;
        let mut next = settings.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(StoreError::RevisionOverflow)?;
        let encoded = next.encode();
        SettingsV1::decode(&encoded).map_err(StoreError::Validation)?;
        let temporary = temporary_path(target);
        self.file_system
            .write_temp_synced(&temporary, encoded.as_bytes())
            .map_err(StoreError::Io)?;
        let persisted = self.file_system.read(&temporary).map_err(StoreError::Io)?;
        let persisted = std::str::from_utf8(&persisted).map_err(|_| {
            StoreError::Validation(SettingsError::MalformedJson(
                "temporary file is not UTF-8".into(),
            ))
        })?;
        SettingsV1::decode(persisted).map_err(StoreError::Validation)?;
        self.file_system
            .atomic_replace(&temporary, target)
            .map_err(StoreError::Io)?;
        self.file_system
            .sync_parent(target.parent().unwrap())
            .map_err(StoreError::Io)?;
        Ok(next)
    }

    pub fn load(&mut self, target: &Path) -> Result<LoadOutcome, StoreError> {
        self.guard.validate_target(target)?;
        let bytes = match self.file_system.read(target) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LoadOutcome {
                    settings: SettingsV1::default(),
                    corrections: Vec::new(),
                    quarantine_path: None,
                });
            }
            Err(error) => return Err(StoreError::Io(error)),
        };
        let decoded = std::str::from_utf8(&bytes)
            .map_err(|_| SettingsError::MalformedJson("settings file is not UTF-8".into()))
            .and_then(SettingsV1::decode);
        match decoded {
            Ok(decoded) => Ok(LoadOutcome {
                settings: decoded.settings,
                corrections: decoded.corrections,
                quarantine_path: None,
            }),
            Err(_) => {
                let quarantine_path = quarantine_path(target);
                self.guard.validate_target(&quarantine_path)?;
                self.file_system
                    .quarantine(target, &quarantine_path)
                    .map_err(StoreError::Io)?;
                self.file_system
                    .sync_parent(target.parent().unwrap())
                    .map_err(StoreError::Io)?;
                Ok(LoadOutcome {
                    settings: SettingsV1::default(),
                    corrections: Vec::new(),
                    quarantine_path: Some(quarantine_path),
                })
            }
        }
    }

    pub fn into_file_system(self) -> F {
        self.file_system
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadOutcome {
    pub settings: SettingsV1,
    pub corrections: Vec<SettingsCorrection>,
    pub quarantine_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Validation(SettingsError),
    UnsafeFixtureRoot(PathBuf),
    TargetEscapesFixture(PathBuf),
    RevisionOverflow,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
            Self::UnsafeFixtureRoot(path) => {
                write!(formatter, "unsafe fixture root: {}", path.display())
            }
            Self::TargetEscapesFixture(path) => {
                write!(formatter, "target escapes fixture root: {}", path.display())
            }
            Self::RevisionOverflow => formatter.write_str("settings revision overflow"),
        }
    }
}

impl std::error::Error for StoreError {}

fn temporary_path(target: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    target.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

fn quarantine_path(target: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    target.with_file_name(format!("{name}.quarantine.{timestamp}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::*;
    use crate::ExecutionPreference;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Failure {
        Write,
        Replace,
        SyncParent,
        Quarantine,
    }

    #[derive(Default)]
    struct MemoryFileSystem {
        files: BTreeMap<PathBuf, Vec<u8>>,
        failure: Option<Failure>,
    }

    impl AtomicSettingsFileSystem for MemoryFileSystem {
        fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }

        fn write_temp_synced(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            if self.failure == Some(Failure::Write) {
                return Err(io::Error::other("write crash"));
            }
            self.files.insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        }

        fn atomic_replace(&mut self, temporary: &Path, target: &Path) -> io::Result<()> {
            if self.failure == Some(Failure::Replace) {
                return Err(io::Error::other("replace crash"));
            }
            let bytes = self
                .files
                .remove(temporary)
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
            self.files.insert(target.to_path_buf(), bytes);
            Ok(())
        }

        fn sync_parent(&mut self, _parent: &Path) -> io::Result<()> {
            if self.failure == Some(Failure::SyncParent) {
                Err(io::Error::other("directory flush crash"))
            } else {
                Ok(())
            }
        }

        fn quarantine(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
            if self.failure == Some(Failure::Quarantine) {
                return Err(io::Error::other("quarantine crash"));
            }
            let bytes = self
                .files
                .remove(source)
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
            self.files.insert(destination.to_path_buf(), bytes);
            Ok(())
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "superdesktop-settings-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn setup(
        failure: Option<Failure>,
    ) -> (SettingsStore<MemoryFileSystem>, PathBuf, TempDirectory) {
        let directory = TempDirectory::new();
        let root = directory.0.clone();
        let guard = FixtureRootGuard::new(&root).unwrap();
        (
            SettingsStore::new(
                MemoryFileSystem {
                    files: BTreeMap::new(),
                    failure,
                },
                guard,
            ),
            root.join("settings.json"),
            directory,
        )
    }

    #[test]
    fn save_increments_revision_and_round_trips_complete_document() {
        let (mut store, target, _directory) = setup(None);
        let mut settings = SettingsV1::default();
        settings.execution_preference = ExecutionPreference::Shell;
        let saved = store.save(&target, &settings).unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(store.load(&target).unwrap().settings, saved);
    }

    #[test]
    fn every_pre_replace_crash_preserves_old_complete_file() {
        for failure in [Failure::Write, Failure::Replace] {
            let (mut store, target, _directory) = setup(None);
            let old = store.save(&target, &SettingsV1::default()).unwrap();
            store.file_system.failure = Some(failure);
            assert!(store.save(&target, &old).is_err());
            store.file_system.failure = None;
            assert_eq!(store.load(&target).unwrap().settings, old);
        }
    }

    #[test]
    fn post_replace_directory_flush_failure_leaves_complete_new_file() {
        let (mut store, target, _directory) = setup(None);
        let old = store.save(&target, &SettingsV1::default()).unwrap();
        store.file_system.failure = Some(Failure::SyncParent);
        assert!(store.save(&target, &old).is_err());
        store.file_system.failure = None;
        assert_eq!(store.load(&target).unwrap().settings.revision, 2);
    }

    #[test]
    fn malformed_and_future_files_are_uniquely_quarantined() {
        for bytes in [
            b"{partial".as_slice(),
            br#"{"schema_version":999}"#.as_slice(),
        ] {
            let (mut store, target, _directory) = setup(None);
            store
                .file_system
                .files
                .insert(target.clone(), bytes.to_vec());
            let loaded = store.load(&target).unwrap();
            assert_eq!(loaded.settings, SettingsV1::default());
            let quarantine = loaded.quarantine_path.unwrap();
            assert!(store.file_system.files.contains_key(&quarantine));
            assert!(!store.file_system.files.contains_key(&target));
        }
    }

    #[test]
    fn fixture_guard_rejects_parent_escape_and_root_target() {
        let directory = TempDirectory::new();
        let guard = FixtureRootGuard::new(&directory.0).unwrap();
        assert!(matches!(
            guard.validate_target(&directory.0),
            Err(StoreError::TargetEscapesFixture(_))
        ));
        let outside = directory.0.parent().unwrap().join("escaped-settings.json");
        assert!(matches!(
            guard.validate_target(&outside),
            Err(StoreError::TargetEscapesFixture(_))
        ));
    }

    #[test]
    fn fixture_guard_rejects_reparse_escape_when_supported() {
        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        let link = directory.0.join("link");
        if std::os::windows::fs::symlink_dir(&outside.0, &link).is_ok() {
            let guard = FixtureRootGuard::new(&directory.0).unwrap();
            let target = link.join("settings.json");
            assert!(matches!(
                guard.validate_target(&target),
                Err(StoreError::TargetEscapesFixture(_))
            ));
        }
    }
}
