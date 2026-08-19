## Why

The owned SuperExplorer taskbar entry stays at 160 DIP while crowded labeled task buttons shrink, producing an uneven Windows 11 button rhythm and mismatched underline. UTIT currently checks only that the fixed entry exists and therefore cannot detect this geometry regression.

## What Changes

- Include the fixed SuperExplorer entry in the adaptive task-slot width calculation.
- Use the shared adaptive width for its hit target, label, and long running indicator while preserving its independent command route.
- Extend live UTIT evidence with ordered task measurements, fixed-entry bounds, logical widths, and strict fixed/task parity and right-boundary assertions.
- Replace a self-referential source-contract assertion with positive current markers and negative obsolete-geometry checks.
- Preserve an Explorer-independent product path and do not archive the change.

## Capabilities

### New Capabilities

- `unified-taskbar-button-geometry`: Defines adaptive width and measurement parity for the fixed entry and ordinary labeled task buttons.

### Modified Capabilities

None.

## Impact

Affected areas are `crates/taskbar-ui/src/view.rs`, the live taskbar capture script, UTIT shell-parity evidence, and associated unit/source-contract tests. No public API, dependency, installer, task grouping, or command-routing change is introduced.
