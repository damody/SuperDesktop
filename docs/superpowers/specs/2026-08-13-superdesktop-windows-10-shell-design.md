# SuperDesktop Windows 10 Shell Design

Date: 2026-08-13  
Status: Approved design; implementation not started

## 1. Purpose

SuperDesktop is a Windows-only desktop environment implemented in Rust. It recreates the user-facing Windows 10 22H2 desktop Shell while using `D:\SuperExplorer` as its file manager. All product-visible surfaces are rendered with GPUI. Windows APIs remain behind a narrow, non-visual adapter because desktop ownership, AppBars, Shell hooks, COM/OLE, notification-area compatibility, monitor topology, and DPI events cannot be implemented reliably without them.

The project root is `D:\SuperDesktop\SuperDesktop`. It is an independent Cargo workspace and Git repository. `D:\SuperExplorer` remains an independently built application and is integrated through a process boundary.

## 2. Scope

### 2.1 Long-term product scope

The complete product targets Windows 10 Shell behavior in these areas:

- Desktop surfaces, wallpaper, Shell items, selection, layout, rename, context menus, drag-and-drop, refresh, and multi-monitor behavior.
- A bottom taskbar whose default high-density layout follows the supplied two-row reference image, with configurable one-to-three-row layouts.
- Window tracking, activation, minimize/restore, grouping, pinning, ordering, progress/attention states, thumbnails, jump lists, Task View, and Show Desktop.
- Start button, Start menu, application indexing, search, Run, power, session, and common Shell commands.
- Notification area, clock, calendar, input/network/volume/power status, notification badges, and compatible third-party tray icons.
- Keyboard shortcuts, accessibility, high contrast, per-monitor DPI, multiple monitors, virtual-desktop integration, autostart, controlled Shell installation, and recovery.
- SuperExplorer as the handler for file-system folder navigation.

“Complete Windows 10 behavior” means the observable desktop-Shell capability and interaction set, tracked in a parity matrix with automated or manual evidence. It does not mean reproducing undocumented implementation details pixel-for-pixel where Windows APIs or third-party providers control the result.

### 2.2 Out of scope

SuperDesktop does not reimplement the Windows kernel, compositor, logon or lock screen, security boundary, device drivers, Control Panel internals, Settings internals, bundled UWP applications, networking stack, audio stack, or file system. It integrates the existing Windows services that provide those capabilities.

M0 does not deliver the whole long-term scope. It establishes the production architecture and provides the first usable desktop and taskbar slice described below.

## 3. Source and licensing boundaries

`D:\SuperDesktop\PExplorer` is an LGPL-2.1-or-later C++/Win32 reference derived from ReactOS Explorer work. It may be used to understand behavior, message flows, Shell hooks, AppBar usage, and failure cases. SuperDesktop must not copy or mechanically translate its source. Any intentionally derived code requires a separate licensing decision and explicit attribution before it enters the product.

`D:\SuperExplorer` is treated as a separately built product. SuperDesktop must not depend on its uncommitted source tree, internal crates, or `vendor/gpui-ce` path. This protects both projects from build coupling and preserves the user's existing SuperExplorer changes.

SuperDesktop pins its own GPUI-CE revision. Using the same known-good revision as SuperExplorer is preferred initially, but each repository owns its dependency lock and upgrade schedule.

## 4. Architectural approach

### 4.1 Visible and platform layers

All SuperDesktop-owned visible desktop, taskbar, Start, search, notification, settings, prompt, and recovery surfaces are GPUI views. Windows-owned or third-party surfaces that SuperDesktop invokes, such as the M0 Windows Start experience or a native Shell context menu, remain externally rendered. The platform layer may create invisible/message-only HWNDs and perform Windows API calls, but it must not implement SuperDesktop product UI.

The primary event flow is:

```text
Windows event -> platform-win -> shell-core transition -> immutable snapshot -> GPUI render
GPUI action -> shell-core command -> platform-win or explorer-bridge effect -> result event
```

Platform callbacks never mutate GPUI state directly. They emit typed events with stable window, monitor, desktop-item, and request identities. `shell-core` is the only authority for user-visible state transitions.

### 4.2 Workspace units

- `superdesktop-app`: composition root, command-line modes, diagnostics, process lifetime, and orderly shutdown.
- `shell-core`: platform-independent state machines for monitors, desktop items, selection, task buttons, grouping, pinning, focus, commands, and recovery phases.
- `platform-win`: non-visual Windows integration for desktop/Shell registration, Shell hooks, window enumeration, AppBars, monitor/DPI events, COM/OLE, known folders, Shell identities, icons, and context-menu sessions.
- `desktop-ui`: GPUI desktop surface, wallpaper, item layout, interaction, and accessibility semantics.
- `taskbar-ui`: GPUI taskbar, task buttons, Start entry point, notification region, clock, overflow, and accessibility semantics.
- `explorer-bridge`: discovery, validation, launch, error reporting, and future versioned IPC for SuperExplorer.
- `settings-store`: versioned configuration, atomic writes, migration, corruption quarantine, and layout persistence.
- `superdesktop-guardian`: minimal Rust recovery process that observes the Shell lease and restores a usable Windows session after abnormal termination.
- `superdesktop-test-support`: fake platform adapters, deterministic clocks, owned fixture roots, window helpers, visual fixtures, and failure injection.

Each unit exposes typed public contracts. `desktop-ui` and `taskbar-ui` depend on `shell-core` values and commands, not on Win32 handles. Windows handles and COM interfaces do not escape `platform-win`.

## 5. Runtime modes and ownership

### 5.1 Preview mode

Preview mode is the default during development. It renders the desktop and taskbar inside ordinary GPUI windows without hiding Explorer or changing the system work area. It must not modify Shell-related registry values. This mode is safe for UI iteration and automated headful tests.

### 5.2 Shell mode

Shell mode is explicit. It uses a transactional takeover:

1. Start `superdesktop-guardian` and establish a lease.
2. Initialize diagnostics, GPUI, COM apartments, typed event channels, and settings.
3. Create all monitor desktop surfaces and taskbar AppBars.
4. Register Shell hooks, hotkeys, notifications, monitor/DPI listeners, and recovery callbacks.
5. Complete a health check that confirms usable desktop and taskbar input.
6. Only after all prior steps succeed, hide or replace the Explorer-owned Shell surfaces for the session.

Failure before step 6 unwinds only SuperDesktop resources. Failure after step 6 causes the guardian to remove AppBars, restore work areas, reveal an existing Explorer Shell, or start `explorer.exe` if no usable Explorer Shell exists.

M0 provides runtime Shell mode but does not change the configured Windows logon Shell. Registry installation, startup integration, uninstall, and recovery UI are later controlled-installation work.

## 6. M0 functional design

### 6.1 Desktop

M0 creates one bottommost GPUI desktop surface per active monitor. It supports Windows wallpaper placement modes: fill, fit, stretch, center, tile, and span where monitor topology permits it.

The item source combines the current user's Desktop known folder and the Public Desktop known folder. Items use stable Shell identity rather than display name as identity. M0 supports:

- Real Shell icon and display-name resolution.
- Single selection, Ctrl/Shift selection, rubber-band selection, focus, keyboard navigation, and activation.
- Double-click and Enter activation.
- Persistent icon positions keyed by stable item identity, monitor identity, and DPI-aware logical coordinates.
- File-system change observation with coalescing and full-refresh recovery after watcher overflow.

Opening a file uses the normal Windows association. Opening a file-system directory goes through `explorer-bridge`. The “This PC” entry launches SuperExplorer without an initial file-system path because the current SuperExplorer startup contract only accepts absolute directories through `EXPLORER_INITIAL_PATH`.

Rename, native context menus, drag-and-drop, auto-arrange, grid alignment, sorting, refresh commands, and Recycle Bin mutation are planned desktop-parity increments after M0. M0 must reserve typed commands and state boundaries for them without presenting enabled controls prematurely.

### 6.2 Taskbar

The taskbar docks at the bottom and registers as a Windows AppBar in Shell mode. The default layout is two compact rows, matching the supplied reference's information density. Users can configure one, two, or three rows. Per-monitor taskbars are represented from the start; the initial acceptance matrix requires a primary taskbar and a secondary-monitor taskbar in a two-monitor setup.

The left edge contains the Start button. In M0 it invokes the existing Windows Start experience rather than drawing a custom Start menu. Shell takeover is allowed on the Windows 10 reference platform only when the Start host capability probe succeeds; a later invocation failure is reported as a recoverable degraded state. Preview mode and non-reference Windows builds may show the button as accessibly unavailable when the host is absent. The central region contains task buttons with the application icon and an ellipsized title. A blue underline indicates a running task or group; an active background indicates the foreground task. The right region contains the overflow entry, core system status, time, date, and notification count.

M0 window behavior includes:

- Discovering eligible top-level windows through Shell-hook events plus periodic `EnumWindows` reconciliation.
- Filtering tool, cloaked, owned transient, invisible, and explicitly excluded windows.
- Stable ordering during title, icon, attention, minimize, and foreground changes.
- Clicking to activate, minimize the foreground window, or restore a minimized window.
- Launching a pinned application when it has no window.
- Grouping windows by resolved application identity while retaining per-window child state.
- Icon and title caching with bounded invalidation.
- A pinned SuperExplorer entry.

M0's notification region provides time/date and Windows-derived core status. Full third-party `Shell_NotifyIcon` compatibility, overflow management, jump lists, live thumbnails, drag reordering, badges, progress, and custom Start/search surfaces are later taskbar-parity increments.

### 6.3 SuperExplorer integration

M0 uses the existing SuperExplorer startup contract instead of changing its dirty working tree:

- The configured executable is validated as an absolute existing executable before launch.
- Resolution checks a persisted user setting first, then the development artifact `D:\SuperExplorer\target\release\SuperExplorer.exe`, then an installed `SuperExplorer.exe` adjacent to SuperDesktop.
- A file-system directory launch creates a new SuperExplorer process with `EXPLORER_INITIAL_PATH` set to that existing absolute directory and no unsupported command-line arguments.
- “This PC” launches SuperExplorer without `EXPLORER_INITIAL_PATH`.
- Launch requests carry a correlation ID and produce exactly one success or failure event.
- A missing, invalid, or failed executable produces a GPUI recovery prompt and diagnostic event. SuperDesktop does not silently substitute Windows Explorer for folder navigation.

Targeted navigation in an existing SuperExplorer process requires a future versioned IPC contract. That addition belongs to its own coordinated OpenSpec change in SuperExplorer and is not implied by M0.

## 7. State, concurrency, and data flow

`shell-core` owns a single logical snapshot containing monitor topology, desktop models, taskbar models, selection/focus, application identities, settings revision, and recovery phase. Platform work is asynchronous and tagged with a request ID plus a generation. Results from stale generations are rejected.

High-frequency window, foreground, title, icon, and filesystem events are coalesced by stable identity before they reach the renderer. Bounded queues expose overflow rather than dropping state silently. Overflow schedules an authoritative reconciliation: `EnumWindows` for tasks and a full namespace refresh for desktop items.

COM/OLE apartment-affine values remain on their owning platform threads. Cross-thread events contain owned Rust values. Shutdown stops new commands, cancels in-flight requests, unregisters external callbacks, removes AppBars, destroys GPUI windows, releases COM resources, flushes diagnostics/settings, and finally releases the guardian lease.

## 8. Settings and persistence

Settings use an explicitly versioned schema and atomic replace-on-success writes. Stored values include runtime mode preferences, taskbar row count, monitor placement, pin ordering, wallpaper placement, desktop item coordinates, SuperExplorer executable path, theme, and accessibility preferences.

Invalid fields fall back independently where safe. An unreadable or structurally invalid settings file is renamed to a timestamped quarantine file, recorded in diagnostics, and replaced with safe defaults. Layout data is keyed by stable monitor and Shell-item identities so DPI or display reordering does not corrupt unrelated layouts.

## 9. Error handling and recovery

- Failure to create every required surface or AppBar prevents Shell takeover.
- Failure to register optional telemetry or a non-essential status provider degrades only that provider and shows truthful availability.
- Loss or overflow of Shell-hook events triggers authoritative window reconciliation.
- Desktop watcher overflow triggers an authoritative desktop refresh while preserving selection by stable identity.
- SuperExplorer launch failure leaves the desktop and taskbar responsive and exposes a repair action.
- A hung third-party Shell or tray provider must be hosted behind a bounded worker or process boundary before the related capability is enabled.
- Settings corruption is quarantined and replaced; it must not prevent a usable Shell.
- Panic or abnormal process termination activates guardian recovery. Recovery is idempotent and safe to repeat.
- Diagnostics avoid file contents, credentials, clipboard data, and full user paths unless a local debug mode explicitly enables them.

## 10. Accessibility, localization, and visual behavior

Every interactive GPUI element has a stable accessibility identity, role, name, state, and action. Keyboard-only operation covers desktop selection, task traversal, activation, context actions when implemented, and recovery prompts. Focus is always visible. High-contrast mode maps semantic tokens to system roles rather than applying fixed colors.

Layout uses logical units and per-monitor DPI. Required validation scales are 100%, 125%, 150%, 175%, and 200%. Text truncation, bidirectional layouts, Traditional Chinese, Simplified Chinese, English, and IME interaction are first-class constraints. M0 ships at least Traditional Chinese and English resource coverage for visible strings.

The Windows 10 visual target is verified through semantic tokens, geometry measurements, and reference captures. Exact third-party icon rendering, font rasterization, and OS-provider pixels are treated as controlled external variation.

## 11. Verification strategy

### 11.1 Automated tests

- Unit tests for every `shell-core` transition, eligibility rule, grouping rule, selection rule, ordering rule, generation check, and recovery transition.
- Property and sequence tests for duplicate, missing, reordered, stale, and overflowed Shell events.
- Contract tests with fake platform, monitor, clock, settings, and explorer adapters.
- Windows integration tests using owned helper windows and owned temporary directories only.
- Process tests for SuperExplorer resolution and environment construction without modifying the SuperExplorer repository.
- Crash and startup-failure injection at each takeover phase.
- Accessibility tree and action tests.
- Deterministic GPUI visual fixtures for desktop/taskbar states and DPI scales.
- Cargo format, check, clippy with warnings denied, workspace tests, architecture checks, and dependency/license audits.

### 11.2 Headful and manual matrix

- Windows 10 22H2 x64 is the reference platform; Windows 11 compatibility is tracked separately and must remain usable.
- One- and two-monitor configurations with mixed DPI.
- Light, dark, and high-contrast themes.
- Keyboard, pointer, touch-sized hit targets, IME, and UI Automation inspection.
- Window storms, title/icon churn, hung helpers, missing SuperExplorer, display hot-plug, Explorer restart, and SuperDesktop crash recovery.
- Preview-mode coexistence and Shell-mode takeover/restore.

### 11.3 M0 performance budgets

- Cold launch to interactive preview or completed Shell health check: no more than 2 seconds on the reference machine.
- Idle CPU median after settling: below 0.5%.
- Shell event to visible taskbar update: p95 below 100 ms.
- M0 working set after settling: below 150 MiB on the reference machine.
- Event queues remain bounded during stress; recovery reaches an authoritative state after overflow.

## 12. M0 acceptance criteria

M0 is complete only when all of the following are verified:

1. Preview mode renders usable desktop and taskbar surfaces without changing the active Windows Shell.
2. Shell mode completes transactional takeover and normal exit restores Explorer and the work area.
3. Guardian recovery restores a usable Explorer session after forced SuperDesktop termination.
4. Desktop selection, rubber-band selection, keyboard navigation, activation, wallpaper modes, and persisted icon positions work on real Shell items.
5. File-system folders launch SuperExplorer with the verified `EXPLORER_INITIAL_PATH` contract; missing SuperExplorer produces a recoverable GPUI error.
6. The two-row taskbar tracks, activates, minimizes, restores, groups, and pins real application windows without unstable reordering.
7. Primary and secondary taskbars remain correctly placed through mixed-DPI display changes.
8. On Windows 10 22H2, the Start button invokes the Windows Start experience in both preview and Shell sessions; Shell takeover is refused if its prerequisite capability probe fails.
9. Accessibility, localization, visual, lifecycle, stress, and performance gates pass with recorded evidence.
10. No test or recovery routine modifies or deletes data outside an explicitly owned fixture root.
11. The parity matrix truthfully marks deferred Windows 10 capabilities rather than presenting placeholders as completed functionality.

## 13. Delivery sequence after M0

Later changes remain independently specified and verified:

1. Desktop parity: rename, menus, drag/drop, arrange/sort, refresh, Recycle Bin, and advanced namespace items.
2. Taskbar parity: reordering, jump lists, thumbnails, progress, badges, attention, full pin lifecycle, and multi-monitor policy.
3. Start and search: application index, Win32/UWP entries, pinned tiles/list behavior, search, Run, power, and session commands.
4. Notification area and Action Center: third-party tray compatibility, overflow, flyouts, clock/calendar, quick actions, and notifications.
5. Shell integration: shortcuts, virtual-desktop integration, settings, autostart, controlled installation, uninstall, and recovery UI.
6. Parity hardening: complete Windows 10 capability matrix, accessibility, localization, performance, soak, and release evidence.

Each increment receives its own OpenSpec proposal, design, delta specs, task plan, implementation, and verification evidence.

## 14. Decisions made during brainstorming

- Use all-GPUI visible UI, with only a minimal invisible Win32 adapter.
- Include both desktop and taskbar; prioritize the supplied two-row taskbar reference.
- Treat Windows 10 22H2 as the behavioral reference and Windows 11 as a compatibility target.
- Preserve a process boundary between SuperDesktop and SuperExplorer.
- Use the existing `EXPLORER_INITIAL_PATH` contract for M0 instead of modifying SuperExplorer immediately.
- Use preview mode by default and transactional Shell takeover only when explicitly requested.
- Add an independent Rust guardian before any session-level Shell replacement.
- Treat PExplorer as behavioral reference material, not a source to port mechanically.
- Decompose full parity into separately specifiable increments rather than claiming complete Windows 10 parity in the first build.
