# System Flyout and Taskbar Geometry Design

## Intent

Remove the most visible Windows 11 proportion defect in the owned shell: input, network/power, volume, and calendar flyouts currently reserve the taskbar twice in Explorer-free shell mode, leaving an extra taskbar-row-sized gap. Extend UTIT so actual HWND geometry, not only screenshots and Rust constants, blocks regressions.

## Chosen design

System flyout placement receives the explicit runtime mode already owned by `run(shell, ...)`. In preview mode, SuperDesktop's taskbar is positioned immediately above `MonitorRecord.work_area.bottom`; the popup bottom is therefore `work_area.bottom - owned_taskbar_height - 8 DIP`. In replacement-shell mode, the taskbar is positioned at `MonitorRecord.bounds.bottom`; the popup bottom is `bounds.bottom - owned_taskbar_height - 8 DIP`. This avoids depending on whether Windows still reports a stale Explorer work-area reservation while Explorer is being suppressed.

The horizontal work span remains the monitor work area and the vertical top remains clamped to the work-area top. Preferred Windows 11 logical sizes stay 360 DIP for input/network/volume and 380 DIP for calendar. One-to-three taskbar rows, 96/144/168/216 DPI, negative monitor origins, constrained monitors, and preview/shell modes are covered by a pure geometry matrix.

## UTIT geometry admission

The existing `gui-system-status` case is extended instead of adding a second competing test program. For each owned flyout it records the taskbar and popup physical rectangles, DPI, width/height in DIP, bottom gap in DIP, monitor containment, and popup type. The case fails when:

- the popup is outside the source monitor;
- width differs from the preferred logical width by more than the DWM/non-client tolerance;
- the popup-to-taskbar gap is outside 2–16 DIP;
- the flyouts do not replace one another in one owned window slot;
- Explorer appears or recovery fails.

Screenshots remain supporting visual evidence, while the JSON geometry matrix is authoritative. Text rendering and dynamic content are deliberately excluded from pixel-perfect comparison because font antialiasing and notification contents are not deterministic.

## Alternatives considered

- Screenshot-only pixel diffs are visually direct but too sensitive to wallpaper, text rasterization, timestamps, and notification content.
- Constant-only Rust tests are stable but cannot expose GPUI scaling, DWM shadow, or actual HWND placement errors.
- Detecting taskbar reservation by comparing `bounds` and `work_area` is ambiguous during Explorer suppression because the system can retain stale work-area state. Explicit runtime mode is deterministic and matches the existing taskbar placement contract.

## Safety and failure handling

The change carries only numeric geometry and the existing typed shell-mode flag. It opens no system UI, sends no Explorer command, adds no privilege, and changes no persistence. Constrained monitors clamp to at least one logical pixel. UTIT continues to use an independent Explorer recovery watchdog and restores the original input profile.

## Verification and rollout

Blocking gates are `G-FLYOUT-GEOMETRY`, `G-DPI-MATRIX`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. Run focused Rust tests, Explorer-free system-status UTIT, full shell-parity, workspace tests, Clippy warnings-as-errors, architecture/source-boundary audits, release build, strict/detailed OpenSpec validation, screenshot inspection, and both NSIS builds with `--no-launch`. Rollback is a source revert; no migration exists. The OpenSpec change remains unarchived.
