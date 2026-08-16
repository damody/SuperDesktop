## Context

M0 deliberately renders no fake tray and reports the provider unavailable. The taskbar now has advanced overlay infrastructure and the provider protocol has notification icon DTOs. Notification clients must not share the GPUI failure boundary.

## Goals / Non-Goals

**Goals:** Add an isolated notification host, bounded client/icon registry, add/modify/delete/focus messages, visible/overflow layout, DPI-aware owned icons, event delivery, cleanup, restart snapshots, health, and accessibility.

**Non-Goals:** Load application code, emulate undocumented toolbar internals, retain raw HICON ownership across processes, or display synthetic icons for unavailable clients.

## Decisions

1. Add a dedicated `notification-area-host` binary/library so a tray fault cannot poison general search/menu providers.
2. Accept only owned RGBA icon data and stable `(client, icon)` identity. Native handles are copied at the boundary and never sent to GPUI.
3. Bound clients, icons, tooltip bytes, icon dimensions, pending events, and heartbeat lifetime.
4. Separate visible and overflow placement policy in `taskbar-ui`; the host owns registry truth, not rendering.
5. Every mutation increments a generation. Restart recovery uses a full snapshot; stale incremental generations are rejected.

## Risks / Trade-offs

- [Legacy clients use diverse message versions] → Capability-tag versions and fail unsupported messages per icon.
- [Clients exit without cleanup] → Associate leases with process/connection lifetime and remove owned icons.
- [Event storms] → Coalesce move/hover and protect activation/context events in a bounded queue.
- [Large icons exhaust memory] → Enforce dimensions, RGBA length, icon count, and total capacity.

## Migration Plan

Add the host and taskbar model behind an unavailable-by-default provider state, then enable it only after a successful handshake. Existing clock/status rendering remains independent.

## Open Questions

None.
