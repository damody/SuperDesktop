## Why

The owned input-language button currently models Windows input methods only as HKLs. That collapses distinct TSF profiles such as Cangjie and Bopomofo into guessed labels, prevents exact same-language profile activation, and exposes a local simulated detail page instead of the Language preferences action shown by Windows.

## What Changes

- Extend the additive system-status profile DTO with authoritative input-method description, profile kind, and exact TSF/HKL identity metadata.
- Enumerate enabled Windows TSF input processor and keyboard-layout profiles with an HKL fallback, bounded ownership, and stable identities.
- Activate a freshly revalidated exact TSF or keyboard profile and preserve the existing foreground observation fence.
- Add a fixed, typed Language preferences command that can launch only `ms-settings:regionlanguage` and returns admission rather than fake visibility.
- Replace the local simulated keyboard-settings page with a scrollable Windows-style profile list and working Language preferences footer.
- Add protocol, platform, host/client, UI, real-profile, accessibility, theme, DPI, and headful validation with user identities redacted from committed evidence.

## Capabilities

### New Capabilities

- `owned-input-method-flyout`: Defines authoritative bounded TSF/HKL profile discovery, exact activation, fixed Language preferences launch, complete owned flyout behavior, and Explorer-free evidence.

### Modified Capabilities

None. Historical system-flyout changes are archived and no active base capability currently owns the complete input-method contract.

## Impact

Affected code spans `shell-provider-protocol`, Windows feature bindings, `platform-win`, `system-status-host`, `superdesktop-app`, `taskbar-ui`, headful scripts, and evidence. Snapshot fields are additive and old JSON remains accepted. The host/client protocol minor version advances. The change adds no crate, arbitrary URI, credential, settings mutation, input-method installation/removal, synthetic Win+Space, or Explorer dependency.
