# Owned Notification Center Tasks

## 1. Additive notification contracts and native ingress

### 1.1 Define backward-compatible notification DTOs

**目的：** Freeze bounded record, snapshot, and typed mutation contracts.
**輸入：** Approved design, current notification protocol, serde compatibility fixtures.
**產出：** Additive DTOs/defaults, validation and round-trip tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-NOTIFICATION-PROTOCOL`; automated protocol report.
**完成門檻：** Old and new payloads round-trip; every invalid bound fails before mutation.

- [x] 1.1.1 Add owned notification identity, severity, content, time, and generation DTOs.
- [x] 1.1.2 Add defaulted icon/snapshot notification fields and dismiss/clear mutations.
- [x] 1.1.3 Add old-payload, new-payload, oversized-text, invalid-identity, and frame-bound tests.

### 1.2 Copy documented NotifyIcon balloon data

**目的：** Preserve `NIF_INFO` content without borrowed native state.
**輸入：** DTOs, current layout matrix and `WM_COPYDATA` decoder.
**產出：** Bounded copied balloon input and translation tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-NOTIFICATION-PROTOCOL`, `G-SHELL-NONINTERFERENCE`; native ingress report.
**完成門檻：** Supported layouts copy exact fields; truncated/foreign/stale inputs are zero-effect.

- [x] 1.2.1 Copy bounded info title, body, flags, timeout/version, and realtime fields by `cbSize`.
- [x] 1.2.2 Translate non-empty balloon data into an owned record while preserving icon mutation.
- [x] 1.2.3 Add layout, malformed UTF-16, empty info, truncation, dead-window, and cross-session tests.

## 2. Authoritative bounded host history

### 2.1 Implement history admission and retention

**目的：** Maintain deterministic, bounded notification history.
**輸入：** Owned records and compatibility operations.
**產出：** Registry history, deduplication, eviction, snapshot reconciliation.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-NOTIFICATION-HISTORY`; host unit/integration report.
**完成門檻：** At most 100 newest unique records remain under storms and disconnects.

- [x] 2.1.1 Add notification history storage and atomic icon-plus-notification admission.
- [x] 2.1.2 Add deterministic duplicate suppression, oldest-first eviction, and disconnect retention.
- [x] 2.1.3 Add capacity, storm, deduplication, ordering, disconnect, and snapshot tests.

### 2.2 Implement typed history actions

**目的：** Make dismiss and clear authoritative and race-safe.
**輸入：** Registry history and expected host generation.
**產出：** Dismiss/clear reducers, client calls, stale behavior.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-NOTIFICATION-HISTORY`; action report.
**完成門檻：** Current actions reconcile; stale/unknown actions preserve rows and icons.

- [x] 2.2.1 Add generation-aware single-dismiss and clear-all host mutations.
- [x] 2.2.2 Add SuperDesktop client helpers and authoritative snapshot reconciliation.
- [x] 2.2.3 Add current, stale, unknown, repeated, clear-empty, and icon-preservation tests.

## 3. Windows 11 owned notification-center UI

### 3.1 Build the combined notification/calendar model and geometry

**目的：** Add one clamped popup model without changing calendar/provider authority.
**輸入：** Snapshot history, existing calendar flyout and monitor geometry.
**產出：** Pure view model, tokens, bounded geometry, scroll ownership.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-NOTIFICATION-CENTER-UI`; model/geometry report.
**完成門檻：** Empty through overflow states fit all reference DPI/taskbar-row cases.

- [x] 3.1.1 Add localized notification view rows, empty state, and provider-unavailable state.
- [x] 3.1.2 Add Windows 11 light/dark/high-contrast card, focus, dismiss, and header tokens.
- [x] 3.1.3 Add 100–225% DPI, negative-origin, compact-work-area, overflow, and calendar-reachability tests.

### 3.2 Render and wire accessible typed interactions

**目的：** Expose real history actions through pointer, keyboard, and UIA.
**輸入：** View model, typed client helpers and flyout composition.
**產出：** Combined GPUI surface, callbacks, focus restoration and accessibility tree.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-NOTIFICATION-CENTER-UI`, `G-NOTIFICATION-A11Y`, `G-SHELL-NONINTERFERENCE`; headful/UIA report.
**完成門檻：** Dismiss/clear/Escape work equivalently and never delegate to system UI.

- [x] 3.2.1 Render header, cards, real icons, bounded text, times, empty state, and calendar below.
- [x] 3.2.2 Wire dismiss, Delete, clear-all, Enter/Space, Escape, and clock-focus restoration.
- [x] 3.2.3 Add UIA role/name/value, long-text, disabled-provider, localization, and no-delegation tests.

## 4. Headful admission, traceability, and packaging

### 4.1 Verify the real Explorer-free surface

**目的：** Prove production composition on the Windows 11 175% host.
**輸入：** Release app, controlled notification fixture and capture harness.
**產出：** Empty/populated/overflow/theme screenshots, UIA traces, process report.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-NOTIFICATION-CENTER-UI`, `G-NOTIFICATION-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful/*`.
**完成門檻：** Every surface/action passes with Explorer absent and is restored safely after the gate.

- [x] 4.1.1 Add a controlled NotifyIcon balloon fixture and production headful capture harness.
- [x] 4.1.2 Capture light, dark, high contrast, empty, populated, overflow, dismiss, clear, and 175% bounds.
- [x] 4.1.3 Prove UIA/keyboard parity, focus return, zero forbidden processes, and authoritative reconciliation.

### 4.2 Run full gates and admit packages

**目的：** Produce distributable, traceable artifacts without archive mutation.
**輸入：** Passing automated/headful implementation and clean committed revision.
**產出：** Full gate report, release hashes, both NSIS installers, 24-leaf evidence index.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence index and packaging report.
**完成門檻：** 24/24 unique passed leaves, strict/detailed validation, both installers, unarchived change.

- [x] 4.2.1 Run fmt, locked/offline workspace tests, Clippy warnings-as-errors, and release build.
- [ ] 4.2.2 Commit implementation and build standalone plus combined NSIS installers without launch.
- [ ] 4.2.3 Record revision/binary/package hashes, create 24 unique evidence records, and pass detailed/strict validation.
