## Context

M0 tracks, groups, pins, activates, minimizes, and restores windows. It currently opens a simple multi-window selection and lacks hover previews, Jump Lists, progress overlays, and durable advanced preferences.

## Goals / Non-Goals

**Goals:** Add bounded live-preview requests, grouped thumbnail flyouts, close/activate actions, Jump Lists, progress/attention overlays, recent/frequent destinations, and stable persistence.

**Non-Goals:** Mirror undocumented taskband storage, inject into applications, or promise previews when DWM composition/capture is unavailable.

## Decisions

1. Keep `WindowId` as the authority and revalidate native HWND immediately before preview or action.
2. Use DWM composition capability probing and a 400 ms hover delay; preview becomes explicitly unavailable rather than showing stale pixels.
3. Represent Jump Lists as sanitized `CommandDescriptor` groups supplied through the provider boundary. Built-in pin/unpin/close commands remain local.
4. Store pin order and display preferences through versioned settings snapshots, never Windows taskband binary data.
5. Model progress and attention independently so failure/paused/normal overlays do not erase attention state.

## Risks / Trade-offs

- [DWM preview unavailable] → Show title/icon fallback and keep activation/close functional.
- [Window disappears during flyout] → Revalidate identity and reconcile the group before action.
- [Unbounded recent destinations] → Cap, deduplicate, and sanitize descriptors.
- [Preference drift] → Reconcile persisted IDs against current pins while preserving relative order.

## Migration Plan

Add advanced state beside the existing group/interaction models, then have GPUI overlays consume it. Existing single-click behavior remains unchanged.

## Open Questions

None.
