# Taskbar Preview Anchor and Focus Design

## Intent

Bring the owned taskbar preview closer to Windows 11 behavior: a preview opens above the task button that caused it, stays inside that monitor's work area, and a pointer hover never steals foreground activation or keyboard focus. SuperDesktop remains the sole owner of the popup and does not call Explorer or system taskbar UI.

## Chosen design

Introduce a typed preview-open source with `Hover` and `Click` variants. Both variants use the physical pointer position captured when the preview is admitted as the horizontal source anchor. Geometry converts the anchor and monitor work area through the monitor DPI, centers the content-sized popup over the anchor, and clamps it to the work-area edges. If a pointer position cannot be read, the existing monitor-center placement is the safe fallback.

`Hover` opens a non-focusing GPUI popup, does not activate its HWND, and does not assign the internal focus handle during rendering. This preserves the foreground fixture window while retaining pointer activation and close controls. `Click` keeps the current activation and keyboard-focus behavior so grouped task selection remains accessible with Left, Right, Enter, Delete, and Escape.

The placement helper is pure and returns logical geometry. It is tested at 96, 144, 168, and 216 DPI, negative monitor origins, one through four cards, and both work-area edges. Popup width remains content-sized and vertical placement remains immediately above the owned taskbar work-area boundary.

## Alternatives considered

- Keeping the popup screen-centered is stable but visually disconnects it from the source task.
- Always activating the popup makes keyboard handling simple but interrupts typing in the foreground application during ordinary hover.
- Delegating positioning or previews to Explorer would violate the replacement-shell boundary.

## Safety and failure handling

The anchor is numeric screen geometry only; no foreign HWND ownership is retained. DPI conversion uses the selected monitor record already used by the taskbar surface. Clamping guarantees the popup stays within the monitor work area even when it is wider than the available span. Failure to read the pointer degrades to deterministic monitor centering. Existing generation tokens, DWM RAII cleanup, leave grace, and Explorer recovery remain unchanged.

## Automated verification

The existing Explorer-free UTIT hover case records the foreground HWND before hovering and after popup admission, the source-task bounds, popup bounds, expected clamped center, actual center delta, monitor bounds, and app hash. It fails if hover changes the foreground window, the popup leaves the monitor, or horizontal placement differs by more than two physical pixels from the clamped expected position.

Focused Rust tests cover source focus policy and geometry. The final gate runs formatting, focused/workspace tests, Clippy warnings-as-errors, architecture/source checks, release build, strict/detailed OpenSpec validation, UTIT shell-parity, and both NSIS packages. The OpenSpec change remains unarchived.
