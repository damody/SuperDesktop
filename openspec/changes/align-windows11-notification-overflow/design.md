## Context

The current overflow content and `WindowOptions` correctly use logical pixels; GPUI scales the native window. The overlap occurs because monitor topology is captured before SuperDesktop registers its AppBar, so the stored work-area bottom does not yet exclude the owned taskbar. A rejected implementation multiplied `WindowOptions` by DPI again and produced an oversized surface; that evidence is stale.

## Goals / Non-Goals

**Goals:**

- Keep popup bounds logical while explicitly reserving the current taskbar row height and edge gap.
- Match Windows 11 panel density and interaction states without changing notification ordering or callbacks.
- Preserve Explorer-free ownership, focus-loss/Escape dismissal and high-contrast visibility.

**Non-Goals:**

- Do not change notification ingestion, promotion policy or visible-capacity settings.
- Do not implement notification-center history or call Explorer.

## Decisions

The popup uses a 344×bounded-grid logical content contract. `WindowOptions` remains logical because GPUI performs the native DPI conversion. Its right edge is inset 8 logical pixels and its bottom edge reserves `40 × taskbar rows + 8` logical pixels from the pre-AppBar work-area bottom. Bounds clamp in logical monitor coordinates.

`NotificationOverflowView` owns shared Windows 11 light/high-contrast tokens: 12px corner radius, subtle border, opaque fallback background, shadow, 48px cells, 24px icons and visible hover/focus/pressed states. Keyboard and UIA actions continue through the same typed callback.

Deterministic tests cover 96–480 DPI, negative monitor origins, small work areas and one-to-six grid rows. Headful evidence at 175% must show an independent popup above the taskbar and retain ordinary-client callbacks.

## Risks / Trade-offs

- **[GPUI/native scale mismatch]** → Assert logical bounds and their expected physical result at multiple DPI values; never pre-scale `WindowOptions` dimensions.
- **[Popup clips on a small monitor]** → Clamp physical bounds and reduce height to available work area.
- **[Visual state depends only on color]** → Keep focus geometry, border and UIA state in high contrast.

## Migration Plan

Update placement and view tokens, run focused/full gates, recapture the existing ordinary NotifyIcon fixture, then rebuild installers. Rollback is a source revert; no persisted data changes.

## Open Questions

None.
