## Why

SuperDesktop currently has task switching but does not reproduce the taskbar states ordinary Windows applications expect: running indicators use arbitrary full-width borders, real `ITaskbarList3` progress is not consumed, and `FlashWindowEx` attention requests do not produce Windows-style flashing. These gaps remain visibly different from the built-in Windows 11 taskbar and prevent Explorer-free parity.

## What Changes

- Render Windows 11-style running indicators for every available task, with distinct active, inactive, minimized and grouped geometry.
- Add an isolated, generation-bound taskbar-state compatibility provider for documented progress and attention behavior.
- Consume real normal, paused, error and indeterminate progress with Windows grouping priority and least-progress selection.
- Observe finite and foreground-terminated attention requests with system cadence, bounded flashing and steady post-flash highlight.
- Keep active, minimized, grouped, progress, badge, attention and availability state independent.
- Add high-contrast, reduced-motion, accessibility, stress, headful comparison and installer evidence.

## Capabilities

### New Capabilities

- `windows-taskbar-visual-states`: Windows-compatible running indicators, progress overlays, attention flashing, reconciliation and Explorer-free verification.

### Modified Capabilities

None.

## Impact

Affected code includes `shell-provider-protocol`, `platform-win`, a taskbar-state broker/compatibility boundary, `taskbar-ui`, `superdesktop-app`, lifecycle health, test fixtures, capture scripts and packaging. Preview mode remains non-invasive and unsupported taskbar operations remain unavailable rather than fabricated.
