## Why

The desktop exposes a rubber-band selection model but no pointer-driven selection rectangle, while the owned Start surface is a flat text list that does not match the current Windows 11 structure or application-icon behavior. These gaps make the shell visibly and interactively incomplete after native icon parity.

## What Changes

- Add empty-space pointer marquee selection with normalized geometry, live hit testing, Ctrl-additive selection, and transient visual feedback.
- Prevent item clicks and drags from incorrectly starting a desktop marquee.
- Rebuild the owned Start surface around Search, Pinned, Recommended, All apps, Account, Settings, and a collapsed Power menu.
- Reuse Shell/BC7 application icons in Start and provide truthful semantic fallbacks.
- Center and clamp Start above the work-area edge and preserve keyboard, IME, UI Automation, persistence, activation, and power confirmation contracts.
- Add automated, headful, accessibility, strict-validation, and installer evidence.

## Capabilities

### New Capabilities

- `desktop-pointer-marquee`: Defines pointer capture, rectangle geometry, hit testing, modifier behavior, visual feedback, and selection completion.
- `windows11-start-surface`: Defines owned Start information architecture, modes, icons, placement, keyboard/accessibility behavior, and safe power actions.

### Modified Capabilities

None.

## Impact

- Affects `desktop-ui`, `taskbar-ui`, and `superdesktop-app` composition/verification scripts.
- Reuses existing settings, SearchResult, Shell icon, BC7, activation, and power boundaries; no registry, persisted-schema, dependency, or external API change.
- Rebuilds standalone and combined NSIS installers without launching them.
