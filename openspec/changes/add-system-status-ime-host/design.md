## Context

The current taskbar clock uses the real Windows local date/time adapter, but `status()` supplies fixed values for network, volume, mute, input language, battery and notification count. That was truthful only as an early visual fixture and cannot remain in an Explorer-free shell. The approved architecture in `docs/superpowers/specs/2026-08-18-explorer-free-owned-shell-design.md` requires native state outside GPUI and owned flyouts inside GPUI.

This change owns the shared status contracts so the later legacy notification compatibility change consumes, rather than modifies, them.

## Goals / Non-Goals

**Goals:**

- Publish real, independently available Windows network, audio, power, clock and input-profile state.
- Switch input profiles and volume/mute through typed commands confirmed by later snapshots.
- Isolate COM/TSF/Core Audio/NLM callbacks in `system-status-host`.
- Render owned GPUI flyouts with pointer, keyboard and UIA behavior.
- Restart and reconcile the provider without stale state or fake fallback values.

**Non-Goals:**

- Do not implement legacy `Shell_NotifyIcon` intake.
- Do not reproduce private Quick Settings or notification-center history.
- Do not terminate Explorer in this change.
- Do not load third-party code or retain caller-owned native handles.

## Decisions

### 1. Versioned status protocol owned by this change

`shell-provider-protocol` adds a `system_status` module with bounded snapshot, provider-state, input-profile and command DTOs. Every snapshot has a monotonic generation and per-provider availability. Commands have correlation IDs, deadlines and exactly-one terminal result.

This is preferred over sharing taskbar-ui structs because protocol values must not depend on GPUI and must survive host restart.

### 2. Dedicated host process

A new workspace crate `system-status-host` owns COM apartment initialization, TSF/input-language observation, Core Audio endpoint callbacks, Network List Manager observation, power notifications and periodic authoritative reconciliation. It uses the existing newline-delimited bounded process pattern and never shares raw COM, HWND, HKL or callback pointers.

Putting these adapters in `superdesktop-app` would make provider crashes or callback panics fatal to desktop/taskbar, so it is rejected.

### 3. Documented APIs and truthful partial availability

- Input profiles use documented TSF/input-profile and keyboard-layout APIs; stable identities include language/profile data rather than display text.
- Volume and mute use the default render endpoint through Core Audio.
- Network uses Network List Manager connectivity.
- Power uses `GetSystemPowerStatus` and power-setting notifications where available.
- Clock/calendar continue using Windows local time/locale APIs.

Each provider can fail independently. A missing battery on a desktop is a valid `not-present` power state, not an error. A failed COM provider is `unavailable` and does not retain its last value as if current.

### 4. Event-driven snapshots with bounded reconciliation

Native callbacks enqueue owned events through no-unwind boundaries. The host coalesces high-frequency audio/network changes and emits a new snapshot generation. A bounded timer performs authoritative reconciliation to recover missed callbacks. Stale generations are rejected by the app.

### 5. Command completion requires observation

Input-profile activation, set-volume and toggle-mute commands are not successful merely because the API call returned. The host waits for a confirming observed snapshot before emitting success; deadline, cancellation or contradictory observation emits a terminal failure and preserves truthful UI.

### 6. Owned GPUI flyouts

`taskbar-ui` adds separate input-language, volume, network/power and calendar flyout models/views. Only one system flyout is open at a time. Repeated invocation toggles it; Escape and outside dismissal return focus to the originating status control. Flyout actions emit typed commands through `superdesktop-app`.

## Blocking gates

- `G-STATUS-REAL`: product source contains no fixed network/volume/mute/input-language status and live snapshots match documented Windows observations.
- `G-IME-SWITCH`: installed profiles enumerate and a controlled switch is observed exactly once without breaking Start IME composition.
- `G-STATUS-ISOLATION`: malformed frames, callback panic, stale generation, host crash and restart fail closed.
- `G-STATUS-A11Y`: every status/flyout action has stable name, role, state, focus and keyboard behavior.
- `G-TRACE`: unique task evidence, strict validation and package hashes pass.

## Adjustment policy

- **A:** refine tasks, test fixtures, commands or evidence paths without changing contracts or gates.
- **B:** correct a documented-API or host assumption within scope by updating design/spec/tasks and reopening affected evidence.
- **C:** use an undocumented Windows interface, weaken a gate, add privileged/system mutation, broaden external writes or change public scope only after user approval.

## Risks / Trade-offs

- **[TSF profiles differ from HKL list]** → Prefer TSF identities, preserve a documented HKL fallback with explicit capability state and test both paths.
- **[COM callbacks race host shutdown]** → Fence generations, unregister before apartment teardown and catch all FFI panics.
- **[Provider restart briefly removes icons/text]** → Show truthful unavailable state until a full snapshot arrives; never reuse stale values as current.
- **[Volume command races external changes]** → Correlate to observed generations and accept only the requested terminal state.
- **[NLM service unavailable]** → Isolate network failure; clock, audio, power and input remain actionable.

## Migration Plan

1. Land protocol DTOs and negative tests.
2. Add platform adapters and deterministic fixtures.
3. Add `system-status-host`, client and restart/reconciliation tests.
4. Replace hard-coded taskbar state and add owned flyouts.
5. Add headful input/audio/status evidence and installer packaging.

The feature remains unavailable until host handshake succeeds. Rollback removes the host/client wiring and returns providers to truthful unavailable state; it MUST NOT restore hard-coded values.

## Open Questions

None.
