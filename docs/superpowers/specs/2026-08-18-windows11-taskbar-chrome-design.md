# Windows 11 taskbar chrome alignment

## Outcome

SuperDesktop's owned taskbar visually aligns with Windows 11 build 26200 while preserving the requested extensions: one to three rows, optional labels, long labeled indicators, short icon-only indicators, progress, grouped tasks and attention animation. No visible control is delegated to Explorer.

## Scope

This change owns taskbar presentation: shared light/high-contrast tokens, row background and border, Start/Search/Task View/fixed/task button geometry, hover/pressed/focus states, label width, indicator placement, notification spacing, network/volume/input/clock controls, localized Search and compact input-language presentation. It does not change AppBar ownership, window tracking, grouping, pin persistence, notification ingestion or system-status providers.

## Geometry

Each row retains a 40-logical-pixel hit target so existing multi-row AppBar reservations remain stable. Icon-only controls use 40–44px cells. Labeled task buttons use a 160px bounded width instead of the oversized 190px width; their indicator spans 144px, preserving the requested long Windows-style underline. Icon-only task indicators remain short. The status region uses compact 36px icon cells, a 44px input cell and an 82px two-line clock cell with consistent gaps.

## Presentation

`TaskbarChromeTokens` provides Windows 11 panel, top border, text, secondary text, hover, pressed, focus and attention colors for light/high contrast. Every interactive taskbar control receives hover, active and focus-visible styling. The taskbar search label is localized. Input-language presentation maps known locales to Windows-like compact labels (`中`, `ENG`) without changing provider values. Time and date use explicit typography rather than inheriting task-label size.

## Explorer independence

Production taskbar composition may use documented Windows data and callback protocols, but must not invoke `explorer.exe`, `Shell_TrayWnd`, Start/Search/ShellExperience hosts or Explorer tray UI. Unavailable providers remain independent and truthful.

## Verification

Automated tests cover tokens, high contrast, row geometry, labeled/icon-only widths, indicators, localized Search/IME, system-region bounds and forbidden delegation. Headful evidence captures light and high-contrast taskbars at 175% with two rows, labeled/icon-only tasks, progress/attention, notification overflow, IME and clock. Full workspace tests, clippy, strict/detailed OpenSpec validation, release hashes and both installers must pass. The change remains unarchived.
