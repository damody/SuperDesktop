# Taskbar icon fidelity and preview clearance design

## Goal

Match the observable Windows Explorer taskbar behavior in two places:

1. taskbar application icons remain crisp at the monitor DPI instead of enlarging a 16 px or lossy-compressed source; and
2. a hover preview never covers the SuperDesktop taskbar, including preview mode where the native Explorer taskbar still owns the Windows work area.

The visible taskbar icon remains 24 DIP. This change does not redesign task buttons, preview cards, grouping, or animation.

## Icon pipeline

SuperDesktop selects one shared source size that is safe for every active monitor. The requested physical edge is `ceil(24 * max_monitor_dpi / 96)`, clamped to 32–64 px. A monitor with incomplete DPI data falls back to 96 DPI.

`platform-win` retrieves the closest high-quality source in this order:

1. a size-matched executable icon resource when an executable path is available;
2. the window's large icon;
3. the window's small icon variants and class icons;
4. the existing shell/file fallback.

Each owned `HICON` is destroyed after conversion. Borrowed window/class handles are never destroyed. Invalid paths, missing resources, protected processes, and short-lived windows remain recoverable fallbacks and are reported through the existing console diagnostics rather than panicking.

Icons at or below 64 px are uploaded as lossless BGRA pixels. BC7 remains available for larger raster assets, but not for small taskbar icons where block compression visibly damages edges and alpha detail. The renderer still draws at 24 DIP, allowing GPUI to downsample a sufficiently detailed source instead of upscaling a small source.

## Preview geometry

Preview placement uses the same effective taskbar mode and row count as the visible SuperDesktop taskbar.

- Owned-shell mode uses the monitor bounds bottom as the taskbar bottom.
- Explorer-compatible preview mode uses the Windows work-area bottom as the native taskbar top and places the SuperDesktop taskbar immediately above it.
- The SuperDesktop taskbar top is the selected bottom minus the DPI-scaled height for the configured 1–3 rows.
- The preview outer bounds end above that top with the normal popup gap and shadow allowance.

All calculations stay in monitor-local logical coordinates until the final GPUI bounds are created. The result is clamped to the selected monitor and supports mixed DPI, negative desktop origins, narrow monitors, and multi-row taskbars.

The taskbar mode and row count are captured when a hover is scheduled and are supplied again for immediate click previews. A delayed hover therefore cannot silently fall back to the Explorer work area formula.

## Failure handling and observability

Icon extraction failure preserves the prior fallback chain and never hides a task. Preview geometry with missing or invalid monitor DPI uses 96 DPI and a one-row minimum. These conditions produce truthful console diagnostics where the existing runtime already reports platform failures; they do not use `unwrap` or panic paths.

The runtime records the selected icon source edge and preview/taskbar geometry in test-visible state so automated checks can prove the source is not undersized and the popup does not intersect the taskbar.

## Verification gates

Blocking verification is:

- unit tests prove 96/144/192 DPI source sizing, 32–64 px clamping, large-first fallback order, and lossless upload for small icons;
- geometry tests prove the preview bottom is at or above the taskbar top for owned-shell and Explorer preview modes, 1–3 rows, mixed DPI, and negative-origin monitors;
- headful UTIT opens a real hover preview while Explorer is present and records `preview_bottom <= superdesktop_taskbar_top`;
- the focused headful case passes twice consecutively;
- workspace tests, formatting, Clippy with warnings denied, release build, and installer build pass.

## Rollback

The change is local to icon acquisition/upload and preview geometry parameters. Reverting the implementation commit restores the prior behavior without migrating user settings or persistent data.

## Plan correction policy

Task splitting, ordering, commands, and evidence paths may be refined without changing scope (A). A discovered Windows API or GPUI constraint may correct the design/spec within the approved behavior only after the affected artifacts and tests are updated (B). Reducing the fidelity or clearance gates, changing permissions, or expanding scope requires user approval (C).
