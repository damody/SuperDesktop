## 1. Stable chevron and snapshot routing

- [x] 1.1 Reserve and render the notification up-chevron unconditionally with localized Button semantics and theme text color.
- [x] 1.2 Forward one complete `accessible_nodes` snapshot for both pointer and keyboard activation without placement filtering.
- [x] 1.3 Add focused taskbar tests for zero, all-visible, mixed-placement, reserved-width, theme, and input-parity contracts.

## 2. Popup admission and empty state

- [x] 2.1 Remove the composition empty-list rejection while preserving singleton toggle, geometry, dismissal, and typed actions.
- [x] 2.2 Render a localized accessible empty state for zero nodes and preserve the non-empty six-column grid.
- [x] 2.3 Add focused popup/composition tests for empty admission, complete node sets, actions, and Explorer-free ownership.

## 3. Validation and evidence

- [x] 3.1 Run focused tests, locked compilation, formatting, and warnings-as-errors clippy; save hashed quality evidence.
- [x] 3.2 Run headful empty and populated show-all scenarios in light/dark/high contrast; visually review and hash reports/screenshots.
- [x] 3.3 Run strict OpenSpec validation, map every scenario to current evidence, and confirm no failed, blocked, stale, P0, or P1 item.
