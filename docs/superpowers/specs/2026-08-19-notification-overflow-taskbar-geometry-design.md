# Notification Overflow and Taskbar Geometry Design

## Intent

Correct the owned hidden-notification-icons panel in replacement-shell mode. Its geometry always subtracts taskbar height from `work_area.bottom`, while the shell taskbar is anchored to `bounds.bottom`; a retained Explorer work-area reservation therefore creates an extra row-sized gap. Add a forced-overflow Explorer-free UTIT case with actual HWND geometry.

## Chosen design

Pass the existing `shell` flag to `notification_overflow_bounds` and its window options. Preview mode uses `work_area.bottom`; shell mode uses `bounds.bottom`. Both subtract `40 DIP × configured rows` and an 8 DIP Windows gap exactly once. The panel keeps a preferred 344 DIP width, 12 DIP padding, six 48 DIP cells per row, and up to six rows. Width/height clamp to the selected monitor with at least one logical pixel.

Pure tests cover 96/144/168/216 DPI, both modes, one-to-three rows, negative origins, stale work areas, icon counts 1/6/7/20/36+, and constrained monitors.

## UTIT admission

Promote the existing NotifyIcon compatibility script into the UTIT shell-parity catalog. Launch the documented NotifyIcon fixture with 20 icons, suppress Explorer with a recovery watchdog, open `Show hidden icons`, locate the owned `Hidden icons` dialog, and record popup/taskbar/monitor rectangles, DPI, logical width/height/gap, cell count, containment, callback trace, host recovery and binary/screenshot hashes.

The case fails if the panel is absent, not owned by SuperDesktop, differs from 344 DIP by more than 16 DIP including DWM frame, has a taskbar gap outside 2–16 DIP, leaves the monitor, exposes fewer than the forced hidden icons, calls Explorer/system tray UI, loses callbacks, or fails recovery.

## Alternatives and safety

Inferring shell ownership from `work_area` is ambiguous during Explorer suppression. Always using bounds breaks preview mode. Screenshot-only comparison is unstable. Explicit mode plus pure and real HWND measurement matches the established Start/system-flyout contracts.

The product change is numeric geometry only and uses no Explorer call, undocumented API, new privilege or persistence. Blocking gates are `G-OVERFLOW-GEOMETRY`, `G-DPI-MATRIX`, `G-NOTIFYICON`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. Run full local/Explorer-free/governance/release/package gates and keep the change unarchived.
