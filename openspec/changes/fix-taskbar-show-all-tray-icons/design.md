## Context

`TaskbarView` derives `has_overflow` from nodes placed in `NotificationPlacement::Overflow` and uses it for both 32 DIP reservation and chevron rendering. `NotificationAreaModel` fills up to five visible slots, so zero-to-five ordinary icons produce no chevron. Activation filters the node snapshot back to overflow only, while the composition root rejects empty lists. Production already owns a separate DPI-aware overflow popup with typed actions and focus-loss dismissal.

## Goals / Non-Goals

**Goals:**

- Keep an up-chevron at a stable position in every taskbar notification area.
- Show a complete point-in-time snapshot of all current notification-area icons.
- Show a truthful empty state when no icons are registered.
- Preserve pointer/keyboard/UIA parity and theme visibility.

**Non-Goals:**

- Change icon ingestion, visible/overflow placement, status controls, popup geometry, or Explorer-free ownership.

## Decisions

### Unconditional stable control

Replace conditional `has_overflow` reservation/rendering with an unconditional 32 DIP control. Use localized labeling and `tokens.text` for the SVG instead of a hard-coded dark color. Lowering capacity or changing placement was rejected because it does not guarantee a stable affordance and changes unrelated policy.

### Single all-node snapshot

Pointer and keyboard handlers call `accessible_nodes()` once and forward the complete vector without filtering placement. The independent composition popup remains the only production surface; inline overflow remains disabled. This avoids stale divergence between visible and overflow collections and includes each registered icon exactly once.

### Truthful empty popup

Remove the composition root's empty-list early return. Existing geometry already allocates one minimum grid row for zero icons. `NotificationOverflowView` displays a localized empty label when `nodes.is_empty()` and otherwise preserves the existing icon grid/actions. Network, volume, input, and clock are excluded because they are dedicated system-status controls rather than notification registrations.

## Risks / Trade-offs

- [Risk] Stable reservation slightly reduces task-button width. → Mitigation: the existing adaptive-width calculation already consumes notification reserved width and the change is a fixed 32 DIP.
- [Risk] Visible icons are duplicated inside show-all. → Mitigation: duplication is intentional in the popup; the vector is keyed and contains each registered icon once.
- [Risk] Empty popup is mistaken for provider failure. → Mitigation: label it as no current tray icons, while provider availability remains modeled separately.
- [Risk] Theme glyph disappears. → Mitigation: use `tokens.text` and capture dark/high-contrast evidence.

## Migration Plan

No data migration is required. Update taskbar rendering/callbacks, then empty popup admission/content and focused/headful evidence. Rollback is a source revert.

## Open Questions

None. The user requested autonomous correction.
