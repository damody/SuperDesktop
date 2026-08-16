## Context

Windows exposes a documented `IVirtualDesktopManager` for querying a window's desktop and moving a window to a known desktop ID. Enumeration, creation, removal, and switching do not have a stable documented Win32 contract across supported builds. SuperDesktop must capability-gate those operations.

## Goals / Non-Goals

**Goals:** Add documented window-desktop query/move support, capability discovery, owned task-view state, keyboard/accessibility interactions, stale reconciliation, and explicit unavailable behavior for unsupported mutations.

**Non-Goals:** Bind undocumented internal COM interfaces, fabricate desktop identities, or claim create/remove/switch support when the adapter cannot prove it.

## Decisions

1. Implement the documented `IVirtualDesktopManager` boundary in `platform-win` and expose owned u128 desktop IDs.
2. Split capabilities into query, move-window, enumerate, switch, create, remove, and rename. Each UI command checks its capability.
3. Use provider snapshots for enumeration-capable adapters; snapshot generation and stable IDs suppress stale state.
4. Task View remains useful when only query/move is available by grouping tracked windows by observed desktop IDs and explaining unavailable mutations.
5. Revalidate HWND ownership/liveness immediately before query or move and never retain COM pointers in UI state.

## Risks / Trade-offs

- [Documented API cannot enumerate desktops] → Expose partial capability honestly and keep unsupported controls disabled.
- [OS update changes optional adapter] → Runtime probe, version tag, and fail closed.
- [Window moves concurrently] → Requery after effects and authoritative snapshot wins.
- [Pinned windows appear across desktops] → Preserve adapter-reported membership and label shared visibility explicitly.

## Migration Plan

Add the platform service and task-view model disabled by default, then enable capabilities returned by runtime admission. Removing the modules leaves existing taskbar behavior unchanged.

## Open Questions

None.
