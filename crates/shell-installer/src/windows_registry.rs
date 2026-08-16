use std::path::Path;
use std::process::{Command, Output};

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
Set-ItemProperty -LiteralPath $p -Name 'Shell' -Type String -Value $v -ErrorAction Stop
"#;
const DELETE_SCRIPT: &str = r#"
$p = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
if (Test-Path -LiteralPath $p) {
    $item = Get-ItemProperty -LiteralPath $p -ErrorAction Stop
    if ($null -ne $item.PSObject.Properties['Shell']) {
        Remove-ItemProperty -LiteralPath $p -Name 'Shell' -ErrorAction Stop
    }
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
}
