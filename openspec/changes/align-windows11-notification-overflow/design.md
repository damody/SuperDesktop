## Context

The current overflow content is 336 logical pixels wide, but `WindowOptions` also receives 336 without DPI conversion and uses an origin divided by scale. Other corrected product popups use physical outer bounds while GPUI content remains logical. The current-host 175% capture consequently shows an undersized panel overlapping the two-row taskbar.

## Goals / Non-Goals

**Goals:**

- Use one explicit logical-to-physical conversion for popup size, origin and edge gap.
- Match Windows 11 panel density and interaction states without changing notification ordering or callbacks.
- Preserve Explorer-free ownership, focus-loss/Escape dismissal and high-contrast visibility.

**Non-Goals:**

- Do not change notification ingestion, promotion policy or visible-capacity settings.
- Do not implement notification-center history or call Explorer.

## Decisions

The popup uses a 344×bounded-grid logical content contract, converted to physical outer bounds using monitor DPI. Its right edge is inset 8 logical pixels and its bottom edge is 8 logical pixels above the work-area bottom. Physical bounds clamp to the monitor work area. This follows the working taskbar-settings DPI model instead of relying on GPUI to scale native bounds implicitly.

`NotificationOverflowView` owns shared Windows 11 light/high-contrast tokens: 12px corner radius, subtle border, opaque fallback background, shadow, 48px cells, 24px icons and visible hover/focus/pressed states. Keyboard and UIA actions continue through the same typed callback.

Deterministic tests cover 96–480 DPI, negative monitor origins, small work areas and one-to-six grid rows. Headful evidence at 175% must show an independent popup above the taskbar and retain ordinary-client callbacks.

## Risks / Trade-offs

- **[GPUI/native scale mismatch]** → Assert exact physical width, height and work-area containment at multiple DPI values.
- **[Popup clips on a small monitor]** → Clamp physical bounds and reduce height to available work area.
- **[Visual state depends only on color]** → Keep focus geometry, border and UIA state in high contrast.

## Migration Plan

Update placement and view tokens, run focused/full gates, recapture the existing ordinary NotifyIcon fixture, then rebuild installers. Rollback is a source revert; no persisted data changes.

## Open Questions

None.
