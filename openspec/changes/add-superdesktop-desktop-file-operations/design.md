## Context

`desktop-ui` currently exposes rename, delete/recycle, refresh, and transfer as unavailable deferred actions. Item identity and watcher reconciliation already exist. This change adds typed operation planning, filesystem execution, progress/cancellation, and layout ordering while keeping Win32 calls in `platform-win`.

## Goals / Non-Goals

**Goals:** Support rename, explicit refresh, recycle/delete, copy/move, collision policy, sorting, alignment, position persistence, cancellation, and post-operation reconciliation.

**Non-Goals:** Implement shell extension menus, cloud-provider hydration, privilege escalation, or cross-session operations.

## Decisions

1. Add a platform-neutral operation state machine to `desktop-ui`; it owns validation, correlation, progress, cancellation, and terminal outcomes.
2. Add filesystem and recycle adapters to `platform-win`. Destructive permanent delete requires an explicit policy; normal desktop delete routes to the recycle bin.
3. Treat drag inside the desktop as layout repositioning unless a transfer source/destination is explicitly identified. Cross-root drag defaults to copy; same-root defaults to move, with modifier override.
4. Use deterministic collision policies (`fail`, `replace`, `rename`) and never silently overwrite.
5. Every terminal operation schedules namespace refresh; watcher deltas are hints and stable identity restores selection after reconciliation.

## Risks / Trade-offs

- [Partial multi-item failure] → Return per-item outcomes and refresh from source state.
- [Recycle Bin API behavior varies] → Wrap it in a typed adapter and test non-destructive fixtures separately from permanent deletion.
- [Copy cancellation is cooperative] → Check between chunks and remove only the incomplete destination created by the operation.
- [External mutation races] → Revalidate paths immediately before effects and reconcile after terminal state.

## Migration Plan

Replace `DeferredUnavailable` emissions for operations as UI wiring is added. Existing activation behavior remains unchanged. Removing the new modules restores M0 behavior without migrating user files.

## Open Questions

None.
