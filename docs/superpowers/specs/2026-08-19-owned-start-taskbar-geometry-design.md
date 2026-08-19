# Owned Start and Taskbar Geometry Design

## Intent

Correct the next largest Windows 11 proportion defect: the owned Start window uses `work_area.bottom - 12 DIP` as its bottom edge without reserving SuperDesktop's own taskbar. In preview mode the Start window overlaps the owned taskbar; in replacement-shell mode stale Explorer work-area state makes the gap depend on history. Make actual Start/taskbar geometry an Explorer-free UTIT gate.

## Chosen design

Pass the existing runtime `shell` flag and configured taskbar row count into `start_window_geometry` and `start_options`. The taskbar anchor matches `taskbar_physical_geometry`: preview mode uses `work_area.bottom`, replacement-shell mode uses `bounds.bottom`. Start's available bottom is the matching anchor minus `40 DIP × rows` and a 12 DIP Windows gap. Preferred width remains 640 DIP and preferred height remains 720 DIP; both clamp to the monitor span above that bottom, with a minimum of one logical pixel.

This uses explicit runtime mode rather than inferring ownership from `bounds - work_area`, because Windows can retain an Explorer work-area reservation while Explorer is being suppressed. Horizontal centering remains within the selected work area. One through three rows, 96/144/168/216 DPI, negative monitor origins, stale work areas, and constrained monitors are pure-tested.

## UTIT Explorer-free admission

Upgrade the existing `gui-start` case rather than creating another runner. It gains a recovery watchdog, suppresses Explorer, launches `superdesktop-app --shell`, and records:

- taskbar and Start physical rectangles;
- effective DPI and logical Start width/height/gap;
- monitor containment and non-overlap;
- owned PID equality;
- system Start/Search host PID sets before/after;
- home, all-apps and power UIA semantics;
- screenshots, release-app hash, Explorer absence, and input recovery state.

The case fails if Start overlaps the taskbar, the visible gap is outside 4–20 DIP after DWM shadow, logical width differs from 640 DIP by more than 16 DIP, Start leaves its monitor, a new system Start/Search host appears, Explorer appears during capture, or recovery fails.

## Alternatives considered

- Reducing Start height avoids some overlap but fails across row counts and small monitors.
- Inferring shell mode from the current work area is ambiguous during Explorer suppression.
- Pixel-only screenshot comparison is unstable across text, icons, locale and antialiasing. The hybrid pure-geometry plus actual HWND/UIA gate is deterministic and still retains screenshots.

## Safety and rollout

The change carries numeric geometry and an existing mode flag only. It does not call Explorer, system Start, SearchHost, ShellExperienceHost, or undocumented APIs. The UTIT watchdog restores Explorer even after failure. Blocking gates are `G-START-GEOMETRY`, `G-DPI-MATRIX`, `G-OWNED-START`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. Run focused/workspace tests, Explorer-free shell-parity, Clippy, architecture/source-boundary audits, release, strict/detailed OpenSpec, screenshot inspection, and both NSIS builds with `--no-launch`. Keep the change unarchived.
