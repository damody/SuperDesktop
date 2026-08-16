# SuperDesktop Windows Shell Completion Design

## 1. Purpose and approved scope

SuperDesktop M0 already supplies the safe GPUI desktop/taskbar foundation, typed Shell state, SuperExplorer launch bridge, transactional takeover, guardian recovery, and evidence system. This design completes the user-visible Windows 10 desktop and taskbar feature set that M0 deliberately deferred.

The target is functional parity for common Windows 10 desktop and taskbar workflows on Windows 10 22H2 and the frozen Windows 11 reference environment. The target includes controlled compatibility with third-party tray and Shell providers. It does not claim bit-for-bit equivalence with undocumented Explorer internals and does not replace the Windows kernel, DWM, Winlogon, credential UI, security desktop, or built-in system applications.

## 2. Delivery strategy

Three approaches were considered:

1. Extend the current desktop and taskbar crates directly. This minimizes crate count but mixes untrusted COM providers, file mutation, search, and installation with rendering and state reduction.
2. Delegate advanced behavior to Explorer. This reduces implementation work but prevents SuperDesktop from owning a coherent desktop/taskbar experience and makes failure recovery depend on Explorer UI.
3. Add bounded feature services and isolated provider hosts behind the existing typed command/effect boundary. This preserves the current architecture, contains third-party failures, and keeps GPUI surfaces testable.

The approved approach is option 3. Work is split into a program change and apply-ready child changes. Shared contracts land before consumers. Each child owns distinct files and evidence. No child may weaken the existing M0 takeover, recovery, safety, performance, or evidence gates.

## 3. Program decomposition and order

1. `extend-superdesktop-shell-contracts`
   - Shared desktop mutation, context menu, drag/drop, search, Jump List, thumbnail, tray, virtual desktop, installation, and provider-host DTOs.
   - Capability detection, cancellation, deadline, generation, terminal-result, and error contracts.
2. `add-superdesktop-desktop-file-operations`
   - Rename, refresh, delete/recycle, sorting, alignment, position persistence, and file-transfer drag/drop.
3. `add-superdesktop-shell-context-menu-host`
   - Out-of-process Shell context menu enumeration/invocation with bounded COM execution.
4. `add-superdesktop-start-search`
   - GPUI Start menu, pinned/all-apps/recent sections, application/file/settings search, keyboard/IME/accessibility behavior.
5. `add-superdesktop-taskbar-advanced-interactions`
   - Jump Lists, task thumbnails, group flyouts, pin/reorder persistence, and advanced pointer behavior.
6. `add-superdesktop-notification-area-host`
   - Tray icon compatibility, tooltip, menus, notifications, provider isolation, and restart reconciliation.
7. `add-superdesktop-virtual-desktops`
   - Enumerate, switch, create, remove, move windows, and filter taskbar state when supported.
8. `add-superdesktop-shell-installer`
   - Install/update/uninstall, explicit login-Shell opt-in, atomic registry changes, rollback, Safe Mode recovery, and emergency Explorer restoration.
9. `verify-superdesktop-shell-completion`
   - Cross-feature, OS, DPI, monitor, provider-failure, performance, security, recovery, and independent-review release gates.

The parent program change is `complete-superdesktop-windows-shell`. It coordinates the dependency DAG and release disposition; it does not replace child task plans.

## 4. Architecture

### 4.1 Existing boundaries retained

- `shell-core` remains the only owner of authoritative user-visible state and typed commands/effects.
- `desktop-ui`, `taskbar-ui`, and new GPUI surfaces consume owned values only. They do not expose HWND, PIDL, COM interfaces, registry handles, or provider pointers.
- `platform-win` owns Windows API, COM/OLE, Shell identity, virtual-desktop capability probes, registry transactions, and native message adaptation.
- `superdesktop-app` remains the composition root and lifecycle coordinator.
- `superdesktop-guardian` remains the only crash-recovery authority after validated takeover.

### 4.2 New components

- `shell-provider-protocol`: versioned, serialization-safe request/result messages shared with isolated hosts.
- `shell-provider-host`: low-integrity-compatible out-of-process host for context menus, thumbnails, Jump Lists, and other potentially blocking Shell extensions.
- `start-search`: pure query/ranking/state logic plus GPUI Start/search presentation.
- `notification-area`: tray identity, ordering, status, input, and reconciliation model.
- `virtual-desktop`: capability-neutral state and commands; Windows implementation remains in `platform-win`.
- `shell-installer`: declarative install plan, registry transaction journal, rollback plan, and signed product identity checks.

Provider hosts are disposable. The main process never trusts provider output without correlation ID, generation, size limits, identity validation, and schema validation.

## 5. Feature behavior

### 5.1 Desktop mutation and organization

- F2 and inline editing rename exactly one selected item through stable Shell identity.
- F5 requests an authoritative namespace refresh without discarding stable selection or valid positions.
- Delete uses Recycle Bin semantics by default; permanent deletion requires an explicit distinct command and confirmation.
- Sorting and alignment operate on logical coordinates and preserve deterministic ordering across DPI changes.
- Pointer drag distinguishes icon reposition from file transfer using movement threshold, source/target identity, key modifiers, and drop capability.
- Copy/move results are asynchronous, cancellable, generation-fenced, and reconciled through a namespace refresh.
- Conflicts and provider failures appear as keyboard/UIA-operable GPUI recovery prompts; no mutation is reported as successful before a terminal result.

### 5.2 Shell context menus

- The main process requests a menu descriptor from `shell-provider-host`; it renders the visible menu in GPUI.
- Commands retain opaque provider tokens only inside the host. The UI receives sanitized labels, hierarchy, enabled/default state, icons, and stable request-local command IDs.
- Enumeration and invocation have separate deadlines. Timeout, crash, malformed output, excessive nesting, or oversized payload terminates the request and may restart the host.
- Built-in safe actions remain available when third-party provider enumeration fails, but SuperDesktop must label this as a reduced menu rather than a complete native result.

### 5.3 Start menu and search

- Start is a SuperDesktop-owned GPUI surface with pinned apps, all apps, recently used items, power/session actions, and search.
- Search providers cover installed applications, file-system results through Windows Search when available, and a curated settings catalog.
- Each provider streams bounded result batches with provider identity, query generation, ranking inputs, and terminal state. Stale results cannot replace a newer query.
- Search does not read file contents outside provider policy and does not log raw queries by default.
- Keyboard-only navigation, IME composition, high contrast, RTL/bidi, and UIA roles/actions are release-blocking.

### 5.4 Advanced taskbar behavior

- Jump Lists are loaded through the isolated provider protocol and rendered in GPUI.
- Thumbnails use bounded capture/cache budgets and become static placeholders when capture is unavailable or protected.
- Group flyouts preserve stable application/window identity and expose activate, minimize, restore, close, and move-to-desktop actions when eligible.
- Pin/reorder changes are atomic, revisioned, and reconciled with running groups without title/icon events causing reorder.

### 5.5 Notification area

- A dedicated native compatibility adapter receives supported tray registration/update/removal messages and converts them into typed notification-area events.
- Icon identity includes owner process identity, callback identity, GUID when supplied, and generation fencing.
- Tooltip, pointer/keyboard activation, context menu requests, notification count, and balloon/toast handoff are supported where the provider contract permits.
- Provider callbacks execute outside the main UI path. Hung or crashed providers cannot block rendering, task switching, Start, or guardian recovery.
- Unsupported or unverifiable tray behavior remains truthful and does not create fake icons.

### 5.6 Virtual desktops

- Public state represents desktop identity, name when available, current desktop, window membership, and capability disposition.
- Commands include switch, create, remove with fallback target, and move eligible windows.
- Windows build-specific adapters are capability-probed. Unsupported builds fail closed without corrupting taskbar state.
- Reconciliation after Explorer restart, DWM event loss, or stale callback uses an authoritative snapshot.

### 5.7 Installation and login-Shell selection

- Installation copies only signed/hashed product artifacts into an explicit product directory and records a versioned manifest.
- Login-Shell replacement requires a separate explicit command and preflight: guardian availability, Explorer recovery target validation, interactive session, supported OS, and recovery shortcut.
- Registry changes use a journaled transaction with before/after values and atomic rollback. Preview mode never modifies login-Shell configuration.
- Update and uninstall restore the prior Shell configuration before removing binaries. If restoration cannot be proven, destructive removal is refused.
- Safe Mode and emergency recovery always prefer verified system Explorer.

## 6. Data flow and concurrency

All user actions follow:

`GPUI action -> typed command -> reducer admission -> typed effect -> platform/host request -> terminal result -> reducer -> rendered state`

Every asynchronous request has a correlation ID, generation, deadline, cancellation owner, exactly-once terminal result, and late-result diagnostic path. Bounded queues coalesce repeatable events. Overflow produces an explicit event and authoritative reconciliation. No COM callback or external process callback mutates GPUI state directly.

## 7. Security and failure handling

- Canonicalize and validate every mutation target. Reject workspace root, drive root, user profile root, reparse escape, protected system locations, and unresolved paths unless the user explicitly selected an allowed target through the Shell contract.
- File operations retain undo/recycle semantics where Windows provides them and never silently fall back from recycle to permanent delete.
- Provider hosts use restricted handle inheritance, controlled environment/working directory, request size limits, and process lifetime limits.
- Registry and installer operations validate exact keys and values; no wildcard or computed broad deletion is allowed.
- FFI callbacks remain no-unwind boundaries and validate ownership before accessing handles.
- Crash loops degrade the affected provider only. They do not disable guardian recovery or leave the session without Explorer.

## 8. Performance budgets

- Start surface first visible frame: at most 250 ms after invocation on the reference machine.
- Local application search first batch: at most 150 ms; all enabled providers reach terminal state within 2 seconds unless explicitly cancelled.
- Context menu built-in first frame: at most 250 ms; external provider enrichment terminal within 2 seconds.
- Task thumbnail first eligible image: at most 500 ms with bounded cache memory.
- Notification event-to-visible p95: below 100 ms under the declared fixture load.
- Existing M0 cold-start, idle CPU, event-to-visible, working-set, and guardian thresholds remain unchanged.

## 9. Testing and evidence

Each child change owns append-only evidence with unique task IDs. Required test layers are:

- Pure model tests for identity, ordering, cancellation, deadlines, stale generations, conflict resolution, and recovery.
- Fake-provider contract tests, malformed payloads, timeouts, crashes, duplicate callbacks, and queue overflow.
- Windows integration tests for file operations, Recycle Bin, COM/OLE, search, tray messages, virtual desktops, registry transactions, and Explorer recovery.
- GPUI headful tests for pointer, keyboard, UIA, high contrast, localization, IME, bidi, DPI, and multi-monitor layout.
- Security tests for path/reparse escape, argument/environment substitution, provider spoofing, handle inheritance, registry target validation, and diagnostic redaction.
- Performance and resource soak tests with raw samples.

Mandatory release environments remain Windows 10 22H2, the frozen Windows 11 reference profile, 100/125/150/175/200% DPI, virtual mixed-DPI, and physical mixed-DPI confirmation. Missing external environments remain blocked rather than not-applicable.

## 10. Blocking gates

- `G-DESKTOP-FILE-OPS`
- `G-CONTEXT-MENU-HOST`
- `G-START-SEARCH`
- `G-TASKBAR-ADVANCED`
- `G-NOTIFICATION-AREA`
- `G-VIRTUAL-DESKTOP`
- `G-INSTALL-ROLLBACK`
- Existing `G-ARCH`, `G-SAFETY`, `G-A11Y-I18N`, `G-DPI-MONITOR`, `G-PERF`, `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`, and `G-TRACE`

No final program disposition may pass with blocked, failed, stale, not-applicable mandatory leaves, unresolved P0/P1 findings, an unverified login-Shell rollback, or a provider-host escape.

## 11. Implementation adjustment policy

- A — task refinement: task split/order/command/evidence detail may change without changing scope, public contracts, gates, thresholds, or safety boundaries.
- B — design/spec correction: an incorrect assumption within this approved scope requires affected work to pause, artifacts to be updated, evidence to become stale, and affected gates to rerun.
- C — material change: removing functionality, lowering a gate, changing a public contract, framework, required platform, permission boundary, destructive target, or external-write policy requires new user authority.

## 12. Rollout and rollback

Feature changes ship disabled behind typed capability/configuration boundaries until their own gate passes. Preview mode remains the default. Shell takeover remains explicit. Login-Shell installation is the final workstream and cannot be enabled merely because feature UI is complete.

Rollback order is: disable failing provider -> restore Explorer surfaces/work area -> restore prior login-Shell registry values -> retain diagnostics and quarantine unsafe configuration -> leave verified Explorer available. Uninstall never removes the last known recovery binary before registry restoration is verified.

## 13. Acceptance boundary

Completion means the listed Windows 10-style user workflows are implemented, independently testable, accessible, recoverable, and integrated into SuperDesktop. It does not mean undocumented Windows internals or every third-party extension are guaranteed compatible. Unsupported providers must fail independently and truthfully without compromising the desktop session.
