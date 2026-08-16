//! Durable Windows filesystem adapter for the versioned settings store.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use settings_store::{AtomicSettingsFileSystem, FixtureRootGuard, SettingsStore, StoreError};
use windows::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::core::PCWSTR;

#[derive(Default)]
pub struct NativeSettingsFileSystem;

impl AtomicSettingsFileSystem for NativeSettingsFileSystem {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write_temp_synced(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn atomic_replace(&mut self, temporary: &Path, target: &Path) -> io::Result<()> {
        let temporary = wide(temporary);
        let target = wide(target);
        // SAFETY: Both strings are owned, NUL-terminated UTF-16 paths and remain
        // alive for the duration of the synchronous call. The flags restrict the
        // operation to replacing the single validated settings target.
        unsafe {
            MoveFileExW(
                PCWSTR(temporary.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(io::Error::other)
    }

    fn sync_parent(&mut self, parent: &Path) -> io::Result<()> {
        // MoveFileExW with MOVEFILE_WRITE_THROUGH already waits for the move to
        // reach durable storage. Directory handles are intentionally not opened.
        parent.metadata().map(|_| ())
    }

    fn quarantine(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }
}

pub fn production_settings_store()
-> Result<(SettingsStore<NativeSettingsFileSystem>, PathBuf), StoreError> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| StoreError::Io(io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA")))?;
    let root = local_app_data.join("SuperDesktop");
    fs::create_dir_all(&root).map_err(StoreError::Io)?;
    let guard = FixtureRootGuard::new(&root)?;
    let target = guard.root().join("settings.json");
    Ok((SettingsStore::new(NativeSettingsFileSystem, guard), target))
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_paths_are_nul_terminated() {
        let encoded = wide(Path::new(r"C:\fixture\settings.json"));
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|unit| **unit == 0).count(), 1);
    }

    #[test]
    fn native_adapter_round_trips_and_replaces() {
        let root = std::env::temp_dir().join(format!(
            "superdesktop-native-settings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("settings.json");
        let mut file_system = NativeSettingsFileSystem;
        let first = root.join("first.tmp");
        file_system.write_temp_synced(&first, b"one").unwrap();
        file_system.atomic_replace(&first, &target).unwrap();
        let second = root.join("second.tmp");
        file_system.write_temp_synced(&second, b"two").unwrap();
        file_system.atomic_replace(&second, &target).unwrap();
        assert_eq!(file_system.read(&target).unwrap(), b"two");
        let _ = fs::remove_dir_all(root);
    }
}
