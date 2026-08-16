# Add SuperDesktop Notification Area Host

## 1. Protocol and isolated registry

### 1.1 Add notification lifecycle contracts

**目的：** Define owned, bounded icon/client/event messages and snapshots.
**輸入：** Existing notification DTO, provider limits, stable identity rules.
**產出：** Add/modify/delete/focus/disconnect/snapshot/event/health contracts.
**依賴：** `extend-superdesktop-shell-contracts`.
**Owner／Wave：** Primary integrator / Wave 6.
**Gate／Evidence：** `G-NOTIFICATION-AREA`, `G-SAFETY`; protocol tests.
**完成門檻：** Malformed icons, stale generations, oversize text/data, and duplicate identity fail closed.

- [x] 1.1.1 Add notification client, icon key, mutation, snapshot, event, and terminal DTOs.
- [x] 1.1.2 Add size/cardinality/generation validation and negative fixtures.
- [x] 1.1.3 Add bounded protected-event queue and coalescing rules.

### 1.2 Implement dedicated host

**目的：** Isolate notification registry and event delivery from GPUI.
**輸入：** 1.1 contracts and provider-host framing conventions.
**產出：** `notification-area-host` library/binary, registry, leases, snapshots, health.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 6.
**Gate／Evidence：** `G-NOTIFICATION-AREA`, `G-SAFETY`; host process tests.
**完成門檻：** Host handles lifecycle, capacity, disconnect cleanup, EOF shutdown, and restart snapshots deterministically.

- [x] 1.2.1 Add host crate, bounded framing, handshake, and registry.
- [x] 1.2.2 Add generation filtering, client cleanup, restart snapshot, and health behavior.
- [x] 1.2.3 Add process integration tests for lifecycle, malformed input, crash/EOF, and capacity.

## 2. Taskbar notification surface

### 2.1 Add visible/overflow model and interactions

**目的：** Render truthful notification state with complete input/accessibility routes.
**輸入：** Host snapshots/deltas, DPI, user visibility preferences.
**產出：** Visible and overflow models, focus/navigation, tooltips, typed client events.
**依賴：** 1.2 and taskbar status region.
**Owner／Wave：** Primary integrator / Wave 6.
**Gate／Evidence：** `G-NOTIFICATION-AREA`, `G-A11Y-I18N`; model tests.
**完成門檻：** Ordering is stable, all input routes match, and unavailable state renders no fake icons.

- [x] 2.1.1 Add snapshot/delta reconciliation and visible/overflow placement.
- [x] 2.1.2 Add pointer, keyboard, accessibility, tooltip, activation, and context event routing.

### 2.2 Verify latency and integration

**目的：** Prove isolation, correctness, resource bounds, and latency.
**輸入：** Host/taskbar fixtures and deterministic timestamps.
**產出：** Latency/resource evidence and completed change.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / Wave 6 exit.
**Gate／Evidence：** `G-NOTIFICATION-AREA`, `G-PERF`, `G-TRACE`; cargo/OpenSpec logs.
**完成門檻：** p95 is under 100 ms, stress remains bounded, and all validation pass.

- [x] 2.2.1 Add end-to-end lifecycle/restart/input/latency/resource tests.
- [x] 2.2.2 Run fmt, offline check, clippy, tests, strict OpenSpec, and task validation.
