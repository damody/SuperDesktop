//! Owned Windows desktop namespace, association, and watcher callback adapters.

use std::ffi::c_void;
use std::fs;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
    OPEN_EXISTING,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_PublicDesktop, KF_FLAG_DEFAULT, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW, SHGetKnownFolderPath, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, GetWindowThreadProcessId, HWND_BOTTOM, IsWindow,
    SPI_GETDESKWALLPAPER, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos,
    SystemParametersInfoW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use windows::core::{GUID, PCWSTR};

use super::ffi_boundary::{CallbackFence, CallbackResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopOrigin {
    User,
    Public,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedDesktopEntry {
    pub stable_identity: String,
    pub display_name: String,
    pub canonical_path: PathBuf,
    pub origin: DesktopOrigin,
    pub folder: bool,
    pub hidden: bool,
    pub system: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopPlatformError {
    KnownFolder(String),
    Enumeration(String),
    Identity(String),
    Association(String),
    Window(String),
    Wallpaper(String),
}

pub fn current_wallpaper_path() -> Result<PathBuf, DesktopPlatformError> {
    let mut buffer = vec![0u16; 32_768];
    // SAFETY: The output buffer is writable for the declared UTF-16 capacity;
    // SPI_GETDESKWALLPAPER is an observation-only query.
    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr().cast()),
            Default::default(),
        )
    }
    .map_err(|error| DesktopPlatformError::Wallpaper(error.to_string()))?;
    let length = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length]));
    if !path.is_file() {
        return Err(DesktopPlatformError::Wallpaper(
            "wallpaper-path-unavailable".into(),
        ));
    }
    Ok(path)
}

/// Applies the native portion of the desktop-surface contract to a GPUI-owned
/// HWND. Ownership and destruction remain with GPUI.
pub fn configure_and_show_desktop_window(
    hwnd_value: isize,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<(), DesktopPlatformError> {
    let hwnd = HWND(hwnd_value as *mut c_void);
    let mut owner_pid = 0;
    // SAFETY: All operations are limited to a live HWND owned by this process.
    // The function changes styles/z-order but neither borrows nor destroys it.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(DesktopPlatformError::Window("invalid-hwnd".into()));
        }
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        if owner_pid != GetCurrentProcessId() {
            return Err(DesktopPlatformError::Window("foreign-hwnd".into()));
        }
        let existing = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let required = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW).0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, existing | required);
        let applied = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if applied & required != required {
            return Err(DesktopPlatformError::Window(
                "desktop-window-style-not-applied".into(),
            ));
        }
        SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            left,
            top,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|error| DesktopPlatformError::Window(error.to_string()))?;
    }
    Ok(())
}

pub fn enumerate_known_desktops() -> Result<Vec<OwnedDesktopEntry>, DesktopPlatformError> {
    let (user, public) = known_desktop_roots()?;
    let mut entries = enumerate_directory(&user, DesktopOrigin::User)?;
    entries.extend(enumerate_directory(&public, DesktopOrigin::Public)?);
    Ok(entries)
}

pub fn known_desktop_roots() -> Result<(PathBuf, PathBuf), DesktopPlatformError> {
    Ok((
        known_folder(&FOLDERID_Desktop)?,
        known_folder(&FOLDERID_PublicDesktop)?,
    ))
}

fn known_folder(id: &GUID) -> Result<PathBuf, DesktopPlatformError> {
    // SAFETY: The GUID is a valid static known-folder identifier; the returned
    // CoTaskMem allocation is copied to Rust-owned text and freed exactly once.
    unsafe {
        let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)
            .map_err(|error| DesktopPlatformError::KnownFolder(error.to_string()))?;
        let result = path
            .to_string()
            .map(PathBuf::from)
            .map_err(|error| DesktopPlatformError::KnownFolder(error.to_string()));
        CoTaskMemFree(Some(path.0.cast::<c_void>()));
        result
    }
}

fn enumerate_directory(
    path: &Path,
    origin: DesktopOrigin,
) -> Result<Vec<OwnedDesktopEntry>, DesktopPlatformError> {
    let mut entries = Vec::new();
    let directory =
        fs::read_dir(path).map_err(|error| DesktopPlatformError::Enumeration(error.to_string()))?;
    for entry in directory {
        let entry = entry.map_err(|error| DesktopPlatformError::Enumeration(error.to_string()))?;
        let canonical_path = entry
            .path()
            .canonicalize()
            .map_err(|error| DesktopPlatformError::Enumeration(error.to_string()))?;
        let metadata = entry
            .metadata()
            .map_err(|error| DesktopPlatformError::Enumeration(error.to_string()))?;
        let attributes = std::os::windows::fs::MetadataExt::file_attributes(&metadata);
        entries.push(OwnedDesktopEntry {
            stable_identity: stable_file_identity(&canonical_path)?,
            display_name: entry.file_name().to_string_lossy().into_owned(),
            canonical_path,
            origin,
            folder: metadata.is_dir(),
            hidden: attributes & 0x2 != 0,
            system: attributes & 0x4 != 0,
        });
    }
    entries.sort_by(|left, right| left.stable_identity.cmp(&right.stable_identity));
    Ok(entries)
}

pub fn stable_file_identity(path: &Path) -> Result<String, DesktopPlatformError> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: The UTF-16 path is NUL-terminated and lives for the call. The
    // returned handle is closed exactly once after initialized metadata is read.
    unsafe {
        let handle = CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
        .map_err(|error| DesktopPlatformError::Identity(error.to_string()))?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        let result = GetFileInformationByHandle(handle, &mut info)
            .map(|()| {
                format!(
                    "winfile:{:08X}:{:08X}{:08X}",
                    info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
                )
            })
            .map_err(|error| DesktopPlatformError::Identity(error.to_string()));
        let _ = CloseHandle(handle);
        result
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssociationAdmission {
    Launched,
    ValidationFailed,
    LaunchFailed,
}

pub fn launch_association(path: &Path) -> Result<AssociationAdmission, DesktopPlatformError> {
    if !path.is_file() {
        return Ok(AssociationAdmission::ValidationFailed);
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| DesktopPlatformError::Association(error.to_string()))?;
    let wide: Vec<u16> = canonical.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: SHELLEXECUTEINFOW owns no borrowed data after the synchronous
    // admission call. lpFile is NUL-terminated and hProcess is closed once.
    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpFile: PCWSTR(wide.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        if let Err(error) = ShellExecuteExW(&mut info) {
            return if error.code().is_err() {
                Ok(AssociationAdmission::LaunchFailed)
            } else {
                Err(DesktopPlatformError::Association(error.to_string()))
            };
        }
        if info.hProcess != HANDLE::default() {
            let _ = CloseHandle(info.hProcess);
        }
        Ok(AssociationAdmission::Launched)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedWatcherEvent {
    pub action: u32,
    pub identity: String,
}

pub fn invoke_watcher_callback(
    fence: &CallbackFence,
    action: u32,
    identity: &str,
) -> CallbackResult<OwnedWatcherEvent> {
    let owned = identity.to_owned();
    fence.invoke(|| OwnedWatcherEvent {
        action,
        identity: owned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn known_desktop_enumeration_returns_owned_stable_values() {
        let entries = enumerate_known_desktops().unwrap();
        let identities: std::collections::BTreeSet<_> = entries
            .iter()
            .map(|entry| entry.stable_identity.clone())
            .collect();
        assert_eq!(identities.len(), entries.len());
        assert!(
            entries
                .iter()
                .all(|entry| entry.canonical_path.is_absolute())
        );
    }
    #[test]
    fn stable_file_identity_survives_rename() {
        let root =
            std::env::temp_dir().join(format!("superdesktop-identity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let before = root.join("before.txt");
        let after = root.join("after.txt");
        fs::write(&before, b"fixture").unwrap();
        let identity = stable_file_identity(&before).unwrap();
        fs::rename(&before, &after).unwrap();
        assert_eq!(stable_file_identity(&after).unwrap(), identity);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn watcher_callback_is_owned_and_no_unwind() {
        let fence = CallbackFence::default();
        assert_eq!(
            invoke_watcher_callback(&fence, 1, "identity"),
            CallbackResult::Returned(OwnedWatcherEvent {
                action: 1,
                identity: "identity".into()
            })
        );
    }
    #[test]
    fn association_rejects_non_file_without_spawning() {
        assert_eq!(
            launch_association(Path::new("Z:\\definitely-missing-superdesktop-file")),
            Ok(AssociationAdmission::ValidationFailed)
        );
    }

    #[test]
    fn real_fixture_file_association_reaches_admission_and_executes_hidden() {
        let root =
            std::env::temp_dir().join(format!("superdesktop-association-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let marker = root.join("launched.txt");
        let script = root.join("launch.cmd");
        fs::write(
            &script,
            format!("@echo off\r\n>\"{}\" echo launched\r\n", marker.display()),
        )
        .unwrap();
        assert_eq!(
            launch_association(&script).unwrap(),
            AssociationAdmission::Launched
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(fs::read_to_string(&marker).unwrap().trim(), "launched");
        fs::remove_dir_all(root).unwrap();
    }
}
