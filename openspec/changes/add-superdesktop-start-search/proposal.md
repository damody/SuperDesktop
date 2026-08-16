## Why

The current Start affordance does not provide the application launch and search workflows expected from a Windows desktop shell.

## What Changes

- Add a GPUI-owned Start surface with pinned, recent, all-apps, power, and settings sections.
- Add cancellable app, file, and setting search providers with ranking and streaming results.
- Add keyboard navigation, IME-safe input, accessibility semantics, and deterministic fallback states.

## Capabilities

### New Capabilities

- `start-search`: Start surface, provider aggregation, ranking, activation, accessibility, and performance behavior.

### Modified Capabilities

None.

## Impact

- Affects taskbar UI, GPUI focus/input, provider host, app discovery, file search, settings launch, localization, tests, and evidence.
