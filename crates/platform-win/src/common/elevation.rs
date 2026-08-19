use std::{mem::size_of, os::windows::ffi::OsStrExt, path::Path, time::Duration};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_HIDE,
        },
    },
    core::PCWSTR,
};

pub fn run_elevated_helper(
    executable: &Path,
    arguments: &str,
    timeout: Duration,
) -> Result<u32, &'static str> {
    if !executable.is_absolute() || !executable.is_file() || arguments.contains('\0') {
        return Err("elevated-helper-invalid-input");
    }
    let executable = executable
        .canonicalize()
        .map_err(|_| "elevated-helper-canonicalize")?;
    let file = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let parameters = arguments.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let verb = "runas".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let directory = executable
        .parent()
        .ok_or("elevated-helper-parent")?
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: all UTF-16 buffers are terminated and remain live for the call. The returned
    // process handle is owned and closed on every path.
    unsafe { ShellExecuteExW(&mut info) }.map_err(|_| "elevated-helper-uac-rejected")?;
    if info.hProcess == HANDLE::default() {
        return Err("elevated-helper-no-process");
    }
    let wait_ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
    if unsafe { WaitForSingleObject(info.hProcess, wait_ms) } != WAIT_OBJECT_0 {
        let _ = unsafe { CloseHandle(info.hProcess) };
        return Err("elevated-helper-timeout");
    }
    let mut exit_code = 0u32;
    let result = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) }
        .map(|()| exit_code)
        .map_err(|_| "elevated-helper-exit-code");
    let _ = unsafe { CloseHandle(info.hProcess) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_inputs_without_uac() {
        assert_eq!(
            run_elevated_helper(Path::new("relative.exe"), "close-explorer", Duration::ZERO),
            Err("elevated-helper-invalid-input")
        );
    }

    #[test]
    fn source_uses_runas_waits_and_owns_the_process_handle() {
        let source = include_str!("elevation.rs");
        for required in [
            "\"runas\"",
            "SEE_MASK_NOCLOSEPROCESS",
            "WaitForSingleObject",
            "GetExitCodeProcess",
            "CloseHandle",
        ] {
            assert!(source.contains(required), "missing {required}");
        }
    }
}
