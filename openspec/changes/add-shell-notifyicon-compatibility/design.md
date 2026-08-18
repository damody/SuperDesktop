## Context

`notification-area-host` already owns a bounded registry and newline-delimited provider protocol, while SuperDesktop already renders registry snapshots and routes typed activate/context events. Ordinary applications instead call `Shell_NotifyIcon`, which targets Explorer's taskbar compatibility identity and passes process-local `NOTIFYICONDATA`/HICON/callback-window state. The boundary must therefore exist outside GPUI, be exclusive with Explorer, and copy every native input before publication.

## Goals / Non-Goals

**Goals:**

- Support documented add, modify, delete, set-focus and version negotiation semantics for bounded legacy icons.
- Bind each icon to current interactive session, live process and callback HWND identity.
- Copy icon pixels/tooltips into owned DTOs and route user events back through validated callback messages.
- Recover through documented taskbar-created re-registration after takeover or host restart.
- Preserve preview-mode non-interference and transactional Shell admission.

**Non-Goals:**

- Reimplement undocumented Explorer toolbar, notification-center history, promotion policy or arbitrary private Shell IPC.
- Accept raw pointers/handles across the host/app protocol or load client code into SuperDesktop.
- claim compatibility before controlled headful add/modify/callback/delete succeeds without Explorer.

## Decisions

### Exclusive native compatibility adapter

`platform-win` owns a small compatibility-window adapter and native structure parser; `notification-area-host` owns its lifetime and registry. Admission requires the existing controlled Shell capability and session owner lease. Preview mode never creates the class identity. A direct GPUI implementation was rejected because malformed client traffic or callback panics would become desktop-fatal.

### Bounded normalized ingress

The adapter normalizes supported `NOTIFYICONDATAW` sizes/versions into a new protocol DTO containing copied process/session/window identity, icon ID or GUID identity, callback message, state/version, bounded tooltip and owned icon pixels. Structures with invalid size, dead/wrong-session windows, oversized strings/icons or unsupported versions fail before registry mutation. HICON is rendered/copied and never transferred.

### Registry generations and callback delivery

Normalized NIM_ADD/MODIFY/DELETE/SETFOCUS/SETVERSION map to the existing registry using monotonic host/icon generations. User events are admitted by SuperDesktop, returned to the host, revalidate the client lease, then post the documented callback message. Exactly one terminal disposition is recorded; stale identities cannot receive callbacks.

### Recovery and teardown

The host broadcasts the documented `TaskbarCreated` recovery notification only after compatibility admission. Re-registration creates a new host generation and old icons remain cleared. Teardown first fences callbacks, destroys compatibility HWNDs, clears client leases and only then releases the apartment/thread. An authoritative timer removes dead clients.

### Evidence and corrections

Blocking gates are `G-NOTIFY-COMPAT`, `G-NOTIFY-ISOLATION`, `G-NOTIFY-A11Y`, `G-SHELL-NONINTERFERENCE` and `G-TRACE`. A-level refinements may split task mechanics. B-level corrections update design/spec/tasks and stale dependent evidence. Scope, compatibility promises, gates, permissions and destructive/external behavior are C-level and require user approval.

## Risks / Trade-offs

- **[Explorer identity collision]** → admit only under controlled Shell ownership and prove preview absence.
- **[Cross-process structure ambiguity]** → accept explicit supported sizes/versions and copy via fenced adapter tests; reject unknown layouts.
- **[Dead/reused HWND callback]** → bind PID/session/window generation and revalidate immediately before posting.
- **[Icon storms exhaust resources]** → coalesce modify events, cap clients/icons/events and preserve terminal events.
- **[Host crash loses registrations]** → clear state, restart boundedly and emit taskbar-created re-registration.

## Migration Plan

1. Extend contracts and deterministic native parsing fixtures.
2. Add exclusive compatibility window/lease and registry mapping behind Shell admission.
3. Wire callback delivery and recovery.
4. Integrate lifecycle health, UI/overflow and packaging.
5. Run Explorer-present non-interference and Explorer-free controlled client gates.

Rollback disables compatibility admission, destroys owned HWNDs and restores Explorer through the existing guardian path; registry files require no migration.

## Open Questions

No material question remains. Unsupported structure revisions and private protocols terminate as unavailable rather than expanding scope during apply.
