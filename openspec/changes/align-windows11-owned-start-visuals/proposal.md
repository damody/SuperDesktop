## Why

SuperDesktop exclusively owns Start, but the current presentation is English-only and visually resembles a test panel more than Windows 11. This gap is especially visible on the Traditional Chinese 175% DPI reference host.

## What Changes

- Add bounded Traditional Chinese and English Start presentation strings with Windows locale selection and deterministic override.
- Align the search field, section density, pinned/recommended cells, All apps page, footer and Power flyout with Windows 11 visual tokens.
- Add visible hover, focus, pressed and high-contrast states without changing typed actions.
- Prove the aligned Start remains owned by SuperDesktop and package updated binaries.

## Capabilities

### New Capabilities

- `windows11-owned-start-visuals`: Defines localized Windows 11 Start layout, visual states, accessibility and Explorer-free presentation.

### Modified Capabilities

None.

## Impact

Primarily changes `taskbar-ui/src/start.rs`, Start presentation tests, capture evidence and packaged SuperDesktop. Search, activation, persistence, power confirmation and provider protocols remain compatible.
