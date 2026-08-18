# System Status and IME Host Implementation Tasks

## 1. Contracts and Guards

### 1.1 Add versioned bounded status DTOs

**目的：** Define the platform/host/app contract without GPUI or native handles.
**輸入：** Approved design and existing provider envelope conventions.
**產出：** `system_status` protocol module and serialization tests.
**依賴：** Exclusive owned Start change complete.
**Owner／Wave：** Primary agent / wave 1 contract owner.
**Gate／Evidence：** `G-STATUS-ISOLATION`, `G-TRACE`; automated evidence index.
**完成門檻：** Every frame/field has bounds, generations and deterministic round-trip/negative tests.

- [x] 1.1.1 Add provider availability, health and generation DTOs.
- [x] 1.1.2 Add network, audio, power and clock/calendar snapshot DTOs.
- [x] 1.1.3 Add stable input-profile identity/list/active-state DTOs.
- [x] 1.1.4 Add set-volume, toggle-mute and activate-profile command/terminal DTOs.
- [x] 1.1.5 Add size, count, deadline, duplicate and malformed serialization tests.

### 1.2 Add no-fixed-status source guards

**目的：** Prevent product taskbar composition from substituting fixture values.
**輸入：** Current `status()` implementation and protocol contract.
**產出：** Product source guard with negative fixtures.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-STATUS-REAL`; source-guard record.
**完成門檻：** Guard rejects fixed online/40/unmuted/zh-TW product status and allows deterministic test fixtures.

- [x] 1.2.1 Add a production-source scanner for fixed system-status values.
- [x] 1.2.2 Add negative fixtures for fixed network/audio/input substitutions.
- [x] 1.2.3 Add a contract test requiring each product provider to expose available or unavailable state.

## 2. Windows Platform Adapters

### 2.1 Implement input-profile observation and activation

**目的：** Provide owned documented Windows input profiles and observed switching.
**輸入：** 1.1 DTOs, TSF/keyboard-layout APIs and interactive-session identity.
**產出：** `platform-win` input-profile adapter and fixtures.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 2 platform owner.
**Gate／Evidence：** `G-IME-SWITCH`, `G-STATUS-ISOLATION`; adapter tests and live probe.
**完成門檻：** Bounded profiles enumerate, active identity is real, switching is observed and invalid/stale targets fail closed.

- [x] 2.1.1 Enumerate installed input profiles with stable identity, language tag and display name.
- [x] 2.1.2 Observe the foreground/current active input profile through an owned snapshot.
- [x] 2.1.3 Activate a validated profile and wait for exact observed confirmation.
- [x] 2.1.4 Add missing-profile, wrong-session, timeout, stale-generation and callback-panic tests.

### 2.2 Implement audio, network, power and clock providers

**目的：** Replace every fixed core status with documented Windows observations/commands.
**輸入：** 1.1 DTOs and Windows provider APIs.
**產出：** Owned `platform-win` status adapters with independent availability.
**依賴：** 1.1; parallel with 2.1.
**Owner／Wave：** Primary agent / wave 2 platform owner.
**Gate／Evidence：** `G-STATUS-REAL`, `G-STATUS-ISOLATION`; platform tests and live snapshot.
**完成門檻：** Real values match Windows observations; provider failure is independent; commands require confirming observation.

- [x] 2.2.1 Read default render endpoint volume/mute and apply validated commands.
- [x] 2.2.2 Observe Network List Manager connectivity with truthful unavailable handling.
- [x] 2.2.3 Read AC/battery state and distinguish valid battery-not-present from provider failure.
- [x] 2.2.4 Extend real clock/calendar locale metadata without fixed date/time values.
- [x] 2.2.5 Add external-change, endpoint-loss, service-failure, command-race and no-battery tests.

## 3. Isolated Host and Client

### 3.1 Add system-status-host process

**目的：** Own native subscriptions and bounded reconciliation outside GPUI.
**輸入：** 1.1 protocol and wave 2 adapters.
**產出：** New workspace crate/library/binary with host-process tests.
**依賴：** 2.1 and 2.2.
**Owner／Wave：** Primary agent / wave 3 host owner.
**Gate／Evidence：** `G-STATUS-ISOLATION`; process tests and resource snapshots.
**完成門檻：** Handshake, initial/full snapshots, commands, callback coalescing, overflow and teardown are deterministic and bounded.

- [x] 3.1.1 Add the `system-status-host` workspace crate and binary entry point.
- [x] 3.1.2 Implement versioned handshake, client/session lease and initial full snapshot.
- [x] 3.1.3 Register provider callbacks with no-unwind/shutdown fencing.
- [x] 3.1.4 Implement bounded coalescing queue, overflow event and authoritative reconciliation timer.
- [x] 3.1.5 Implement exactly-once status command terminals and host-process malformed-input tests.

### 3.2 Add SuperDesktop host client and restart recovery

**目的：** Consume host snapshots without stale-state resurrection.
**輸入：** 3.1 host binary and existing provider client patterns.
**產出：** `superdesktop-app` status client and supervised restart state machine.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 3 integration owner.
**Gate／Evidence：** `G-STATUS-ISOLATION`; client/restart tests.
**完成門檻：** Crash clears state, restart accepts one new full generation and late pre-restart data is rejected.

- [x] 3.2.1 Add adjacent-binary resolution, process launch, handshake and clean shutdown.
- [x] 3.2.2 Apply only monotonic full/incremental snapshot generations.
- [x] 3.2.3 Route commands with deadlines, cancellation and exactly-one terminal result.
- [x] 3.2.4 Add crash, restart, stale snapshot, duplicate terminal and bounded-retry tests.

## 4. Taskbar Status and Owned Flyouts

### 4.1 Replace fixture status models

**目的：** Render the host's real independently available state.
**輸入：** 3.2 client snapshots and current `StatusRegion`.
**產出：** Revised taskbar status model/accessibility nodes.
**依賴：** 3.2.
**Owner／Wave：** Primary agent / wave 4 UI owner.
**Gate／Evidence：** `G-STATUS-REAL`, `G-STATUS-A11Y`; model/view tests.
**完成門檻：** No product fixed values remain; each provider state updates independently with stable control order.

- [x] 4.1.1 Replace CoreStatus fixture construction with snapshot conversion.
- [x] 4.1.2 Render real input label, volume/mute, network, power and clock/calendar states.
- [x] 4.1.3 Render localized independent unavailable/not-present states without fake actions.
- [x] 4.1.4 Add stable ordering, accessible name/state/action and stale-update tests.

### 4.2 Add owned GPUI system flyouts

**目的：** Provide Explorer-independent status interaction surfaces.
**輸入：** 4.1 models and typed command callbacks.
**產出：** Input, volume, network/power and calendar GPUI views.
**依賴：** 4.1.
**Owner／Wave：** Primary agent / wave 4 UI owner.
**Gate／Evidence：** `G-IME-SWITCH`, `G-STATUS-A11Y`; UI tests and headful captures.
**完成門檻：** One flyout at a time, pointer/keyboard/UIA actions work, Escape/outside dismissal returns focus and unavailable controls are safe.

- [x] 4.2.1 Add shared single-system-flyout window/toggle/dismissal coordination.
- [x] 4.2.2 Add input-profile list with active mark and typed switch actions.
- [x] 4.2.3 Add volume slider/mute state and typed audio actions.
- [x] 4.2.4 Add truthful network/power summary and clock/calendar views.
- [x] 4.2.5 Add pointer, keyboard, UIA, focus-return, unavailable and rapid-switch tests.

### 4.3 Integrate snapshots, commands and Start IME stability

**目的：** Wire live host state to taskbar/flyouts without disrupting owned Start.
**輸入：** 3.2 client, 4.1/4.2 views and owned Start composition.
**產出：** SuperDesktop composition and integration tests.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / wave 4 integrator.
**Gate／Evidence：** `G-STATUS-REAL`, `G-IME-SWITCH`, `G-STATUS-A11Y`.
**完成門檻：** Live ticks reconcile, commands confirm, broker failure is truthful and Start IME composition/focus survives a profile switch.

- [x] 4.3.1 Start/supervise the host and apply initial/live status snapshots.
- [x] 4.3.2 Route input/audio flyout actions and observed terminal state back to views.
- [x] 4.3.3 Preserve Start IME composition and focus during taskbar profile changes.
- [x] 4.3.4 Add end-to-end provider unavailable/restart and mode-independent integration tests.

## 5. Verification and Packaging

### 5.1 Run real-status automated and headful gates

**目的：** Prove real host state and interaction on the reference Windows session.
**輸入：** Integrated release binaries and controlled provider fixtures.
**產出：** Quality logs, live snapshots, UIA traces, screenshots and evidence index.
**依賴：** 4.3.
**Owner／Wave：** Primary agent / wave 5 verification owner.
**Gate／Evidence：** All change gates; `evidence/evidence-index.json`.
**完成門檻：** Complete workspace gates pass; headful input/audio/status values match Windows; every leaf has unique evidence and strict validation passes.

- [ ] 5.1.1 Run fmt, complete locked/offline workspace check/tests and clippy warnings-as-errors.
- [ ] 5.1.2 Build release binaries and record all product/host hashes.
- [ ] 5.1.3 Capture real taskbar status and owned input/volume/calendar flyouts at host DPI.
- [ ] 5.1.4 Switch a controlled real input profile and verify Start IME/focus stability.
- [ ] 5.1.5 Record resource/restart traces, create unique task-linked evidence and pass strict validation.

### 5.2 Package the host

**目的：** Install the status host with standalone and combined products.
**輸入：** Gate-passing revision and UTF-8 NSIS packaging scripts.
**產出：** Updated package manifests and hashed installers.
**依賴：** 5.1.
**Owner／Wave：** Primary agent / wave 6 packaging owner.
**Gate／Evidence：** `G-STATUS-ISOLATION`, `G-TRACE`; packaging record.
**完成門檻：** Both installers contain `system-status-host.exe`, build without launch and pass exact submodule admission.

- [x] 5.2.1 Add `system-status-host.exe` to release/package/NSIS manifests and uninstall cleanup.
- [ ] 5.2.2 Build and hash standalone and combined installers without launching them.
