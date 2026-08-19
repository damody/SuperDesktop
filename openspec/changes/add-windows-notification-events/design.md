## Context

SuperDesktop owns an Explorer-free notification center backed by `notification-area-host`. The host currently ingests documented NotifyIcon compatibility messages and stores their balloon content as bounded `OwnedNotification` history. It has no modern Windows Toast source; dismiss and clear mutate only its local registry. The supplied Windows center demonstrates real app notifications, while the archived center design explicitly excluded private toast-history ingestion.

Windows documents `Windows.UI.Notifications.Management.UserNotificationListener` for current Toast snapshots, `NotificationChanged` added/removed events, access status/request, and remove/clear operations. The current machine's non-mutating preflight reports `Allowed`. Windows exposes app label, native ID, creation time, and ToastGeneric text, but not arbitrary action-button invocation.

## Goals / Non-Goals

**Goals:**

- Ingest current Windows Toast notifications through documented WinRT events and authoritative snapshots.
- Preserve independent NotifyIcon history and merge both origins deterministically under existing bounds.
- Synchronize Windows-origin dismiss/clear before local publication.
- Publish truthful access/synchronization/failure state and recover after transient errors.
- Preserve Explorer-free ownership, accessibility, themes, privacy, and host isolation.

**Non-Goals:**

- Private notification databases, ShellExperienceHost integration, Toast action buttons, app activation, Focus sessions, Do Not Disturb, notification settings, groups, or app-logo stream decoding.

## Decisions

### Additive provider state

`NotificationSnapshot` gains defaulted `WindowsNotificationEventStatus` with `access`, `synchronized`, `last_change`, and bounded `reason`. `WindowsNotificationAccess` has `allowed`, `denied`, `unspecified`, and `unavailable`; `WindowsNotificationChange` has `added`, `removed`, and `none`. Validation requires non-empty bounded reasons only for unavailable state and forbids contradictory synchronized/non-allowed combinations. Old JSON defaults to unavailable.

### Scoped WinRT event source

`platform-win::windows_notification_events` owns `RoInitialize(RO_INIT_MULTITHREADED)`, `UserNotificationListener`, its event token, an atomic dirty flag, and last copied change metadata. Startup checks access; unspecified requests once and joins the documented async operation. Allowed access subscribes to `NotificationChanged`. Callback work is limited to atomics/owned numeric values and returns `Result<()>`; drop revokes the token before `RoUninitialize`.

Alternative polling-only and private-database designs are rejected for event latency and undocumented coupling respectively.

### Authoritative conversion and reconciliation

The adapter joins `GetNotificationsAsync(NotificationKinds::Toast)`, caps native items before conversion, and processes each item independently. It obtains App display name, native ID, creation time, ToastGeneric binding, and text elements. First text becomes title; the bounded join of later text becomes body. Empty/unparseable items are skipped and counted. Creation time converts from Windows 1601 100-ns ticks to Unix milliseconds with saturation checks.

The host reconciles on startup, dirty events, and a 5-second authoritative interval before Snapshot/Health. Event storms coalesce. A successful result replaces only `windows:` notifications, sorts newest-first with stable ID tie-break, and increments generation only on content/state change. Failure keeps the last good Windows subset but reports unavailable. Recovery restores allowed/synchronized state.

### Identity and privacy

Native notification identity is `windows:<u32>`. `IconKey.client_id` is fixed `windows-events`; `icon_id` is a stable 32-bit FNV hash of AppUserModelId, never the raw AUMID. The native ID is parsed only after the exact prefix and decimal bounds validate. No AUMID/native notification content appears in traces or committed live reports.

### Windows mutation ordering

`NativeCompatibilityRegistry` intercepts dismiss/clear. Expected generation is validated first. Windows-origin dismiss revalidates the native ID against the current listener snapshot, calls `RemoveNotification`, then reconciles and confirms absence. Clear calls `ClearNotifications` only when Windows-origin items exist, then confirms an empty Windows subset. A failure returns Rejected and preserves the prior local snapshot. NotifyIcon-only dismiss remains local. The live test never calls clear and removes only its controlled Toast.

### UI state without fake actions

The existing card list renders converted notifications. A localized provider banner appears for denied, unspecified, unavailable, or synchronizing state. Windows and NotifyIcon notifications coexist. Empty copy distinguishes no current notification from inaccessible Windows events. Existing dismiss/Delete/Clear pointer-keyboard-UIA parity remains; no unsupported Toast action button is painted.

## Gates and evidence

- **G-WNE-PROTOCOL:** additive JSON, access/state validation, bounds and round trips pass.
- **G-WNE-RUNDOWN:** callback is no-unwind, event token revoked once, apartment teardown ordered.
- **G-WNE-SNAPSHOT:** malformed item isolation, 100-item cap, dedupe/order and recovery pass.
- **G-WNE-MUTATION:** stale/foreign IDs never remove; Windows remove/clear precede and are confirmed before local state changes.
- **G-WNE-ACCESS:** allowed/denied/unspecified/unavailable states are truthful; no repeated prompts or Settings delegation.
- **G-WNE-UI:** provider banner, coexistence, empty state, keyboard/UIA, themes and scroll pass.
- **G-WNE-PRIVACY:** evidence contains no raw AUMID, native ID, app label, title or body.
- **G-WNE-LIVE:** access and current count are redacted; controlled event/dismiss passes or is evidence-backed not-applicable.

Each focused/full command and live/headful procedure is hashed in the change evidence indexes. Identifier-bearing screenshots remain uncommitted.

## Risks / Trade-offs

- **[Access/package identity unavailable on another deployment]** → Publish unavailable/denied status and retain NotifyIcon functionality; do not claim coverage.
- **[One malformed Toast binding throws]** → Per-item conversion boundary skips only that item.
- **[Event callback races shutdown]** → Atomics only, revoke token before apartment teardown, and ignore late dirty state.
- **[External notification changes between validation and remove]** → Native u32 ID is freshly present-checked and post-operation absence is confirmed; no fallback ID is used.
- **[Listener APIs expose no buttons]** → UI remains text/dismiss only and labels no unsupported action.

## Migration Plan

1. Land additive protocol status and validation.
2. Land WinRT source and read-only live probe.
3. Integrate host reconciliation while leaving Windows mutation disabled.
4. Add exact remove/clear confirmation and UI provider state.
5. Run controlled event, recovery, theme, privacy and full quality gates.

Rollback removes the source and additive producer use; old payload defaults and NotifyIcon behavior remain. No persistent schema or notification setting is migrated.

## Planning adjustments

- **A — task refinement:** task ordering, commands, owners, or splits may change without scope/contract/gate/evidence changes.
- **B — design/spec correction:** in-scope API corrections pause affected work and update artifacts/tasks/evidence before revalidation.
- **C — material change:** private APIs, packaging migration, permission/settings mutation, app activation, button actions, weakened gates, or new external writes require user approval.

## Open Questions

None. App-logo decoding, Toast actions, Focus/Do Not Disturb, and packaging-specific capability declarations remain separate changes if live evidence later proves they are necessary.
