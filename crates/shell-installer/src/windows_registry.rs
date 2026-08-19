use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use std::os::windows::process::CommandExt;

use serde::Deserialize;

use crate::{InstallerError, ShellRegistry};

const VALUE_ENV: &str = "SUPERDESKTOP_INSTALLER_SHELL_VALUE";
const READ_SCRIPT: &str = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$p = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
try {
    $v = Get-ItemPropertyValue -LiteralPath $p -Name 'Shell' -ErrorAction Stop
    [Console]::Out.Write((@{ exists = $true; value = [string]$v } | ConvertTo-Json -Compress))
} catch {
    if (Test-Path -LiteralPath $p) {
        $item = Get-ItemProperty -LiteralPath $p -ErrorAction Stop
        if ($null -eq $item.PSObject.Properties['Shell']) {
            [Console]::Out.Write((@{ exists = $false } | ConvertTo-Json -Compress))
            exit 0
        }
    }
    [Console]::Error.Write($_.Exception.Message)
    exit 31
}
"#;
const WRITE_SCRIPT: &str = r#"
$p = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
$v = [Environment]::GetEnvironmentVariable('SUPERDESKTOP_INSTALLER_SHELL_VALUE', 'Process')
if ($null -eq $v) { [Console]::Error.Write('missing shell value environment'); exit 32 }
$class = 'Registry::HKEY_CURRENT_USER\Software\Classes\CLSID\{56FDF344-FD6D-11d0-958A-006097C9A090}'
$server = Join-Path $class 'LocalServer32'
$hadClass = Test-Path -LiteralPath $class
$oldOwner = if ($hadClass) { [string](Get-Item -LiteralPath $class -ErrorAction Stop).GetValue('') } else { $null }
$oldServer = if (Test-Path -LiteralPath $server) { [string](Get-Item -LiteralPath $server -ErrorAction Stop).GetValue('') } else { $null }
function Restore-TaskbarClass {
    if (-not $hadClass) {
        if (Test-Path -LiteralPath $class) { Remove-Item -LiteralPath $class -Recurse -Force -ErrorAction SilentlyContinue }
        return
    }
    New-Item -Path $class -Force -ErrorAction Stop | Out-Null
    Set-Item -LiteralPath $class -Value $oldOwner -ErrorAction Stop
    if ($null -ne $oldServer) {
        New-Item -Path $server -Force -ErrorAction Stop | Out-Null
        Set-Item -LiteralPath $server -Value $oldServer -ErrorAction Stop
    } elseif (Test-Path -LiteralPath $server) {
        Remove-Item -LiteralPath $server -Recurse -Force -ErrorAction Stop
    }
}
if ($v -match '^"([^"]+superdesktop-app\.exe)"\s+--shell(?:\s|$)') {
    $appPath = $Matches[1]
    if ($appPath.StartsWith('\\?\')) { $appPath = $appPath.Substring(4) }
    $taskbarHost = Join-Path (Split-Path -Parent $appPath) 'taskbar-state-host.exe'
    if (-not (Test-Path -LiteralPath $taskbarHost -PathType Leaf)) { [Console]::Error.Write('taskbar state host missing'); exit 34 }
    if ($oldOwner -and $oldOwner -ne 'SuperDesktop Taskbar Communication') { [Console]::Error.Write('per-user taskbar COM registration already owned'); exit 35 }
}
try {
    if ($v -match '^"([^"]+superdesktop-app\.exe)"\s+--shell(?:\s|$)') {
        $appPath = $Matches[1]
        if ($appPath.StartsWith('\\?\')) { $appPath = $appPath.Substring(4) }
        $taskbarHost = Join-Path (Split-Path -Parent $appPath) 'taskbar-state-host.exe'
        New-Item -Path $class -Force -ErrorAction Stop | Out-Null
        Set-Item -LiteralPath $class -Value 'SuperDesktop Taskbar Communication' -ErrorAction Stop
        New-Item -Path $server -Force -ErrorAction Stop | Out-Null
        Set-Item -LiteralPath $server -Value ('"' + $taskbarHost + '"') -ErrorAction Stop
    } elseif ($oldOwner -eq 'SuperDesktop Taskbar Communication') {
        Remove-Item -LiteralPath $class -Recurse -Force -ErrorAction Stop
    }
    New-ItemProperty -LiteralPath $p -Name 'Shell' -PropertyType String -Value $v -Force -ErrorAction Stop | Out-Null
} catch {
    Restore-TaskbarClass
    throw
}
"#;
const DELETE_SCRIPT: &str = r#"
$class = 'Registry::HKEY_CURRENT_USER\Software\Classes\CLSID\{56FDF344-FD6D-11d0-958A-006097C9A090}'
$server = Join-Path $class 'LocalServer32'
$hadClass = Test-Path -LiteralPath $class
$oldOwner = if ($hadClass) { [string](Get-Item -LiteralPath $class -ErrorAction Stop).GetValue('') } else { $null }
$oldServer = if (Test-Path -LiteralPath $server) { [string](Get-Item -LiteralPath $server -ErrorAction Stop).GetValue('') } else { $null }
$p = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
try {
    if ($oldOwner -eq 'SuperDesktop Taskbar Communication') { Remove-Item -LiteralPath $class -Recurse -Force -ErrorAction Stop }
    if (Test-Path -LiteralPath $p) {
        $item = Get-ItemProperty -LiteralPath $p -ErrorAction Stop
        if ($null -ne $item.PSObject.Properties['Shell']) { Remove-ItemProperty -LiteralPath $p -Name 'Shell' -ErrorAction Stop }
    }
} catch {
    if ($hadClass) {
        New-Item -Path $class -Force -ErrorAction Stop | Out-Null
        Set-Item -LiteralPath $class -Value $oldOwner -ErrorAction Stop
        if ($null -ne $oldServer) {
            New-Item -Path $server -Force -ErrorAction Stop | Out-Null
            Set-Item -LiteralPath $server -Value $oldServer -ErrorAction Stop
        }
    }
    throw
}
"#;
const IDENTITY_SCRIPT: &str = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$appPath = [Environment]::GetEnvironmentVariable('SUPERDESKTOP_INSTALLER_APP_PATH', 'Process')
$guardianPath = [Environment]::GetEnvironmentVariable('SUPERDESKTOP_INSTALLER_GUARDIAN_PATH', 'Process')
if ([string]::IsNullOrWhiteSpace($appPath) -or [string]::IsNullOrWhiteSpace($guardianPath)) { exit 33 }
$app = (Get-Item -LiteralPath $appPath -ErrorAction Stop).VersionInfo
$guardian = (Get-Item -LiteralPath $guardianPath -ErrorAction Stop).VersionInfo
[Console]::Out.Write((@{
    app_original = [string]$app.OriginalFilename
    app_product = [string]$app.ProductName
    guardian_original = [string]$guardian.OriginalFilename
    guardian_product = [string]$guardian.ProductName
} | ConvertTo-Json -Compress))
"#;

/// Safe, Unicode-preserving adapter around the Windows inbox PowerShell
/// Registry provider. The mutation script is fixed and receives the value only
/// through a child-process environment variable, so registry data cannot become
/// script text.
#[derive(Default)]
pub struct WindowsShellRegistry;

impl ShellRegistry for WindowsShellRegistry {
    fn read_shell(&mut self) -> Result<Option<String>, InstallerError> {
        let output = powershell(READ_SCRIPT, None)?;
        if !output.status.success() {
            return Err(command_error("read", &output));
        }
        parse_observation(&output.stdout)
    }

    fn write_shell(&mut self, shell: &str) -> Result<(), InstallerError> {
        successful(powershell(WRITE_SCRIPT, Some(shell))?, "write")
    }

    fn delete_shell(&mut self) -> Result<(), InstallerError> {
        successful(powershell(DELETE_SCRIPT, None)?, "delete")
    }
}

#[derive(Deserialize)]
struct Observation {
    exists: bool,
    value: Option<String>,
}

#[derive(Deserialize)]
struct ProductIdentity {
    app_original: String,
    app_product: String,
    guardian_original: String,
    guardian_product: String,
}

pub(crate) fn verify_product_identity(
    app_path: &Path,
    guardian_path: &Path,
) -> Result<(), InstallerError> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            IDENTITY_SCRIPT,
        ])
        .env("SUPERDESKTOP_INSTALLER_APP_PATH", app_path)
        .env("SUPERDESKTOP_INSTALLER_GUARDIAN_PATH", guardian_path)
        .output()
        .map_err(|error| InstallerError::PreflightRejected(format!("identity-probe:{error}")))?;
    if !output.status.success() {
        return Err(InstallerError::PreflightRejected(format!(
            "identity-probe:exit={}",
            output.status.code().unwrap_or(-1)
        )));
    }
    let identity: ProductIdentity = serde_json::from_slice(&output.stdout).map_err(|error| {
        InstallerError::PreflightRejected(format!("identity-probe-output:{error}"))
    })?;
    if identity.app_original != "SuperDesktop.exe"
        || identity.app_product != "SuperDesktop"
        || identity.guardian_original != "SuperDesktopGuardian.exe"
        || identity.guardian_product != "SuperDesktop"
    {
        return Err(InstallerError::PreflightRejected(
            "binary product identity mismatch".into(),
        ));
    }
    Ok(())
}

pub(crate) fn probe_guardian_recovery(guardian_path: &Path) -> Result<(), InstallerError> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut child = Command::new(guardian_path)
        .arg("--installer-recovery-probe")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| {
            InstallerError::PreflightRejected(format!("guardian-recovery-probe:{error}"))
        })?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(InstallerError::PreflightRejected(format!(
                    "guardian-recovery-probe:exit={}",
                    status.code().unwrap_or(-1)
                )));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InstallerError::PreflightRejected(
                    "guardian-recovery-probe:timeout".into(),
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InstallerError::PreflightRejected(format!(
                    "guardian-recovery-probe:wait:{error}"
                )));
            }
        }
    }
}

fn powershell(script: &str, value: Option<&str>) -> Result<Output, InstallerError> {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        script,
    ]);
    if let Some(value) = value {
        command.env(VALUE_ENV, value);
    } else {
        command.env_remove(VALUE_ENV);
    }
    command
        .output()
        .map_err(|error| InstallerError::Registry(format!("powershell.exe:{error}")))
}

fn successful(output: Output, operation: &str) -> Result<(), InstallerError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(operation, &output))
    }
}

fn command_error(operation: &str, output: &Output) -> InstallerError {
    InstallerError::Registry(format!(
        "powershell.exe:{operation}:exit={}:{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn parse_observation(bytes: &[u8]) -> Result<Option<String>, InstallerError> {
    let observation: Observation = serde_json::from_slice(bytes).map_err(|error| {
        InstallerError::Registry(format!("powershell.exe:invalid-observation:{error}"))
    })?;
    match (observation.exists, observation.value) {
        (true, Some(value)) => Ok(Some(value)),
        (false, None) => Ok(None),
        _ => Err(InstallerError::Registry(
            "powershell.exe:inconsistent-observation".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unicode_and_exact_absence() {
        assert_eq!(
            parse_observation(
                r#"{"exists":true,"value":"\"C:\\使用者\\桌面.exe\" --shell"}"#.as_bytes(),
            )
            .unwrap(),
            Some(r#""C:\使用者\桌面.exe" --shell"#.into())
        );
        assert_eq!(
            parse_observation(br#"{"exists":false,"value":null}"#).unwrap(),
            None
        );
    }

    #[test]
    fn write_script_uses_the_windows_powershell_registry_type_contract() {
        assert!(WRITE_SCRIPT.contains("New-ItemProperty"));
        assert!(WRITE_SCRIPT.contains("-PropertyType String"));
        assert!(WRITE_SCRIPT.contains("$appPath.Substring(4)"));
        assert!(WRITE_SCRIPT.contains("$taskbarHost"));
        assert!(!WRITE_SCRIPT.contains("$host ="));
        assert!(!WRITE_SCRIPT.contains("Set-ItemProperty -LiteralPath $p -Name 'Shell' -Type"));
    }
}
