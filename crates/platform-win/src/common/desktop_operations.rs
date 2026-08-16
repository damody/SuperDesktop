//! Admitted filesystem effects used by the desktop operation controller.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows::Win32::UI::Shell::{
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
    SHFileOperationW,
};
use windows::core::PCWSTR;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionPolicy {
    Fail,
    Replace,
    Rename,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferTerminal {
    Completed,
    Cancelled,
}

#[derive(Debug)]
pub enum FileOperationError {
    OutsideAllowedRoot,
    InvalidName,
    Collision,
    UnsupportedItem,
    RecycleFailed(i32),
    Io(io::Error),
}

impl From<io::Error> for FileOperationError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn validate_filename(name: &str) -> Result<(), FileOperationError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.ends_with(['.', ' '])
        || trimmed
            .chars()
            .any(|value| "<>:\"/\\|?*".contains(value) || value == '\0')
    {
        return Err(FileOperationError::InvalidName);
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        return Err(FileOperationError::InvalidName);
    }
    Ok(())
}

pub fn admit_existing(
    path: &Path,
    allowed_roots: &[PathBuf],
) -> Result<PathBuf, FileOperationError> {
    let canonical = path.canonicalize()?;
    let admitted = allowed_roots.iter().any(|root| {
        root.canonicalize()
            .is_ok_and(|root| canonical.starts_with(root))
    });
    if admitted {
        Ok(canonical)
    } else {
        Err(FileOperationError::OutsideAllowedRoot)
    }
}

pub fn rename_item(
    source: &Path,
    new_name: &str,
    allowed_roots: &[PathBuf],
) -> Result<PathBuf, FileOperationError> {
    validate_filename(new_name)?;
    let source = admit_existing(source, allowed_roots)?;
    let destination = source
        .parent()
        .ok_or(FileOperationError::UnsupportedItem)?
        .join(new_name);
    if destination.exists() {
        return Err(FileOperationError::Collision);
    }
    fs::rename(&source, &destination)?;
    Ok(destination)
}

pub fn recycle_item(path: &Path, allowed_roots: &[PathBuf]) -> Result<(), FileOperationError> {
    let path = admit_existing(path, allowed_roots)?;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.extend([0, 0]);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(wide.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    // SAFETY: `pFrom` is a live double-NUL-terminated UTF-16 list for the
    // synchronous call. No other pointers are provided and the admitted path
    // is confined to a configured desktop root.
    let result = unsafe { SHFileOperationW(&mut operation) };
    if result != 0 || operation.fAnyOperationsAborted.as_bool() {
        Err(FileOperationError::RecycleFailed(result))
    } else {
        Ok(())
    }
}

pub fn permanent_delete(
    path: &Path,
    allowed_roots: &[PathBuf],
    explicitly_allowed: bool,
) -> Result<(), FileOperationError> {
    if !explicitly_allowed {
        return Err(FileOperationError::UnsupportedItem);
    }
    let path = admit_existing(path, allowed_roots)?;
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn copy_file_cancellable(
    source: &Path,
    destination: &Path,
    allowed_roots: &[PathBuf],
    collision: CollisionPolicy,
    mut continue_copy: impl FnMut(u64, u64) -> bool,
) -> Result<(PathBuf, TransferTerminal), FileOperationError> {
    let source = admit_existing(source, allowed_roots)?;
    if !source.is_file() {
        return Err(FileOperationError::UnsupportedItem);
    }
    let destination_parent = destination
        .parent()
        .ok_or(FileOperationError::UnsupportedItem)?;
    let parent = admit_existing(destination_parent, allowed_roots)?;
    let requested_name = destination
        .file_name()
        .ok_or(FileOperationError::InvalidName)?;
    validate_filename(&requested_name.to_string_lossy())?;
    let destination = resolve_destination(&parent.join(requested_name), collision)?;
    let total = source.metadata()?.len();
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)?;
    let mut copied = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        if !continue_copy(copied, total) {
            drop(output);
            let _ = fs::remove_file(&destination);
            return Ok((destination, TransferTerminal::Cancelled));
        }
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        copied = copied.saturating_add(read as u64);
    }
    output.sync_all()?;
    Ok((destination, TransferTerminal::Completed))
}

pub fn move_item(
    source: &Path,
    destination: &Path,
    allowed_roots: &[PathBuf],
    collision: CollisionPolicy,
) -> Result<PathBuf, FileOperationError> {
    let source = admit_existing(source, allowed_roots)?;
    let parent = admit_existing(
        destination
            .parent()
            .ok_or(FileOperationError::UnsupportedItem)?,
        allowed_roots,
    )?;
    let name = destination
        .file_name()
        .ok_or(FileOperationError::InvalidName)?;
    validate_filename(&name.to_string_lossy())?;
    let destination = resolve_destination(&parent.join(name), collision)?;
    fs::rename(source, &destination)?;
    Ok(destination)
}

fn resolve_destination(
    path: &Path,
    collision: CollisionPolicy,
) -> Result<PathBuf, FileOperationError> {
    if !path.exists() {
        return Ok(path.to_owned());
    }
    match collision {
        CollisionPolicy::Fail => Err(FileOperationError::Collision),
        CollisionPolicy::Replace => {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
            Ok(path.to_owned())
        }
        CollisionPolicy::Rename => {
            let parent = path.parent().ok_or(FileOperationError::UnsupportedItem)?;
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("item");
            let extension = path.extension().and_then(|value| value.to_str());
            for index in 2..=10_000 {
                let name = match extension {
                    Some(extension) => format!("{stem} ({index}).{extension}"),
                    None => format!("{stem} ({index})"),
                };
                let candidate = parent.join(name);
                if !candidate.exists() {
                    return Ok(candidate);
                }
            }
            Err(FileOperationError::Collision)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "superdesktop-file-ops-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn rename_copy_collision_cancel_and_delete_are_admitted() {
        let root = fixture();
        let source = root.join("source.txt");
        fs::write(&source, vec![7u8; 200_000]).unwrap();
        let renamed = rename_item(&source, "renamed.txt", std::slice::from_ref(&root)).unwrap();
        let copy = root.join("copy.txt");
        let (_, terminal) = copy_file_cancellable(
            &renamed,
            &copy,
            std::slice::from_ref(&root),
            CollisionPolicy::Fail,
            |copied, _| copied == 0,
        )
        .unwrap();
        assert_eq!(terminal, TransferTerminal::Cancelled);
        assert!(!copy.exists());
        fs::write(&copy, b"collision").unwrap();
        assert!(matches!(
            copy_file_cancellable(
                &renamed,
                &copy,
                std::slice::from_ref(&root),
                CollisionPolicy::Fail,
                |_, _| true
            ),
            Err(FileOperationError::Collision)
        ));
        assert!(permanent_delete(&renamed, std::slice::from_ref(&root), false).is_err());
        permanent_delete(&renamed, std::slice::from_ref(&root), true).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_names_and_outside_roots_fail_closed() {
        assert!(validate_filename("CON.txt").is_err());
        assert!(validate_filename("bad?.txt").is_err());
        let root = fixture();
        assert!(admit_existing(Path::new("C:\\Windows"), std::slice::from_ref(&root)).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
