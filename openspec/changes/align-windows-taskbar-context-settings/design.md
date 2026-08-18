## Context

`TaskbarView` renders task buttons and system status, but only task buttons expose right-click callbacks; blank taskbar space has no context route. Application right-click opens `JumpListView`, whose behavior is typed but whose dark rectangular presentation does not match current Windows 11 command surfaces. `TaskbarSettings` already persists rows, pins, grouping, labels, previews, and monitor policy, yet none are available in an owned settings GUI. Explorer and Windows Settings cannot be dependencies because the product target is an Explorer-free session.

The approved source design is `docs/superpowers/specs/2026-08-18-windows-taskbar-context-settings-design.md`. Host platform is Windows 11 build 26200. GPUI remains the product UI framework and `settings-store` remains the only settings persistence owner.

## Goals / Non-Goals

**Goals:**

- Add owned empty-taskbar and application context surfaces with Windows 11 geometry, theming, input, and accessibility.
- Add an owned taskbar settings window that exposes every currently actionable taskbar preference and truthfully disables unsupported Windows inbox surfaces.
- Persist bounded new fields, migrate missing fields, and apply successful saves immediately to all taskbars.
- Keep Preview and Shell composition equivalent and preserve unrelated state on every failure.
- Produce task-linked automated and headful evidence plus standalone/combined installer proof.

**Non-Goals:**

- Calling Explorer menus, `ms-settings:taskbar`, or private Windows settings protocols.
- Claiming Widgets, pen menu, touch keyboard, auto-hide, or notification-center history without an owned implementation.
- Replacing Task Manager or broadening Shell takeover authority.
- Redesigning Start, desktop context menus, or notification provider contracts.

## Decisions

### 1. Separate pure models from GPUI windows

`taskbar-ui` will add `TaskbarContextModel`, `TaskbarSettingsModel`, typed commands/effects, and dedicated `TaskbarContextView`/`TaskbarSettingsView`. Models own selection, expansion, disabled explanations, and validation, but never launch processes or write files. `superdesktop-app` owns singleton windows, focus restoration, geometry, process launch, and settings saves.

This is preferred over embedding menus inside `TaskbarView`, because independent windows match Windows z-order/dismissal behavior and keep the main render tree bounded. Reusing `JumpListView` for all settings was rejected because it cannot express grouped cards and switches without tangling two interaction models.

### 2. Use one typed effect contract

The context menu emits `OpenTaskbarSettings`, `OpenTaskManager`, or `Dismiss`. The settings model emits a complete validated `TaskbarSettings` candidate rather than field-specific filesystem callbacks. Application Jump Lists continue emitting `CommandDescriptor` so provider and local command behavior remains unchanged.

This makes pointer, keyboard, and UIA activation converge before any external mutation and lets composition apply one atomic save path.

### 3. Extend settings with bounded enums

`TaskbarSettings` gains `search_mode: Hidden | Icon | Box`, `show_task_view: bool`, and `alignment: Left | Center`. Missing fields decode to `Hidden`, `true`, and `Left` to preserve the current visible layout; invalid enum values fall back only that field. Existing fields remain unchanged. Serialization remains schema version 1 because the decoder is additive and forward-compatible.

Search icon/box invokes the owned Start search window. Task View is omitted from rendering and hit testing when disabled. Alignment applies to the task control cluster while keeping system status anchored to the monitor edge.

### 4. Truthful Windows settings inventory

The settings page mirrors Windows section names and order. Supported controls are interactive. Widgets, pen menu, touch keyboard ownership, and auto-hide appear disabled with localized unavailable explanations. This is preferred over hiding every unsupported row because visible disabled rows make parity boundaries clear and allow later implementations without changing navigation structure.

### 5. Atomic live update

Composition clones the current `SettingsV1`, replaces its taskbar value with the validated candidate, calls `SettingsStore::save`, and publishes only the returned saved snapshot. Every live `TaskbarView` receives recalculated layout, label/search/task-view/alignment state, and a notification. Failure keeps the previous settings and exposes a bounded accessible error in the settings window.

### 6. Singleton, monitor-clamped window lifecycle

There is at most one generic taskbar context menu, one application Jump List, and one taskbar settings window. Opening a conflicting menu closes the prior one. Context windows are positioned from the pointer/taskbar anchor and clamped to the selected monitor work area. Escape, outside focus loss, successful commands, taskbar teardown, and monitor retirement dismiss them. Focus returns to the originating taskbar when possible.

### 7. Evidence and adjustment policy

Blocking gates are `G-TASKBAR-CONTEXT`, `G-TASKBAR-SETTINGS`, `G-TASKBAR-A11Y`, `G-TASKBAR-PERSISTENCE`, `G-SHELL-NONINTERFERENCE`, and `G-PACKAGE`.

- **A — task refinement:** task split/order/command changes that preserve scope, contracts, gates, and evidence.
- **B — design/spec correction:** an in-scope behavior correction requires affected artifacts/tasks/evidence to be updated and revalidated before work resumes.
- **C — material change:** new authority, external writes, destructive behavior, platform/framework changes, or weaker gates require user approval.

No adjustment may silently lower a blocking gate. Stale evidence remains recorded and dependent completed tasks reopen.

### 8. Match indicator length to the configured task-button presentation

The earlier fixed 6 px/16 px running indicators describe Windows icon-only buttons, but the approved reference uses readable labels and uncombined wide buttons. When labels are visible, the running indicator therefore spans the task button content width minus bounded horizontal insets; active/grouped state may increase thickness or layer count but does not collapse back to icon-only length. When labels are hidden and a real icon exists, the short icon-only indicator remains. Progress and attention reuse the same button geometry.

## Risks / Trade-offs

- **[Risk] GPUI popup windows differ from native MenuHost shadow/composition** → Use fixed geometry/theme matrices and headful comparison; do not claim pixel identity where GPUI lacks backdrop APIs.
- **[Risk] Center alignment conflicts with multi-row task packing** → Center the calculated task cluster as a unit after overflow calculation and add row/DPI/overflow tests.
- **[Risk] Settings save failure leaves controls visually optimistic** → Views render authoritative saved settings plus a separate pending/error state; never commit the candidate locally first.
- **[Risk] Right-click reaches both a task and the background** → Task handlers stop propagation; source-contract and headful tests require exactly one menu.
- **[Risk] Task Manager path substitution** → Resolve the inbox executable from the Windows directory, validate it as a regular non-reparse file, and launch with no shell expansion.
- **[Risk] Long localized strings overflow cards** → Enforce bounded text, wrap supporting text, test Traditional Chinese/English and 100–500% scale.
- **[Risk] A fixed short indicator contradicts labeled reference captures** → Derive indicator width from labeled/icon-only presentation and add exact source/headful geometry evidence for both modes.

## Migration Plan

1. Add settings enums/defaults/decode/encode tests; existing files migrate additively.
2. Add pure UI models and renderers behind composition callbacks.
3. Wire background/task right-click, settings persistence, and live rerender.
4. Run Preview first, then controlled Shell headful verification.
5. Build installers without launch and verify no new binary or uninstall residue.

Rollback is a code rollback: older builds ignore the additive JSON fields while preserving unknown top-level data. No registry or external migration is required.

## Open Questions

None. Unsupported inbox surfaces are explicitly disabled, and auto-hide remains outside this change until its AppBar design is independently verified.
