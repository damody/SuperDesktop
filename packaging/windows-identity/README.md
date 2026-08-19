# Windows notification-listener identity

`UserNotificationListener` requires package identity and the
`userNotificationListener` capability. SuperDesktop uses a package with external location so
the existing Win32 installer and executable layout remain unchanged.

Build and sign the identity package with a trusted `CN=SuperDesktop` certificate:

```powershell
./scripts/build-windows-identity-package.ps1 `
  -PfxPath <certificate.pfx> `
  -PfxPassword (Read-Host -AsSecureString) `
  -OutputPath <SuperDesktop.WindowsShell.msix>
```

Include it in a release directory by passing `-SignedIdentityPackage` to
`package-superdesktop.ps1`, then register it against that exact directory:

```powershell
./scripts/register-windows-identity-package.ps1 `
  -PackagePath <SuperDesktop.WindowsShell.msix> `
  -InstallDirectory <SuperDesktop release directory>
```

The registration script rejects unsigned packages, mismatched publishers, and directories that
do not contain `notification-area-host.exe`.
