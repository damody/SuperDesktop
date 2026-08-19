# Taskbar show-all tray icons design

## Problem

The notification-area up-chevron is rendered only when `NotificationAreaModel` places at least one icon in its overflow partition. The model fills up to five visible slots first, so the chevron often disappears even though notification icons exist. Its callback also sends only overflow-partition nodes, so it cannot act as the requested entry point for all current tray icons. The glyph uses a hard-coded dark color that is not reliable in dark or high-contrast themes.

## Design

Always reserve and render the 32 DIP notification overflow control in the taskbar's right cluster. Give it localized Button semantics, pointer/keyboard activation, existing hover/pressed/focus tokens, the up-chevron asset, and the current theme text color.

On pointer or keyboard activation, take one current `accessible_nodes()` snapshot and send every notification-area node—both visible and overflow placements—to the existing `notification_overflow` callback. The composition root retains its single-popup toggle behavior, geometry, focus-loss dismissal, and typed notification actions. It no longer rejects an empty node list: the existing minimum one-row geometry opens a truthful empty surface. `NotificationOverflowView` renders a localized empty-state label when no nodes are registered, while non-empty snapshots retain the six-column icon grid.

The inline overflow branch remains disabled because production owns a separate popup window. `NotificationAreaModel` placement and visible-capacity policy remain unchanged; the new control is a stable show-all entry point rather than a promotion-policy change.

## Alternatives

Reducing visible capacity or forcing every non-pinned icon into overflow was rejected because the arrow would still disappear for empty/all-visible states and would silently change icon promotion policy. Rendering the arrow only when the provider reports at least one icon was rejected because the affordance would still shift in and out of the right cluster. Adding network, volume, clock, or input controls to this popup was rejected because those system-status controls already have dedicated taskbar/flyout contracts and are not notification-area registrations.

## Verification

Pure/source-contract tests cover unconditional reserved width and rendering, all-node snapshot forwarding, pointer/keyboard parity, theme-aware chevron color, empty-state content, and removal of the runtime empty-list rejection. Existing notification ordering/action tests must remain unchanged. Headful validation opens the taskbar with no registered icons and with a controlled mixture of visible/overflow icons, confirms the chevron is UIA-reachable in both states, invokes it, verifies the empty state or complete icon set, and captures light/dark/high-contrast evidence above the taskbar.

## Scope

This change affects only the owned notification-area chevron and popup admission/content. It does not change notification ingestion, icon promotion, system-status controls, popup geometry, taskbar settings, or Explorer-free ownership.
