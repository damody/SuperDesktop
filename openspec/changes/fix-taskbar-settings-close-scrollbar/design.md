## Context

`TaskbarSettingsView` is rendered in a borderless GPUI popup (`titlebar: None`). It already receives a shared dismiss callback and supports Escape, but it has no pointer-visible dismissal affordance. Its root calls `overflow_y_scroll`; this GPUI build tracks wheel offsets and reserves optional scrollbar layout space but does not paint a native scrollbar. The surface must retain its Windows 11 settings appearance, localization, DPI behavior, and existing persistence model.

## Goals / Non-Goals

**Goals:**

- Provide a fixed, localized, keyboard/UIA-accessible close button.
- Provide a visible vertical scrollbar whenever settings content exceeds the viewport.
- Keep wheel scrolling, thumb position, pointer dragging, and accessibility percentage synchronized.
- Preserve light, dark, high-contrast, DPI, and content-width behavior.

**Non-Goals:**

- Add a native title bar, resize the settings window, or change any settings field or persistence rule.
- Redesign cards, section expansion, other popup chrome, or global GPUI scrolling.

## Decisions

### Fixed custom chrome around a tracked viewport

The view root becomes a fixed `relative` container. Its first child is the full-size scrollable content viewport, which retains existing padding/layout and adds `track_scroll` with a view-owned `ScrollHandle`. The fixed scrollbar and close button are later children so they paint above content and do not move with it. A native title bar was rejected because it changes the approved window geometry and visual language.

### Shared close dismissal

The 36 DIP top-right close button uses the existing `TaskbarSurfaceDismiss` callback, as Escape already does. It exposes `Role::Button`, localized `aria_label`, tab focus, visible focus, hover, pressed feedback, and a multiplication-sign glyph. This keeps slot clearing and window removal in the composition root authoritative.

### Scrollbar geometry and drag mapping

The 12 DIP track starts below fixed close chrome. For viewport height `V`, maximum scroll offset `M`, and track height `T`, total content height is `V + M`; thumb height is `clamp(T × V / (V + M), 48, T)`. Scroll progress is `clamp(-offset_y / M, 0, 1)`, and thumb top is `progress × (T - thumb_height)`. Dragging stores the pointer offset inside the thumb, maps the bounded thumb top back to progress, and calls `ScrollHandle::set_offset(0, -M × progress)`. Mouse-up clears drag state. If `M` or `T - thumb_height` is zero, the scrollbar is omitted and no division occurs.

The scrollbar uses existing settings foreground/border tokens for light/dark modes and the focus token for high contrast. It exposes `Role::ScrollBar`, localized labeling, and numeric range 0–100. `scrollbar_width` plus right content padding prevents overlap, but custom painting is retained because reservation alone paints no thumb.

## Risks / Trade-offs

- [Risk] Scroll geometry is unavailable on the first frame. → Mitigation: render an empty stable scrollbar placeholder until the tracked handle reports nonzero bounds/overflow; normal GPUI scroll invalidation supplies current geometry.
- [Risk] Pointer drag escapes the thumb bounds. → Mitigation: register window mouse events through a thumb canvas and retain drag state until global mouse-up, matching the vendored GPUI table example.
- [Risk] Fixed chrome covers narrow content. → Mitigation: reserve 20 DIP on the viewport's right edge and keep the existing bounded content width.
- [Risk] Section collapse reduces overflow while scrolled. → Mitigation: GPUI clamps the tracked offset to the new maximum; thumb geometry reads the updated handle each render.

## Migration Plan

No migration is required. Land view state/geometry tests, then chrome rendering and headful evidence. Rollback restores the single scrolling root; no persisted or public contract changes.

## Open Questions

None. The user requested autonomous resolution without confirmation.
