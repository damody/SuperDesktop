## Context

The owned Start menu is opened as a titlebar-free GPUI popup with `is_resizable: false`. Its geometry always selects 640 DIP width and at most 720 DIP height, anchored above the current taskbar. `StartSettings` stores model state but no geometry. Settings use a custom versioned JSON codec and an atomic revisioned store.

The resize implementation spans settings encoding, monitor/DPI geometry, GPUI native styles, window-bound observers, persistence write frequency, and physical restart behavior.

## Goals / Non-Goals

**Goals:**

- Resize Start from native window edges.
- Persist logical width/height without write storms.
- Restore size across close/reopen and process restart.
- Keep the resized menu fully within the active monitor for all supported DPI/taskbar modes.
- Preserve taskbar alignment anchoring and existing Start content behavior.

**Non-Goals:**

- Persisting window position.
- Adding a dedicated sizing settings page.
- Synchronizing geometry across users or machines.
- Allowing Start outside the current monitor work area.

## Decisions

### Persist optional logical dimensions in StartSettings

`width_dip` and `height_dip` are optional positive integers. Missing fields preserve the current default. The existing schema version remains valid because the fields are additive and the codec already owns explicit compatibility behavior. Logical DIP values are preferred to physical pixels because they preserve perceived size across DPI changes.

### Normalize against the current monitor at every open and save

The default is 640×720 DIP, minimum 420×360 DIP, and maximum is the active work area after taskbar/gap/margins. Effective minima collapse to available space on constrained monitors. Restore and live persistence share one pure normalizer, preventing malformed settings or monitor changes from creating invisible geometry.

### Use native resize edges and derived positioning

The popup becomes resizable but remains titlebar-free and immovable. Physical evidence showed that the vendored Windows GPUI backend ignores `is_resizable` for `WindowKind::PopUp`; the backend therefore adds only `WS_THICKFRAME` when a popup explicitly requests resizing. Non-resizable popups remain style-identical, and resizable popups do not gain maximize, caption, or app-window styles. Reopen derives left/top from saved size, current alignment, monitor, taskbar rows, and preview/shell anchor. Position is not persisted.

### Debounce atomic persistence and flush terminal paths

`StartView` observes window bounds, ignores its initial/unchanged size, and schedules one 250ms generation-fenced callback for the latest size. Deactivation and explicit taskbar-toggle close flush the latest observed size immediately. The app callback saves through the existing `SettingsStore`, updating shared in-memory settings only after success. Trace markers distinguish saved, unchanged, and failed states.

### Evidence adjustment classes

A-level refinements may change debounce implementation or test mechanics without changing limits or persistence semantics. B-level corrections include dimension fields, limits, DPI unit, anchoring, flush terminals, or schema behavior and require artifact revalidation. Persisting position, adding synchronization, changing permissions, or weakening containment is C-level and requires user approval.

## Risks / Trade-offs

- **Risk: resize events create excessive writes** → Generation-fenced 250ms debounce plus terminal flush.
- **Risk: backend change affects other popups** → Gate `WS_THICKFRAME` strictly on `PopUp + is_resizable` and add positive/negative style tests.
- **Risk: close races the debounce** → Flush the latest observed size before explicit removal and on deactivation.
- **Risk: saved geometry no longer fits a monitor** → Normalize every open against the current work area.
- **Risk: small content clips** → Enforce 420×360 minimum when space permits; existing content already uses wrapping and vertical scrolling.

## Migration Plan

Existing settings load with missing optional dimensions and open at the current default. The first completed resize writes both fields. Rollback ignores the additional JSON fields through existing compatibility behavior and reverts to fixed geometry.

## Open Questions

None. Size is per user and shared across monitors, then clamped for the active monitor.
