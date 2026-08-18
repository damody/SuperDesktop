# Complete Windows Taskbar Visual States

## 1. Contracts and Pure Reducers

### 1.1 Add bounded taskbar-state contracts

**目的：** Represent real progress, attention and authoritative generations without platform handles.
**輸入：** Approved design, existing provider conventions and task overlay model.
**產出：** Protocol DTOs, validation and serialization tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-ISOLATION`, `G-TRACE`; protocol test record.
**完成門檻：** Every field/count/deadline is bounded and invalid or stale values reject deterministically.

- [x] 1.1.1 Add window identity, host generation and snapshot generation DTOs.
- [x] 1.1.2 Add none/indeterminate/normal/paused/error progress DTOs with checked values.
- [x] 1.1.3 Add attention cadence/count/foreground-stop and terminal DTOs.
- [x] 1.1.4 Add malformed, zero-total, overflow, stale and serialization tests.

### 1.2 Implement deterministic progress and attention reducers

**目的：** Resolve independent per-window and grouped visual states without GPUI mutation.
**輸入：** 1.1 contracts and existing group identities.
**產出：** Pure reducers and state-transition tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-PROGRESS`, `G-TASKBAR-ATTENTION`; reducer tests.
**完成門檻：** Priority, least progress, flashing termination and state independence match every normative scenario.

- [x] 1.2.1 Implement checked completed/total to bounded permille conversion.
- [x] 1.2.2 Implement error/paused/normal/indeterminate group priority.
- [x] 1.2.3 Select least complete determinate progress for equal-priority groups.
- [x] 1.2.4 Implement finite and timer-no-foreground attention state transitions.
- [x] 1.2.5 Add activation, close, restart, HWND-reuse and independent-field tests.

## 2. Windows Observation and Compatibility

### 2.1 Observe Shell Hook attention behavior

**目的：** Convert documented Windows attention signals into owned generation-bound events.
**輸入：** Registered Shell Hook, validated task HWND identities and system timing APIs.
**產出：** `platform-win` attention adapter and fixtures.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-ATTENTION`, `G-TASKBAR-ISOLATION`; callback traces.
**完成門檻：** Live start/stop/foreground behavior is no-unwind, same-session and timing-bounded.

- [ ] 2.1.1 Decode supported Shell Hook flash/attention and foreground events.
- [ ] 2.1.2 Read the Windows default cadence when request timeout is zero.
- [ ] 2.1.3 Fence PID/session/HWND generation and callback shutdown.
- [ ] 2.1.4 Add wrong-session, retired-window, panic and event-storm tests.

### 2.2 Probe ordinary `ITaskbarList3` progress compatibility

**目的：** Establish the real Explorer-free transport used by unchanged Windows applications.
**輸入：** Controlled ordinary application and committed Shell fixture.
**產出：** Capability probe, raw trace and binding decision.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-PROGRESS`, `G-SHELL-NONINTERFERENCE`; capability report.
**完成門檻：** The selected route observes normal Windows calls without app modification and preview owns no compatibility identity.

- [ ] 2.2.1 Build a controlled unchanged `ITaskbarList3` progress fixture.
- [ ] 2.2.2 Trace `CLSID_TaskbarList` state/value behavior with Explorer present.
- [ ] 2.2.3 Trace the same behavior in controlled Explorer-free Shell mode.
- [ ] 2.2.4 Record and validate the documented route or admitted proxy decision.

### 2.3 Implement isolated progress ingress

**目的：** Publish bounded progress snapshots through the evidence-selected compatibility path.
**輸入：** 2.2 binding decision and 1.1 contracts.
**產出：** Isolated provider/proxy, coalescing queue and process tests.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-PROGRESS`, `G-TASKBAR-ISOLATION`; host test record.
**完成門檻：** Ordinary calls yield exactly-once current-generation state; crash, overflow and malformed input fail closed.

- [ ] 2.3.1 Implement same-session progress state/value ingress.
- [ ] 2.3.2 Implement NOPROGRESS, indeterminate and blocking paused/error semantics.
- [ ] 2.3.3 Add bounded modify coalescing with protected clear/terminal events.
- [ ] 2.3.4 Add no-unwind shutdown and authoritative reconciliation.
- [ ] 2.3.5 Add duplicate, stale, zero-total, overflow, crash and restart tests.

## 3. Supervision and Composition

### 3.1 Add taskbar-state client and restart recovery

**目的：** Consume real overlays without stale resurrection or task-switching failure.
**輸入：** Wave 2/3 providers and existing adjacent-binary client patterns.
**產出：** Supervised client, reconciler and integration tests.
**依賴：** 2.1, 2.3.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TASKBAR-ISOLATION`, `G-TRACE`; client/restart record.
**完成門檻：** Initial/full snapshots apply monotonically; crash clears overlays and bounded restart accepts one new generation.

- [ ] 3.1.1 Add adjacent provider resolution, handshake, health and clean shutdown.
- [ ] 3.1.2 Apply monotonic snapshots and reject pre-restart events.
- [ ] 3.1.3 Reconcile overlays against live PID/session/HWND generations.
- [ ] 3.1.4 Add crash, stale snapshot, duplicate terminal and bounded-retry tests.

### 3.2 Integrate independent task overlays

**目的：** Feed progress/attention into every task and group without clearing other fields.
**輸入：** 3.1 snapshots and current task/group refresh loop.
**產出：** Product composition and mode-independent tests.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TASKBAR-PROGRESS`, `G-TASKBAR-ATTENTION`; integration record.
**完成門檻：** Active/minimized/grouped/progress/attention/badge/availability update independently in preview and Shell modes.

- [ ] 3.2.1 Map per-window snapshots to stable task identities.
- [ ] 3.2.2 Reduce grouped progress and attention using pure priority rules.
- [ ] 3.2.3 Schedule animation ticks only while live animated states exist.
- [ ] 3.2.4 Clear animations on activation, close, retirement and provider failure.
- [ ] 3.2.5 Add mode, group reorder, provider failure and state-independence tests.

## 4. Windows 11 Rendering and Accessibility

### 4.1 Render exact running indicator geometry

**目的：** Replace arbitrary full-width borders with Windows 11 running indicators.
**輸入：** Pure overlay presentation and host DPI/theme state.
**產出：** GPUI indicator components and geometry matrix tests.
**依賴：** 3.2.
**Owner／Wave：** Primary agent / wave 5.
**Gate／Evidence：** `G-TASKBAR-INDICATOR`, `G-TASKBAR-A11Y`; render tests.
**完成門檻：** Every running task has correct inactive/active/grouped/minimized/unavailable geometry at 100–200% DPI.

- [x] 4.1.1 Add pure `TaskVisualState` presentation model.
- [x] 4.1.2 Render centered 6px inactive and 16px active 3px indicators.
- [x] 4.1.3 Render grouped second layer and minimized/unavailable variants.
- [x] 4.1.4 Add DPI, high-contrast and grouped geometry tests.

### 4.2 Render progress and attention layers

**目的：** Match Windows progress colors, fill behavior and bounded flashing.
**輸入：** 4.1 component and live animation phase.
**產出：** GPUI progress/attention presentation and accessibility states.
**依賴：** 4.1.
**Owner／Wave：** Primary agent / wave 5.
**Gate／Evidence：** `G-TASKBAR-PROGRESS`, `G-TASKBAR-ATTENTION`, `G-TASKBAR-A11Y`; visual tests.
**完成門檻：** All colors/fractions/animation/steady states preserve icon, label and indicator and expose exact UIA state.

- [x] 4.2.1 Render green/yellow/red determinate background fills.
- [x] 4.2.2 Render bounded indeterminate moving segment and reduced-motion state.
- [x] 4.2.3 Render amber flash phase and steady post-flash attention state.
- [x] 4.2.4 Add exact percentage, kind and attention UIA names/states.
- [x] 4.2.5 Add coexistence, priority, animation-stop and visual source-contract tests.

## 5. Verification and Packaging

### 5.1 Run Windows 11 parity gates

**目的：** Prove real ordinary-app behavior, fidelity, isolation and resources.
**輸入：** Integrated release binaries and controlled progress/flash fixtures.
**產出：** Automated logs, screenshots, UIA/timing/resource traces and evidence index.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / wave 6.
**Gate／Evidence：** All change gates; `evidence/evidence-index.json`.
**完成門檻：** Every leaf has unique passing evidence and strict validation succeeds on Windows 11 build 26200.

- [ ] 5.1.1 Run fmt, locked/offline workspace check/tests and clippy warnings-as-errors.
- [ ] 5.1.2 Build release product/provider and controlled fixture binaries with hashes.
- [ ] 5.1.3 Capture active/inactive/grouped/minimized indicators across host DPI.
- [ ] 5.1.4 Capture normal/paused/error/indeterminate progress from ordinary API calls.
- [ ] 5.1.5 Capture finite/timer-no-foreground flash cadence and termination.
- [ ] 5.1.6 Capture high-contrast, reduced-motion, UIA and resource/restart evidence.
- [ ] 5.1.7 Prove Explorer-present non-interference and Explorer-free behavior.
- [ ] 5.1.8 Create unique task-linked evidence and pass strict validation.

### 5.2 Package the taskbar-state provider

**目的：** Ship any new provider/compatibility binary in standalone and combined installers.
**輸入：** Gate-passing release revision and NSIS manifests.
**產出：** Hashed installers and uninstall cleanup proof.
**依賴：** 5.1.
**Owner／Wave：** Primary agent / wave 7.
**Gate／Evidence：** `G-TASKBAR-ISOLATION`, `G-TRACE`; packaging record.
**完成門檻：** Both installers contain required binaries, build without launch and remove all new artifacts.

- [ ] 5.2.1 Update release/package/NSIS manifests and uninstall cleanup if a provider is added.
- [ ] 5.2.2 Build and hash standalone and combined installers without launching them.
