# Add SuperDesktop Taskbar Advanced Interactions

## 1. Preview, flyout, and overlays

### 1.1 Add native preview capability boundary

**目的：** Probe and request safe window previews without trusting stale HWND state.
**輸入：** Window tracker identities, DWM composition state, preview budget.
**產出：** Preview capability/result types and revalidation helpers.
**依賴：** M0 taskbar/window tracking.
**Owner／Wave：** Primary integrator / Wave 5.
**Gate／Evidence：** `G-TASKBAR-ADVANCED`, `G-SAFETY`, `G-PERF`; platform tests.
**完成門檻：** Invalid/retired windows fail closed and supported previews meet the 500 ms admission budget.

- [x] 1.1.1 Add DWM composition and live-window preview capability probe.
- [x] 1.1.2 Add typed preview request/result with 500 ms deadline and fallback.
- [x] 1.1.3 Add invalid, retired, unavailable, and supported capability tests.

### 1.2 Add grouped flyout and overlay models

**目的：** Provide Windows-like grouped window selection and independent task states.
**輸入：** Task groups, tracked windows, preview results, task progress/attention events.
**產出：** Flyout focus/navigation/actions, preview cards, progress and attention state.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 5.
**Gate／Evidence：** `G-TASKBAR-ADVANCED`, `G-A11Y-I18N`; model tests.
**完成門檻：** Pointer/keyboard/accessibility routes match and stale windows cannot receive actions.

- [x] 1.2.1 Add hover-delay preview cards and multi-window flyout reconciliation.
- [x] 1.2.2 Add activate, close, focus, escape, and accessibility effects.
- [x] 1.2.3 Add independent progress, paused/error, badge, and attention overlays.

## 2. Jump Lists and persistence

### 2.1 Add sanitized Jump Lists

**目的：** Expose recent/frequent destinations and tasks through safe descriptors.
**輸入：** Provider command descriptors, application identity, local pin/close actions.
**產出：** Bounded grouped Jump List model and typed invocation.
**依賴：** Provider contracts and context-menu sanitization.
**Owner／Wave：** Primary integrator / Wave 5.
**Gate／Evidence：** `G-TASKBAR-ADVANCED`, `G-SAFETY`; sanitization tests.
**完成門檻：** Lists are capped/deduplicated and disabled/stale commands cannot invoke.

- [x] 2.1.1 Add recent/frequent/task groups, limits, deduplication, and local commands.
- [x] 2.1.2 Add Jump List focus/submenu/accessibility and invocation tests.

### 2.2 Persist preferences and verify

**目的：** Preserve user choices and prove the advanced interaction contract.
**輸入：** Pin order, grouping/label/preview/multi-monitor preferences, available apps.
**產出：** Versioned snapshot, reconciliation, verification evidence.
**依賴：** 1.2 and 2.1.
**Owner／Wave：** Primary integrator / Wave 5 exit.
**Gate／Evidence：** `G-TASKBAR-ADVANCED`, `G-TRACE`; Rust/OpenSpec logs.
**完成門檻：** Snapshot round-trip/reconciliation and all validation pass.

- [x] 2.2.1 Add versioned preference snapshot and pin-order reconciliation.
- [x] 2.2.2 Add end-to-end flyout/preview/Jump List/overlay/persistence tests.
- [x] 2.2.3 Run fmt, offline check, clippy, tests, strict OpenSpec, and task validation.
