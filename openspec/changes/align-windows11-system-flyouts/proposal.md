## Why

SuperDesktop already owns real input, audio, network, power, and clock data, but its four system flyouts still use generic cards, fixed English labels, and inconsistent geometry that visibly diverge from Windows 11. The next approved GUI-convergence batch must align those surfaces without delegating to Explorer or pretending unsupported Quick Settings mutations exist.

## What Changes

- Add shared Windows 11 light, dark, and high-contrast chrome for owned system flyouts.
- Align input-language rows, volume control, network/power summaries, and calendar density, typography, interaction states, and unavailable presentation.
- Localize visible Traditional Chinese and English labels while retaining full provider identity in accessibility output.
- Clamp each popup to per-monitor work-area, DPI, and owned-taskbar geometry.
- Preserve typed provider commands, one-open-flyout lifecycle, truthful partial availability, and Explorer-free composition.
- Add deterministic, headful, accessibility, process, traceability, release, and NSIS packaging evidence.

## Capabilities

### New Capabilities

- `windows11-owned-system-flyouts`: Defines owned system-flyout presentation, localization, geometry, accessibility, truthful behavior, and Explorer-free constraints.

### Modified Capabilities

None.

## Impact

Changes `taskbar-ui/src/system_flyout.rs`, taskbar flyout presentation inputs, `superdesktop-app/src/surface_runtime.rs`, tests, capture harnesses, evidence, and packaged SuperDesktop. Status provider DTOs and command semantics remain compatible; no undocumented Windows interface or system-owned visible shell surface is added.
