# Windows 11 owned system flyouts

## Outcome

SuperDesktop shall render the input-language, volume, network/power, and calendar surfaces as owned GPUI popup windows that visually converge on Windows 11 build 26200 while remaining usable without `explorer.exe`. Documented Windows providers remain the source of status data and typed commands; no system Quick Settings, notification-center, Settings URI, or Explorer-owned shell surface may be launched.

## Scope

This batch restyles and tightens the four existing system flyouts. It preserves the `SystemStatusSnapshot`, `SystemStatusAction`, host isolation, exactly-once command terminals, and one-open-flyout lifecycle. It does not add Wi-Fi, Bluetooth, brightness, media-session, notification-history, or calendar-event mutation because those contracts do not exist yet; unavailable actions must not be imitated visually.

## Selected approach

Keep the independent flyout architecture and introduce one presentation token family shared by every surface. This is preferred over building a combined Quick Settings panel because the existing provider contract can truthfully power the current four surfaces, and preferred over delegating to Windows because delegation violates the Explorer-free product boundary.

## Presentation architecture

`SystemFlyoutChromeTokens` is a presentation-only value in `taskbar-ui`. It defines panel, card, border, primary/secondary text, accent, hover, pressed, focus, unavailable, shadow geometry, corner radii, typography, and spacing for light, dark, and high-contrast modes. The app passes locale/theme/high-contrast presentation inputs when constructing `SystemFlyoutView`; provider DTOs remain unchanged.

All actionable rows use stable roles, names, focus geometry, pointer states, Enter/Space activation, and bounded hit targets. Provider failure renders a localized status row without an action. Escape, activation loss, repeated invocation, and switching flyout kinds preserve the existing deterministic dismissal contract.

## Surface behavior

### Input language

The popup uses a compact header and 52 logical-pixel profile rows. Each row shows a bounded language tag, full provider display name, and an accent selection mark. The active profile remains identifiable by UIA state and full provider identity. Selecting another row emits only `ActivateInputProfile`; success remains gated on the observed confirming snapshot.

### Volume

The popup uses a Windows-style horizontal arrangement: mute button, slider track/thumb, and percentage. The slider preserves Arrow/Home/End keyboard behavior and typed `SetVolume`; the mute button emits `SetMute`. Pointer hit zones remain bounded and no optimistic provider value is fabricated.

### Network and power

The popup presents real network connectivity and power/battery summaries in compact Windows-style cards. Since the current protocol exposes no network or power mutation, the cards remain informational and do not pretend to be toggles. Not-present battery is distinct from provider failure.

### Calendar

The popup shows localized date/time metadata, a month heading, localized weekday labels, and a stable 7×6 grid. The observed current day uses accent fill plus high-contrast geometry. Month navigation is presentation-only until a dedicated view-month state is added; this batch must not expose inert chevrons.

## Popup geometry

Each kind has a bounded logical width and content-driven height. Placement is right-aligned to the monitor work area, offset above the owned taskbar rows, converted explicitly between logical GPUI units and physical monitor coordinates, and clamped on small or negative-origin monitors. Calendar is wider/taller than volume; input height is bounded by the number of profiles.

## Explorer independence and failure handling

Production composition rejects `explorer.exe`, `Shell_TrayWnd`, `StartMenuExperienceHost`, `ShellExperienceHost`, system Quick Settings entry points, and `ms-settings:`. Status host loss clears only host-owned state and shows truthful unavailable presentation. Popup creation failure leaves the taskbar alive. Ordinary provider or popup failure never invokes guardian rollback; rollback remains restricted to shell takeover failure.

## Verification

Blocking gates are:

- `G-SYSTEM-FLYOUT-CHROME`: shared light/dark/high-contrast presentation and bounded geometry.
- `G-SYSTEM-FLYOUT-A11Y`: stable dialog/control roles, names, state, keyboard activation, Escape, and focus behavior.
- `G-SYSTEM-FLYOUT-TRUTH`: no fake mutation, fixed provider value, or stale success presentation.
- `G-SHELL-NONINTERFERENCE`: source and process evidence show no delegated shell surface.
- `G-TRACE`: every atomic task has unique evidence linkage.
- `G-PACKAGE`: release and standalone/combined installers build without launch and hashes are recorded.

Automated verification runs formatting, locked/offline workspace tests, clippy with warnings denied, geometry/locale/state tests, and forbidden-source guards. Headful verification captures input, volume, network/power, and calendar at 175% DPI in light, dark, and high contrast; inspects UIA and keyboard routes; and records Explorer-free process observation. Packaging verification rebuilds both NSIS installers without launching them.

## Adjustment policy

- **A — refinement:** task order, helper extraction, fixture data, or evidence paths may change without altering scope, requirements, public contracts, or gates.
- **B — correction:** unreasonable presentation or geometry assumptions inside this scope require design/spec/task correction and revalidation; affected evidence becomes stale.
- **C — material change:** new provider mutations, undocumented Windows interfaces, delegated shell UI, permission changes, weaker gates, or broader product scope require explicit approval.

## Rollback

Reverting this batch restores the prior owned flyout presentation without changing settings or protocol versions. Rollback must not restore hard-coded status values or delegate presentation to Windows shell processes.
