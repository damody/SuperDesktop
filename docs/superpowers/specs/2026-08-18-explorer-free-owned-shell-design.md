# Explorer-Free Owned Shell Design

## Objective

SuperDesktop shall own every visible shell surface required after `explorer.exe` exits. SuperExplorer remains the file manager. Normal steady-state Shell mode contains SuperDesktop, its isolated brokers and SuperExplorer, without using Explorer as a hidden Start, taskbar, desktop, notification-area, input-language, or system-flyout host.

Explorer remains a recovery target only. If SuperDesktop loses ownership or crashes, the guardian restores a usable Explorer session and the original work areas. This safety path does not count as normal Explorer-free operation.

## Scope and completion boundary

The program covers five independently verifiable subsystems:

1. An always-owned GPUI Start surface.
2. Windows-style desktop selection and interaction, including pointer marquee selection.
3. Real input-language, network, volume, power, clock and notification status providers with owned GPUI flyouts.
4. An isolated legacy notification-area compatibility boundary for `Shell_NotifyIcon` clients.
5. Transactional Explorer-free lifecycle admission, takeover, steady-state verification and guardian recovery.

SuperExplorer owns file browsing and folder windows. SuperDesktop does not duplicate SuperExplorer's file-manager UI.

The program does not claim undocumented virtual-desktop operations, private Windows notification-center history, arbitrary Explorer extensions, or every private Shell protocol. Unsupported private behavior must be reported as unavailable instead of being silently delegated to Explorer or represented by fake state.

## Current-state corrections

The current source contains three behaviors that must be removed before Explorer-free completion can be claimed:

- The taskbar Start callback invokes the system Start host outside Shell or verification-owned modes.
- The taskbar status constructor supplies fixed `online`, volume `40`, unmuted, `zh-TW`, desktop battery and zero-notification values.
- The notification-area host accepts the owned provider protocol but does not expose the Win32 compatibility entry used by ordinary `Shell_NotifyIcon` applications.

The desktop already has live normal/reverse marquee selection, Ctrl-additive selection, item event isolation and DPI-aware rendering. That implementation remains authoritative and is extended only where Explorer-free headful verification finds a concrete interaction gap.

## Architecture

### 1. SuperDesktop composition root

`superdesktop-app` remains the only process that composes visible GPUI shell surfaces. It consumes owned DTO snapshots and emits typed commands. GPUI code does not own raw HWND, HICON, COM, TSF, Core Audio, NLM or power handles.

The composition root owns:

- per-monitor desktop surfaces;
- per-monitor taskbars;
- one centered Start surface per active invocation;
- status and overflow flyouts;
- persisted Start, taskbar and desktop preferences;
- exact-generation reconciliation for every broker snapshot.

### 2. Platform adapters and isolated brokers

Native Windows integration is split by failure boundary:

- `platform-win` exposes small owned adapters for documented synchronous APIs and typed commands.
- `notification-area-host` owns legacy notification compatibility, client leases, native icon copying and callback delivery.
- a new `system-status-host` owns COM/TSF/Core Audio/NLM subscriptions and publishes bounded status snapshots.

Neither broker loads third-party code into the GPUI process. Each connection has a versioned handshake, capacity bounds, heartbeat, generation and exactly-once terminal result. A broker crash clears only that provider's visible state, shows truthful unavailable UI and triggers bounded restart/reconciliation.

### 3. Shared contracts

`shell-provider-protocol` gains versioned DTOs for:

- input profiles, active profile and switch result;
- volume scalar, mute, endpoint identity and change generation;
- network connectivity and display label;
- AC/battery state and remaining percentage;
- clock/calendar locale metadata;
- notification icon identity, tooltip, copied pixels, version, visibility and callback route;
- provider health, overflow and restart snapshot.

All strings, result counts, icon dimensions, frame sizes and pending events are bounded. Stale generation results cannot change visible state.

## Owned Start

The Start taskbar callback always toggles `StartView`. The system Start probe/invocation path is removed from product behavior and retained only in historical capability evidence.

Start owns:

- Search;
- a bounded six-column Pinned grid;
- Recommended results;
- All apps with alphabetical ordering;
- Settings launch entries;
- Account presentation;
- collapsed Power actions with explicit confirmation;
- app, settings and file activation;
- keyboard, pointer, UIA and IME composition.

Application discovery uses Start Menu shortcut roots, registered applications and the existing bounded provider. No result may require Explorer's Start process to render or activate. Closing, Escape, focus return, monitor placement and small-work-area clamping remain deterministic.

## Desktop interaction

The desktop keeps SuperExplorer's fixed entry in a reserved grid cell and reconciles all real desktop items around it. Empty-space left drag starts marquee selection; item press stops background propagation. The marquee supports forward and reverse geometry, live intersection, ordinary replace, Ctrl union, final selection persistence and lost-button cancellation.

Explorer-free verification must cover:

- empty click and marquee selection;
- Ctrl/Shift selection and keyboard focus;
- icon reposition without file transfer;
- desktop file operations and context menus;
- SuperExplorer fixed entry activation;
- User and Public Desktop enumeration;
- watcher overflow and authoritative refresh;
- 100%, 125%, 150%, 175% and 200% DPI.

## Input-language provider and flyout

`system-status-host` runs in the interactive session and uses documented TSF and keyboard-layout APIs. It publishes stable profile identity, language tag, display name, active state and availability. It observes foreground/input-language changes without polling fixed text.

The taskbar renders the real active profile. Activating it opens an owned GPUI input switcher. Pointer, keyboard and UIA invocation call a typed activation command; success is accepted only after the provider observes the requested profile as active. Failure leaves the prior profile visible and exposes a recoverable error.

IME composition inside Start remains independent of the taskbar profile switcher. Switching profiles must not lose Start query composition or taskbar keyboard focus.

## Core system status and flyouts

The status host uses documented sources:

- Core Audio endpoint APIs for volume and mute;
- Network List Manager or an equivalent documented connectivity source;
- `GetSystemPowerStatus` for AC and battery state;
- Windows local date/time and locale APIs for clock and calendar;
- owned notification registry counts for notification state.

The taskbar never substitutes fixed values. Each provider is independently available or unavailable. Clicking status controls opens owned GPUI flyouts for volume, network summary, power summary, calendar and notification overflow. Volume changes are applied through Core Audio and confirmed by the subsequent snapshot. Network UI is informational unless a documented settings command is available. Power actions retain explicit confirmation.

## Legacy notification-area compatibility

When Explorer-free Shell mode is admitted, `notification-area-host` creates the compatibility window/class surface required for supported `Shell_NotifyIcon` traffic. Preview mode never competes with Explorer for that identity.

The host validates and copies supported `NOTIFYICONDATA` versions at the process boundary. Raw HICON ownership never crosses into GPUI; pixels are copied to owned RGBA/BC7-compatible data. Add, modify, delete, set-focus, version negotiation, tooltip, visibility and callback messages map to the existing generation-bound registry.

Client identity is bound to process/session/window identity. Dead clients, invalid callback windows, oversize icons, malformed structures and capacity overflow fail closed. After host restart or Shell takeover, the host emits the documented taskbar-created recovery notification and reconciles re-registered clients. Unsupported private toolbar protocols remain unavailable.

## Explorer-free lifecycle

Shell takeover stays transactional:

1. validate the interactive reference session and single-owner lease;
2. arm guardian recovery;
3. start and handshake system-status and notification brokers;
4. create all desktop, taskbar, Start and flyout-capable surfaces;
5. acquire AppBars and Shell hooks;
6. verify owned Start, status, input-language and notification health;
7. switch away from Explorer surfaces and stop Explorer for normal steady state;
8. confirm no eligible Explorer shell process remains;
9. publish an Explorer-free terminal record.

Any failure before step 7 unwinds only SuperDesktop resources and leaves Explorer unchanged. Failure after step 7 invokes guardian recovery. Normal shutdown also restores Explorer unless a separately specified persistent-login-shell installer is explicitly enabled; this design does not broaden installer authority.

## Error handling

- Every native callback is a no-unwind boundary.
- Provider queues are bounded and emit explicit overflow events.
- Stale snapshots and callbacks are rejected by generation.
- Provider restarts clear stale icons/state before applying a full snapshot.
- Missing Start, status or input providers block Explorer-free takeover instead of displaying placeholders.
- A normal broker failure after takeover keeps desktop/taskbar alive, marks the affected region unavailable and attempts bounded restart; loss of lifecycle-critical ownership escalates to guardian recovery.
- Diagnostics redact user paths, clipboard, credentials and unbounded native payloads.

## Implementation sequence

The work is split into five OpenSpec changes with explicit ownership:

1. `make-owned-start-exclusive` removes system Start invocation and proves all Start modes and actions are owned.
2. `add-system-status-ime-host` adds real status/TSF providers and GPUI flyouts.
3. `add-shell-notifyicon-compatibility` adds the isolated Win32 notification compatibility boundary.
4. `complete-explorer-free-shell-lifecycle` requires all owned providers before Explorer exits and proves transactional recovery.
5. `verify-explorer-free-shell-parity` performs cross-domain, DPI, accessibility, stress, lifecycle and installer verification.

Changes 2 and 3 depend on shared contract additions owned by change 2. Change 4 depends on 1–3. Change 5 depends on all prior changes. Existing incomplete external release gates remain active and are not replaced by this program.

## Verification

Automated gates:

- complete locked/offline workspace check, test and clippy;
- provider protocol malformed/capacity/stale/cancel/restart matrices;
- owned Start source guard proving no system Start invocation;
- status adapters with deterministic documented-API fixtures;
- notification structure/version/icon/callback negative tests;
- lifecycle failpoint and single-owner tests;
- strict OpenSpec validation and unique task-linked evidence.

Headful gates on the active Windows 11 reference host:

- `explorer.exe` absent during the measured steady-state interval;
- Start home, All apps, Search, Settings and Power rendered by SuperDesktop;
- desktop marquee visible and selecting real desktop items;
- real active input profile displayed and switched through GPUI;
- real volume/mute, network, power, clock and calendar state displayed;
- a controlled legacy `Shell_NotifyIcon` client completes add/modify/callback/delete;
- notification overflow, keyboard focus and UIA names/actions work;
- SuperExplorer launches from desktop, taskbar and Start;
- forced SuperDesktop crash restores Explorer and work areas within the existing deadline.

Release evidence records binary hashes, OS/profile identity, active processes, HWND/class ownership, provider generations, screenshots, UIA trees, raw lifecycle timestamps and installer hashes. No gate may infer Explorer-free success solely from unit tests.

## Rollback

The implementation is feature-gated until the Explorer-free lifecycle gate passes. Disabling the feature returns to the current transactional Shell behavior. Runtime failure releases AppBars, closes owned compatibility windows, restores work areas and starts or reveals the verified system Explorer through the existing guardian path. No destructive filesystem rollback is required.
