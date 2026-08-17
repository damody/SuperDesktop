## Context

The approved design is `docs/superpowers/specs/2026-08-18-desktop-marquee-start-menu-parity-design.md`. `SelectionModel` already understands a rubber-band result set, but `DesktopView` never creates one from pointer events. `StartModel` already owns search, persistence, focus, activation, and stale-result handling, while `StartView` renders every home/search item as one flat text list.

## Goals / Non-Goals

**Goals:** Windows-style live desktop marquee selection; current Windows 11 Start sections and placement; native application icons; safe collapsed power actions; preserved pointer, keyboard, IME, accessibility, persistence, and activation contracts.

**Non-Goals:** Pixel-perfect private Windows acrylic shaders, Phone Link integration, cloud recommendations, Start folders/groups, touch long-press, or changes to shell authority and registry ownership.

## Decisions

1. `DesktopView` owns transient gesture state because pointer positions and current rendered item hitboxes are view-local. The existing domain `SelectionModel` remains the geometry-independent contract and its tests are extended rather than bypassed.
2. Empty-space left down captures anchor, baseline selection, and Ctrl mode. Item left down stops propagation. Mouse movement normalizes all directions, uses inclusive rectangle intersection against actual item bounds, and recomputes selection from baseline plus current hits to avoid cumulative drift.
3. A three-logical-pixel threshold controls only rectangle visibility; selection still completes deterministically. Mouse-up and a move event without the left button both clear transient capture.
4. `StartModel` gains `StartPage::{Home, AllApps}` plus power-menu state and bounded slice methods. Search text overrides page rendering but does not destroy the chosen page.
5. Home renders 12 pins in six columns and six recommendations in two columns, matching the smaller current Windows 11 Start layout. All apps is alphabetical. Search rows retain ranked provider order.
6. App path icons reuse `platform-win::common::icon` and the taskbar BC7 render cache. Settings and unavailable icons use semantic visual tiles with full labels.
7. Start is centered horizontally, clamped to the work area, and placed 12 logical pixels above its bottom. Verification may request the owned Start surface without shell takeover through a bounded environment fixture.
8. Power actions remain behind the existing confirmation adapter. The footer exposes one Power button; the flyout is keyboard-addressable and dismisses on Escape.
9. Gate `G-DESKTOP-START-PARITY` requires automated interaction/model/view checks, headful marquee and Start captures at 175% DPI, UI Automation validation, strict OpenSpec validation, and both installers.

Evidence may refine task decomposition or commands without changing scope (A). Corrections to pointer geometry, Start layout, or icon routing inside scope require design/spec/task updates and reopening affected evidence (B). Changes to shell authority, power admission, platform, blocking thresholds, permissions, or external writes are material and require user approval (C).

## Risks / Trade-offs

- **[Root receives item pointer-down]** → Stop propagation on item left down and test that drag/click never starts marquee.
- **[Selection accumulates while dragging back]** → Recompute from the immutable pointer-down baseline on every move.
- **[Stuck marquee after capture loss]** → Cancel when a move reports no left button and on every mouse-up path.
- **[Large Start catalog blocks rendering]** → Discover with existing bounded limits and render bounded home/recommended sets; All apps remains capped by discovery.
- **[Missing icon]** → Preserve readable label and semantic fallback tile.
- **[Power action exposed too directly]** → Require explicit Power flyout selection and preserve confirmation.

## Migration Plan

Land gesture/model/view/composition changes atomically, run headful evidence, and rebuild installers. No persisted settings migration is required. Roll back by reverting the source changes; old snapshots remain readable.

## Open Questions

None.
