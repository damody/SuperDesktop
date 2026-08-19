use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const INSTALL_DIRECTORY_ENV: &str = "SUPERDESKTOP_QUIESCE_INSTALL_DIRECTORY";

const QUIESCE_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$root = [Environment]::GetEnvironmentVariable('SUPERDESKTOP_QUIESCE_INSTALL_DIRECTORY', 'Process')
if ([string]::IsNullOrWhiteSpace($root) -or -not [IO.Path]::IsPathRooted($root)) { throw 'invalid install directory' }
$root = [IO.Path]::GetFullPath($root).TrimEnd('\')
$names = @(
    'SuperExplorer.exe',
    'superdesktop-app.exe',
    'superdesktop-guardian.exe',
    'shell-provider-host.exe',
    'notification-area-host.exe',
    'system-status-host.exe',
    'taskbar-state-host.exe',
    'explorer-extension-broker.exe',
    'explorer-extension-worker.exe',
    'superexplorer-mft-helper.exe'
)
$expected = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($name in $names) { [void]$expected.Add([IO.Path]::GetFullPath((Join-Path $root $name))) }
$targets = @(
    Get-Process -ErrorAction SilentlyContinue | ForEach-Object {
        try { $path = $_.Path } catch { $path = $null }
        if ($path -and $expected.Contains([IO.Path]::GetFullPath($path))) { $_ }
    }
)
if ($targets.Count -eq 0) { [Console]::Out.Write('{"closed":0}'); exit 0 }
$ids = @($targets | ForEach-Object { $_.Id })
Stop-Process -Id $ids -Force -ErrorAction Stop
$deadline = [DateTime]::UtcNow.AddSeconds(5)
do {
    $alive = @($ids | Where-Object { $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue) })
    if ($alive.Count -eq 0) { break }
    Start-Sleep -Milliseconds 50
} while ([DateTime]::UtcNow -lt $deadline)
if ($alive.Count -ne 0) { throw ('processes did not exit: ' + ($alive -join ',')) }
[Console]::Out.Write((@{ closed = $ids.Count } | ConvertTo-Json -Compress))
"#;

pub fn quiesce_installation(install_directory: &Path) -> Result<String, String> {
    if !install_directory.is_absolute() {
        return Err("install directory must be absolute".into());
    }
    let current = std::env::current_exe().map_err(|error| format!("current-exe:{error}"))?;
    if current.parent().is_some_and(|parent| {
        parent
            .to_string_lossy()
            .eq_ignore_ascii_case(&install_directory.to_string_lossy())
    }) {
        return Err("process closer must run outside the installation directory".into());
    }
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            QUIESCE_SCRIPT,
        ])
        .env(INSTALL_DIRECTORY_ENV, install_directory)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("powershell-start:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "powershell-exit={}:{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("powershell-output:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_and_in_place_targets_before_process_enumeration() {
        assert!(quiesce_installation(Path::new("relative")).is_err());
        let current = std::env::current_exe().unwrap();
        assert!(quiesce_installation(current.parent().unwrap()).is_err());
    }

    #[test]
    fn script_uses_exact_paths_and_a_bounded_verified_wait() {
        for required in [
            "HashSet[string]",
            "StringComparer]::OrdinalIgnoreCase",
            "Stop-Process -Id $ids -Force",
            "AddSeconds(5)",
            "processes did not exit",
        ] {
            assert!(QUIESCE_SCRIPT.contains(required), "missing {required}");
        }
        assert!(!QUIESCE_SCRIPT.contains("taskkill"));
    }
}
