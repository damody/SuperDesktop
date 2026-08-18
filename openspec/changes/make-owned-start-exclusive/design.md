## Context

`StartView` already implements the owned Windows 11-style Start surface, including Search, Pinned, Recommended, All apps, Settings, Account and Power. The taskbar callback currently takes two different paths: Shell mode or a verification environment variable opens `StartView`, while ordinary preview mode returns after `invoke_start_host_controlled()`. This makes the safe development path exercise a different product than Shell mode and leaves an Explorer dependency in the visible shell contract.

The approved source design is `docs/superpowers/specs/2026-08-18-explorer-free-owned-shell-design.md`. This change owns only the exclusive Start cutover and its direct desktop-marquee regression gate. Status/IME, legacy tray compatibility and Explorer termination remain separate changes.

## Goals / Non-Goals

**Goals:**

- Route every product Start action to the same owned GPUI Start surface.
- Remove system Start-host invocation from the SuperDesktop composition path.
- Preserve all current owned Start sections, icons, persistence, input, activation and placement.
- Fail visibly inside the owned surface when a search or activation provider is unavailable; never delegate rendering to Explorer.
- Reverify desktop marquee selection and fixed-entry pointer routing after the cutover.

**Non-Goals:**

- Do not remove the historical platform capability probe or its archived evidence.
- Do not add real network, volume, power, input-language or legacy tray providers.
- Do not terminate Explorer in this change.
- Do not change Start visual density beyond defects required to pass the existing 175% DPI evidence.

## Decisions

### 1. One callback path for every execution mode

The Start callback always toggles the existing `start_window_for_taskbar` window and constructs `StartView`. The `shell` flag and `SUPERDESKTOP_VERIFICATION_OWNED_START` no longer select the renderer. The environment variable may remain accepted temporarily by scripts, but it has no product-routing effect.

This is preferred over keeping a preview fallback because preview is the safest and most frequent development entry; it must exercise the eventual owned Shell behavior.

### 2. Keep the platform Start probe outside product composition

`platform-win::monitor_dpi_start` remains available for historical capability and reference-profile tests. Product source guards prohibit `invoke_start_host_controlled` and related system-host calls from `surface_runtime.rs` and the taskbar Start action.

Removing the adapter entirely would invalidate earlier platform evidence and is unnecessary. Keeping it unreachable from product composition preserves lineage without preserving delegation.

### 3. Provider failure stays inside Start

Application discovery and search continue through bounded owned catalogs/provider DTOs. Missing or failed results render the current truthful Start error/unavailable state. No error branch invokes Explorer, SearchHost or a system Start process.

### 4. Preserve a single-window toggle and focus contract

Repeated Start invocation closes the existing owned window exactly once. A new invocation centers and clamps the window above the work area, activates Search and restores existing Escape, arrows, Enter, IME and UIA behavior. Closing clears the stored window handle so a later invocation cannot target a stale window.

### 5. Desktop marquee is a blocking non-regression

The cutover changes taskbar/window composition and must not consume desktop pointer input or cover the desktop surface. Headful evidence therefore includes a reverse marquee at host 175% DPI with at least two selected UIA items and the fixed SuperExplorer entry remaining pointer-addressable.

## Blocking gates

- `G-OWNED-START`: Start product source contains no system-host invocation and all execution modes open `StartView`.
- `G-START-INPUT`: pointer, keyboard, UIA, IME, placement, toggle and dismissal paths pass.
- `G-DESKTOP-MARQUEE`: live normal/reverse selection and fixed-entry pointer routing pass after integration.
- `G-TRACE`: every task has unique indexed evidence and strict validation passes.

## Adjustment policy

- **A — task refinement:** tests, commands, evidence file names or leaf splitting may change without altering requirements, gates or public behavior.
- **B — design/spec correction:** an incorrect implementation assumption within the approved owned-Start scope requires design/spec/tasks updates, reopening affected leaves and marking dependent evidence stale.
- **C — material change:** restoring any system Start delegation, changing the Explorer-free scope, weakening a blocking gate, adding external writes or changing permission boundaries requires user approval.

No adjustment may silently convert missing headful evidence into a pass.

## Risks / Trade-offs

- **[Preview no longer mirrors the installed Windows Start]** → This is intentional; preview must mirror SuperDesktop Shell behavior.
- **[Owned provider catalog is incomplete]** → Show truthful unavailable/empty states and improve discovery within this change; never delegate presentation.
- **[Two Start windows open during rapid input]** → Retain one shared window slot and test repeated invocation/dismissal ordering.
- **[Start steals desktop pointer routing]** → Capture desktop marquee and fixed-entry interaction after Start closes.
- **[Historical Start capability scripts become confusing]** → Document that they validate platform lineage only, not the product renderer.

## Migration Plan

1. Add source and model tests that fail while the preview delegation exists.
2. Remove the mode/environment branch and route all callbacks to `StartView`.
3. Run focused and complete workspace gates.
4. Capture owned Start Home/All apps and desktop marquee on the reference host.
5. Rebuild standalone and combined installers without launching them.

Rollback is a source revert to the prior callback. No settings, registry or on-disk Start data migration is required.

## Open Questions

None.
