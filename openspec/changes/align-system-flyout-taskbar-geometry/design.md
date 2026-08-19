## Context

The owned system flyouts have Windows-style content, but their popup bottom is always computed as `work_area.bottom - taskbar_height - 8 DIP`. In shell mode the taskbar bottom is instead `bounds.bottom`, so a retained work-area reservation produces a full extra-row gap. The approved source design is `docs/superpowers/specs/2026-08-19-system-flyout-taskbar-geometry-design.md`.

## Goals / Non-Goals

**Goals:** Correct preview/shell geometry, keep preferred proportions across DPI/rows/monitor origins, and make real HWND geometry a blocking UTIT result.

**Non-Goals:** Pixel-diffing dynamic text, changing system-flyout content, changing taskbar row height, system UI delegation, or physical mixed-DPI certification on unavailable hardware.

## Decisions

### Explicit runtime mode

Pass the existing `shell` flag through composition and options into the pure geometry helper. Preview mode uses `work_area.bottom`; shell mode uses `bounds.bottom`. Both subtract the owned taskbar height and 8 DIP gap exactly once.

### Stable logical proportions

Preferred sizes remain 360×dynamic input, 360×228 network/power, 360×184 volume, and 380×520/720 calendar DIP. Width and height clamp to the selected monitor, with one conversion from physical monitor coordinates to logical GPUI coordinates.

### Hybrid UTIT gate

Pure Rust tests cover the synthetic DPI/mode/row matrix. The existing Explorer-free system-status case records UIA taskbar/popup physical rectangles, derives DIP values from effective DPI, asserts 2–16 DIP bottom gap and monitor containment, and retains screenshot/hash evidence.

### Gates and correction policy

Blocking gates are `G-FLYOUT-GEOMETRY`, `G-DPI-MATRIX`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. A-level changes may refine test mechanics. B-level corrections to mode geometry or tolerances reopen design/spec/tasks/evidence. C-level scope, privilege, delegation, external mutation, weaker gates, or new platform commitments require approval.

## Risks / Trade-offs

- **[Stale Windows work area]** -> explicit shell mode never infers ownership from work-area state.
- **[DWM shadow changes exact bounds]** -> use a bounded 2–16 DIP gap rather than zero-pixel equality.
- **[Small monitor clips popup]** -> preferred size clamps to at least one logical pixel.
- **[Dynamic content changes height]** -> kind-specific preferred height stays deterministic and actual bounds are recorded.

## Migration Plan

Land helper/tests, composition wiring, UTIT geometry evidence, full gates and packages. Rollback is a source revert; no data migration exists.

## Open Questions

None.
