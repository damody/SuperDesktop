# Windows product identity evidence

This L2 creates and verifies the three Windows binary identities without starting a UI, Shell mode, or external Explorer process.

## Built resources

Each product crate has a build script that invokes the installed `llvm-rc.exe` and embeds:

- VERSIONINFO with `CompanyName=SuperDesktop`, `ProductName=SuperDesktop`, version `0.1.0.0`, a unique internal name, and expected original filename.
- RCDATA resource 101 containing its unique AppUserModelID.
- a generated minimal ICO compiled as group icon resource 201. This avoids a source dependency on an external icon asset while producing an actual Windows icon resource.

`identity-verifier.json` verifies FileVersionInfo plus direct Win32 resource API reads for all three built EXEs. The binary SHA-256 values are included in `identity-contract-inputs.sha256`; its overall contract hash is `650C91A2BFD2BB65E0C7EA8B1ED29792A126871788E01D49BE0815E4697F1C2A`.

## Identity mapping

| Binary | AppUserModelID | Original filename |
| --- | --- | --- |
| `superdesktop-app.exe` | `com.superdesktop.shell` | `SuperDesktop.exe` |
| `superdesktop-guardian.exe` | `com.superdesktop.guardian` | `SuperDesktopGuardian.exe` |
| `superdesktop-test-support.exe` | `com.superdesktop.test-support` | `SuperDesktopTestSupport.exe` |

The collision fixture is rejected with `IDENTITY_COLLISION`; the missing-field fixture is rejected with `IDENTITY_MISSING_FIELD`.

`llvm-rc-version.txt` is a stale failed probe because this tool does not implement `--version`; the successful compiler discovery and actual binary build replace it with `llvm-rc-tool.txt` and `identity-binary-build.txt`.
