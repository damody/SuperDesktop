//! Verified Windows Explorer recovery target and restricted launcher.

use std::{
    ffi::{OsStr, c_void},
    fs,
    os::windows::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    ptr::null_mut,
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const WTD_UI_NONE: u32 = 2;
const WTD_REVOKE_NONE: u32 = 0;
const WTD_CHOICE_FILE: u32 = 1;
const WTD_STATEACTION_IGNORE: u32 = 0;
const WTD_CACHE_ONLY_URL_RETRIEVAL: u32 = 0x1000;

type RawHandle = isize;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const PROCESS_TERMINATE: u32 = 0x0001;
const SYNCHRONIZE: u32 = 0x0010_0000;
const WAIT_OBJECT_0: u32 = 0;
const SW_SHOW: i32 = 5;
const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
const INVALID_HANDLE_VALUE: RawHandle = -1;
const MAX_PATH: usize = 260;

#[repr(C)]
struct ProcessEntry32W {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    threads: u32,
    parent_process_id: u32,
    priority_class_base: i32,
    flags: u32,
    executable: [u16; MAX_PATH],
}

impl Default for ProcessEntry32W {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            usage: 0,
            process_id: 0,
            default_heap_id: 0,
            module_id: 0,
            threads: 0,
            parent_process_id: 0,
            priority_class_base: 0,
            flags: 0,
            executable: [0; MAX_PATH],
        }
    }
}

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}
#[repr(C)]
struct WinTrustFileInfo {
    cb_struct: u32,
    file_path: *const u16,
    file: RawHandle,
    known_subject: *const Guid,
}
#[repr(C)]
struct WinTrustData {
    cb_struct: u32,
    policy_callback_data: *mut c_void,
    sip_client_data: *mut c_void,
    ui_choice: u32,
    revocation_checks: u32,
    union_choice: u32,
    file_info: *mut WinTrustFileInfo,
    state_action: u32,
    state_data: RawHandle,
    url_reference: *const u16,
    provider_flags: u32,
    ui_context: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> RawHandle;
    fn CloseHandle(handle: RawHandle) -> i32;
    fn QueryFullProcessImageNameW(
        handle: RawHandle,
        flags: u32,
        path: *mut u16,
        length: *mut u32,
    ) -> i32;
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn WaitForSingleObject(handle: RawHandle, milliseconds: u32) -> u32;
    fn TerminateProcess(handle: RawHandle, exit_code: u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> RawHandle;
    fn Process32FirstW(snapshot: RawHandle, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: RawHandle, entry: *mut ProcessEntry32W) -> i32;
}
#[link(name = "wintrust")]
unsafe extern "system" {
    fn WinVerifyTrust(hwnd: RawHandle, action: *const Guid, data: *mut WinTrustData) -> i32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
    fn GetWindowThreadProcessId(window: isize, pid: *mut u32) -> u32;
    fn ShowWindow(window: isize, command: i32) -> i32;
    fn IsWindowVisible(window: isize) -> i32;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedExplorer {
    pub application: PathBuf,
    pub canonical_windows_directory: PathBuf,
    pub volume_serial: u32,
    pub file_index: u64,
    pub authenticode_verified: bool,
}

impl TrustedExplorer {
    pub fn resolve() -> Result<Self, &'static str> {
        let mut buffer = vec![0u16; 32_768];
        // SAFETY: the buffer is writable for the supplied element count.
        let length = unsafe { GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 || length as usize >= buffer.len() {
            return Err("windows-directory");
        }
        let windows = PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize]));
        let canonical_windows_directory = windows
            .canonicalize()
            .map_err(|_| "canonical-windows-directory")?;
        let application = canonical_windows_directory
            .join("explorer.exe")
            .canonicalize()
            .map_err(|_| "canonical-explorer")?;
        if application.parent() != Some(canonical_windows_directory.as_path()) {
            return Err("explorer-outside-windows-directory");
        }
        let metadata = fs::symlink_metadata(&application).map_err(|_| "explorer-metadata")?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("explorer-reparse-or-not-file");
        }
        if application
            .file_name()
            .and_then(OsStr::to_str)
            .map(|v| v.eq_ignore_ascii_case("explorer.exe"))
            != Some(true)
        {
            return Err("explorer-name");
        }
        let authenticode_verified = verify_authenticode(&application)?;
        let (_, identity) = super::guardian_lease::canonical_file_identity(
            application.to_str().ok_or("explorer-path-encoding")?,
        )
        .map_err(|_| "explorer-file-identity")?;
        Ok(Self {
            application,
            canonical_windows_directory,
            volume_serial: identity.volume_serial,
            file_index: identity.file_index,
            authenticode_verified,
        })
    }

    /// Uses an explicit absolute application, a fixed working directory, a
    /// minimal environment, null stdio and no inherited std handles.
    pub fn launch_restricted(&self) -> Result<Child, &'static str> {
        let current = Self::resolve()?;
        if current != *self {
            return Err("explorer-identity-changed");
        }
        let system_root = self.canonical_windows_directory.as_os_str();
        Command::new(&self.application)
            .current_dir(&self.canonical_windows_directory)
            .env_clear()
            .env("SystemRoot", system_root)
            .env("WINDIR", system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "explorer-spawn")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellRecoveryOutcome {
    ShownExisting { process_id: u32 },
    SpawnedVerified { process_id: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellShutdownOutcome {
    AlreadyAbsent,
    ClosedGracefully { process_id: u32 },
    Terminated { process_id: u32 },
}

/// Closes only the current-session shell process whose executable matches the verified inbox
/// Explorer. Every matching process is terminated by exact PID and its exit is observed before
/// shell takeover continues.
pub fn shutdown_trusted_explorer_shell() -> Result<ShellShutdownOutcome, &'static str> {
    let trusted = TrustedExplorer::resolve()?;
    let mut current_session = 0;
    // SAFETY: current process id is read-only and session output is writable.
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0 {
        return Err("explorer-shutdown-current-session");
    }
    // Explorer can own no Shell_TrayWnd yet still remain alive or can be restarted by Winlogon
    // between window probes. Enumerate the current session and validate each candidate by its
    // canonical executable before terminating the exact processes.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("explorer-process-snapshot");
    }
    let mut entry = ProcessEntry32W::default();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    let mut matched = Vec::new();
    while has_entry {
        let end = entry
            .executable
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(entry.executable.len());
        if String::from_utf16_lossy(&entry.executable[..end]).eq_ignore_ascii_case("explorer.exe") {
            matched.push(entry.process_id);
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    let _ = unsafe { CloseHandle(snapshot) };
    let mut terminated = None;
    for pid in matched {
        let mut session = 0;
        if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 || session != current_session {
            continue;
        }
        // SAFETY: handle is query/synchronize/terminate only and closed on every exit path below.
        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZE,
                0,
                pid,
            )
        };
        if process == 0 {
            continue;
        }
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) } != 0;
        let matches = queried
            && PathBuf::from(String::from_utf16_lossy(&path[..length as usize]))
                .canonicalize()
                .is_ok_and(|observed| observed == trusted.application);
        if !matches {
            let _ = unsafe { CloseHandle(process) };
            continue;
        }
        if unsafe { TerminateProcess(process, 0) } == 0 {
            let _ = unsafe { CloseHandle(process) };
            return Err("explorer-shutdown-failed");
        }
        if unsafe { WaitForSingleObject(process, 2_000) } != WAIT_OBJECT_0 {
            let _ = unsafe { CloseHandle(process) };
            return Err("explorer-termination-not-observed");
        }
        let _ = unsafe { CloseHandle(process) };
        terminated = Some(pid);
    }
    Ok(
        terminated.map_or(ShellShutdownOutcome::AlreadyAbsent, |process_id| {
            ShellShutdownOutcome::Terminated { process_id }
        }),
    )
}

/// Reports whether the current interactive session still has a live shell window owned by the
/// verified system Explorer binary. This is read-only and does not start or show Explorer.
pub fn trusted_explorer_shell_present() -> Result<bool, &'static str> {
    let trusted = TrustedExplorer::resolve()?;
    let mut current_session = 0;
    // SAFETY: current process id is read-only and session output is writable.
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0 {
        return Err("explorer-presence-current-session");
    }
    for class in ["Shell_TrayWnd", "Progman"] {
        let class = OsStr::new(class)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: class is terminated; null title matches any window.
        let window = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
        if window == 0 {
            continue;
        }
        let mut pid = 0;
        // SAFETY: window is query-only and pid is writable.
        if unsafe { GetWindowThreadProcessId(window, &mut pid) } == 0 {
            continue;
        }
        let mut session = 0;
        // SAFETY: pid is from a live window and session is writable.
        if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 || session != current_session {
            continue;
        }
        // SAFETY: query-only handle, owned and closed on this path.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process == 0 {
            continue;
        }
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        // SAFETY: process is live and path/length are valid outputs.
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) } != 0;
        // SAFETY: this function owns the process handle.
        let _ = unsafe { CloseHandle(process) };
        if !queried {
            continue;
        }
        let observed = PathBuf::from(String::from_utf16_lossy(&path[..length as usize]));
        let Ok(observed) = observed.canonicalize() else {
            continue;
        };
        if observed == trusted.application {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Restores the existing Explorer Shell in the current interactive session,
/// spawning the verified system target only when no usable shell window exists.
pub fn recover_explorer_shell() -> Result<ShellRecoveryOutcome, &'static str> {
    let trusted = TrustedExplorer::resolve()?;
    let mut current_session = 0;
    // SAFETY: current process id is read-only and session output is writable.
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0 {
        return Err("recovery-current-session");
    }
    for class in ["Shell_TrayWnd", "Progman"] {
        let class = OsStr::new(class)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: class is terminated; null title matches any window.
        let window = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
        if window == 0 {
            continue;
        }
        let mut pid = 0;
        // SAFETY: window is query-only and pid is writable.
        if unsafe { GetWindowThreadProcessId(window, &mut pid) } == 0 {
            continue;
        }
        let mut session = 0;
        // SAFETY: pid is from a live window and session is writable.
        if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 || session != current_session {
            continue;
        }
        // SAFETY: query-only handle, owned and closed on this path.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process == 0 {
            continue;
        }
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        // SAFETY: process is live and path/length are valid outputs.
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) } != 0;
        // SAFETY: this function owns the process handle.
        let _ = unsafe { CloseHandle(process) };
        if !queried {
            continue;
        }
        let observed = PathBuf::from(String::from_utf16_lossy(&path[..length as usize]));
        let Ok(observed) = observed.canonicalize() else {
            continue;
        };
        if observed != trusted.application {
            continue;
        }
        // SAFETY: validated Explorer-owned shell window; SW_SHOW is reversible/idempotent.
        let _ = unsafe { ShowWindow(window, SW_SHOW) };
        // SAFETY: read-only visibility check of the same live window.
        if unsafe { IsWindowVisible(window) } != 0 {
            return Ok(ShellRecoveryOutcome::ShownExisting { process_id: pid });
        }
    }
    let child = trusted.launch_restricted()?;
    Ok(ShellRecoveryOutcome::SpawnedVerified {
        process_id: child.id(),
    })
}

fn verify_authenticode(path: &Path) -> Result<bool, &'static str> {
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut file = WinTrustFileInfo {
        cb_struct: size_of::<WinTrustFileInfo>() as u32,
        file_path: wide.as_ptr(),
        file: 0,
        known_subject: std::ptr::null(),
    };
    let mut data = WinTrustData {
        cb_struct: size_of::<WinTrustData>() as u32,
        policy_callback_data: null_mut(),
        sip_client_data: null_mut(),
        ui_choice: WTD_UI_NONE,
        revocation_checks: WTD_REVOKE_NONE,
        union_choice: WTD_CHOICE_FILE,
        file_info: &mut file,
        state_action: WTD_STATEACTION_IGNORE,
        state_data: 0,
        url_reference: std::ptr::null(),
        provider_flags: WTD_CACHE_ONLY_URL_RETRIEVAL,
        ui_context: 0,
    };
    let action = Guid {
        data1: 0x00aac56b,
        data2: 0xcd44,
        data3: 0x11d0,
        data4: [0x8c, 0xc2, 0x00, 0xc0, 0x4f, 0xc2, 0x95, 0xee],
    };
    // SAFETY: all WinTrust records and the terminated path remain live for the synchronous call.
    let status = unsafe { WinVerifyTrust(-1, &action, &mut data) };
    if status == 0 {
        Ok(true)
    } else {
        Err("explorer-authenticode")
    }
}

use std::mem::size_of;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_explorer_is_absolute_canonical_non_reparse_and_signed() {
        let explorer = TrustedExplorer::resolve().unwrap();
        assert!(explorer.application.is_absolute());
        assert!(explorer.authenticode_verified);
        assert_eq!(
            explorer.application.parent(),
            Some(explorer.canonical_windows_directory.as_path())
        );
    }
    #[test]
    fn path_and_cwd_substitutes_are_not_inputs_to_resolution() {
        let before_path = std::env::var_os("PATH");
        let before_dir = std::env::current_dir().unwrap();
        let explorer = TrustedExplorer::resolve().unwrap();
        assert_eq!(std::env::var_os("PATH"), before_path);
        assert_eq!(std::env::current_dir().unwrap(), before_dir);
        assert_ne!(explorer.application, before_dir.join("explorer.exe"));
    }

    #[test]
    fn live_recovery_prefers_existing_verified_explorer_without_process_spawn() {
        if !trusted_explorer_shell_present().unwrap_or(false) {
            return;
        }
        let before = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "@(Get-Process explorer -ErrorAction SilentlyContinue).Count",
            ])
            .output()
            .unwrap();
        let before = String::from_utf8_lossy(&before.stdout)
            .trim()
            .parse::<u32>()
            .unwrap();
        let result = recover_explorer_shell().unwrap();
        assert!(matches!(result, ShellRecoveryOutcome::ShownExisting { .. }));
        let after = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "@(Get-Process explorer -ErrorAction SilentlyContinue).Count",
            ])
            .output()
            .unwrap();
        let after = String::from_utf8_lossy(&after.stdout)
            .trim()
            .parse::<u32>()
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn takeover_source_is_exact_bounded_and_recovery_remains_available() {
        let source = include_str!("explorer_recovery.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "shutdown_trusted_explorer_shell",
            "ProcessIdToSessionId",
            "CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS",
            "QueryFullProcessImageNameW",
            "observed == trusted.application",
            "TerminateProcess(process, 0)",
            "WaitForSingleObject(process, 2_000)",
            "recover_explorer_shell",
        ] {
            assert!(
                production.contains(required),
                "missing takeover guard: {required}"
            );
        }
        for forbidden in ["taskkill", "/im explorer.exe", "TerminateProcessByName"] {
            assert!(!production.contains(forbidden));
        }
    }
}
