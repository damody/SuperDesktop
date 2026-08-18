## Context

The isolated status host and typed `SystemStatusAction` routes are complete, and `SystemFlyoutView` already owns input, audio, network/power, and calendar popup windows. Presentation remains a single light palette with generic English labels, fixed 380px geometry, inconsistent cards, a stepped volume hit map, and no dark/high-contrast contract. The approved sources are `docs/superpowers/specs/2026-08-18-explorer-free-windows11-gui-convergence-design.md` and `docs/superpowers/specs/2026-08-18-windows11-system-flyouts-design.md`.

## Goals / Non-Goals

**Goals:**

- Centralize light, dark, and high-contrast flyout chrome.
- Align four owned flyouts with Windows 11 build 26200 density, typography, state geometry, and Traditional Chinese/English presentation.
- Clamp popup bounds above owned taskbar rows on positive- and negative-origin monitors.
- Preserve typed actions, observed command completion, independent unavailable states, UIA/keyboard behavior, and Explorer-free ownership.

**Non-Goals:**

- Do not add Wi-Fi, Bluetooth, brightness, media, notification-history, or calendar-event mutation.
- Do not use undocumented shell interfaces, system Quick Settings, notification center, Settings URI handlers, or Explorer-owned windows.
- Do not alter status protocol DTOs, host restart policy, command terminal semantics, or taskbar AppBar reservation.

## Decisions

### Shared presentation inputs

`SystemFlyoutChromeTokens` is a presentation-only Copy value selected from explicit theme and high-contrast inputs. It contains panel/card/border/text/accent/hover/pressed/focus/unavailable colors and shared geometry. `SystemFlyoutPresentation` carries theme and locale into the view without changing provider DTOs.

Passing explicit presentation state is preferred over environment reads inside `taskbar-ui`, because deterministic tests and headful fixtures must render the same model independent of process-global state.

### Surface-specific composition over a generic data-driven renderer

Input, volume, network/power, and calendar keep separate render branches while consuming shared helpers for chrome, localization, state, and unavailable rows. Their interaction semantics and content structures differ enough that a generic card schema would hide typed actions and complicate accessibility.

### Truthful actions only

Input rows emit `ActivateInputProfile`; volume controls emit `SetVolume` and `SetMute`. Network, power, and calendar are informational because the current provider contract has no mutation for them. No inert chevron, fake toggle, or Settings link is rendered.

### Deterministic geometry

`system_flyout_geometry` computes kind-specific logical width/height from profile count, monitor work area, DPI, and owned taskbar rows. The result is right-aligned with an 8px gap, placed above taskbar rows, and clamped to the full logical work area. Window options only translate this pure result into GPUI bounds.

### Localized visible text, full accessible identity

Traditional Chinese and English strings are selected from the existing locale signal. Compact visible language tags are bounded, while full profile display name and provider identity remain in UIA names. Calendar weekday/month labels follow the selected presentation locale.

## Blocking Gates

- `G-SYSTEM-FLYOUT-CHROME`: token, state, geometry, light/dark/high-contrast, and 175% capture evidence.
- `G-SYSTEM-FLYOUT-A11Y`: roles, names, values, keyboard activation, Escape, focus, and unavailable-state evidence.
- `G-SYSTEM-FLYOUT-TRUTH`: typed-action and no-fake-action/source evidence.
- `G-SHELL-NONINTERFERENCE`: production-source and Explorer-free process evidence.
- `G-TRACE`: detailed task validation, strict OpenSpec validation, and unique evidence linkage.
- `G-PACKAGE`: release binary and both NSIS installers built without launch with recorded hashes.

## Adjustment Policy

- **A — task refinement:** task ordering, helper extraction, fixture values, command spelling, or evidence paths may change without changing scope, requirements, gates, or public contracts.
- **B — design/spec correction:** unreasonable geometry or documented presentation assumptions inside approved scope require updating design/spec/tasks, reopening affected leaves, and invalidating dependent evidence.
- **C — material change:** new provider mutations, undocumented interfaces, delegated shell UI, permissions, scope, public contracts, blocking gates, or required evidence need explicit approval.

## Risks / Trade-offs

- **[Independent flyouts differ from the combined stock Quick Settings panel]** → Match Windows 11 chrome and interaction density while preserving truthful provider scope; schedule new provider capabilities separately.
- **[Long profile names overflow]** → Bound visible labels with ellipsis and preserve complete UIA identity.
- **[Small work areas clip calendar]** → Clamp pure geometry and keep essential content inside the visible popup.
- **[High contrast cannot rely on accent fill]** → Add non-color focus/selection border geometry.
- **[Theme changes while a popup is open]** → Reconciliation updates presentation inputs or the popup is recreated on next invocation; no provider value is cached as presentation state.

## Migration Plan

1. Add presentation/localization/geometry helpers and deterministic tests.
2. Restyle the four surfaces without changing typed action contracts.
3. Wire theme, locale, DPI, and taskbar-row inputs through app composition.
4. Run automated, headful, Explorer-free, traceability, release, and packaging gates.

Rollback is a source revert. No settings or protocol migration is required, and rollback MUST NOT restore fixed provider values or system-shell delegation.

## Open Questions

None.
