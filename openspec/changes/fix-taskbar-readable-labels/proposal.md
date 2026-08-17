## Why

The production taskbar substitutes the first character of each window title when `show_labels=false`, even though the task model supplies no real icon. This makes the visible taskbar unreadable at the current 175% DPI profile and affects both English and CJK titles.

## What Changes

- Default new and partially specified taskbar settings to readable labels.
- Replace the first-character pseudo-icon fallback with a truthful full-label fallback whenever no real icon is available.
- Render labels in a shrinkable, single-line, ellipsis-aware child so badges and narrow task slots do not collapse text unpredictably.
- Add settings, label-policy, Unicode/grouping, source-contract, and headful regression verification.
- Preserve task accessibility names, actions, ordering, grouping, AppBar geometry, and stored schema compatibility.

## Capabilities

### New Capabilities

- `taskbar-readable-labels`: Defines readable fallback labels, settings defaults, grouping suffixes, and bounded ellipsis behavior for task buttons without real icons.

### Modified Capabilities

None.

## Impact

- Changes `crates/settings-store/src/schema.rs` and `crates/taskbar-ui/src/view.rs` plus focused tests/evidence.
- Existing explicit `show_labels=false` remains parseable, but it no longer produces first-character placeholders when no icon exists.
- No external system mutation, schema version bump, or new dependency is introduced.
