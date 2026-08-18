## Why

SuperDesktop already renders an owned GPUI Start surface, but the taskbar still invokes the system Start host outside Shell or verification-owned modes. That delegation violates the Explorer-free objective and makes preview behavior differ from the eventual Shell behavior.

## What Changes

- **BREAKING:** Remove product invocation of the Explorer/ExplorerPatcher Start host from the taskbar Start action.
- Make preview, Shell and verification modes toggle the same owned `StartView` surface.
- Preserve owned Search, Pinned, Recommended, All apps, Settings, Account and confirmed Power behavior.
- Add source and headful gates proving Start remains usable without system Start-host invocation.
- Reverify desktop marquee selection beside the exclusive Start behavior so the two required shell surfaces remain independently interactive.

## Capabilities

### New Capabilities

- `owned-start-exclusive`: Defines exclusive GPUI Start ownership, mode parity, activation, placement, input, accessibility and no-system-host delegation.

### Modified Capabilities

None.

## Impact

- Changes `crates/superdesktop-app/src/surface_runtime.rs` Start composition and removes its product dependency on the Start-host invocation adapter.
- Exercises `crates/taskbar-ui/src/start.rs`, existing search/provider contracts, settings persistence and power confirmation.
- Adds source guards, model tests, headful Start and desktop-marquee evidence, and installer refresh evidence.
- Does not yet implement real system-status, IME, legacy notification-area compatibility or Explorer termination; those remain follow-up changes defined by the approved Explorer-free design.
