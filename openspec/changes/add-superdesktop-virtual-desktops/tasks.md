# Add SuperDesktop Virtual Desktops

## 1. Documented platform capability

### 1.1 Implement `IVirtualDesktopManager` boundary

**目的：** Provide safe documented window membership and move operations.
**輸入：** Live HWND admission, COM manager, owned desktop IDs.
**產出：** Capability probe, current-membership query, desktop-ID query, and move effect.
**依賴：** M0 window tracker and platform FFI boundary.
**Owner／Wave：** Primary integrator / Wave 7.
**Gate／Evidence：** `G-VIRTUAL-DESKTOP`, `G-SAFETY`; platform tests.
**完成門檻：** Invalid/retired HWND and COM failures are typed, and UI receives no COM/native pointers.

- [x] 1.1.1 Add documented manager creation and capability probe.
- [x] 1.1.2 Add live-window current desktop/ID queries and admitted move.
- [x] 1.1.3 Add invalid, retired, unavailable, and round-trip contract tests.

### 1.2 Define capability and snapshot contracts

**目的：** Separate documented and optional features and suppress stale state.
**輸入：** Platform dispositions and shared virtual desktop DTO.
**產出：** Capability set, generation snapshot, membership and typed effects.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 7.
**Gate／Evidence：** `G-VIRTUAL-DESKTOP`, `G-TRACE`; model tests.
**完成門檻：** Every effect is capability-checked and stale generations cannot mutate state.

- [x] 1.2.1 Add separate query/move/enumerate/switch/create/remove/rename capabilities.
- [x] 1.2.2 Add owned snapshots, stable ordering, membership and stale filtering.

## 2. Task View and verification

### 2.1 Add Task View model

**目的：** Expose supported desktop/window workflows with truthful unavailable states.
**輸入：** 1.2 snapshots/capabilities and taskbar tracked windows.
**產出：** Task View open/focus/navigation/actions/accessibility model.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / Wave 7.
**Gate／Evidence：** `G-VIRTUAL-DESKTOP`, `G-A11Y-I18N`; interaction tests.
**完成門檻：** All input routes match, unsupported actions emit no effect, and focus survives reconciliation.

- [x] 2.1.1 Add desktop cards, window membership, focus, dismissal, and accessibility nodes.
- [x] 2.1.2 Add capability-gated switch/create/remove/rename/move effects.
- [x] 2.1.3 Add partial-capability and stale-window end-to-end tests.

### 2.2 Verify integration

**目的：** Prove capability truthfulness, safety, and workspace compatibility.
**輸入：** Platform and Task View fixtures.
**產出：** Verification evidence and completed change.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / Wave 7 exit.
**Gate／Evidence：** `G-VIRTUAL-DESKTOP`, `G-TRACE`; cargo/OpenSpec logs.
**完成門檻：** Rust checks, strict spec, task validation, and evidence pass.

- [x] 2.2.1 Add contract evidence for documented and unavailable optional capabilities.
- [x] 2.2.2 Run fmt, offline check, clippy, tests, strict OpenSpec, and task validation.
