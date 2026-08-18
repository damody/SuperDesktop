# Windows 11 Owned Taskbar Settings Layout Design

## Scope

Align SuperDesktop's owned taskbar settings window with the Windows 11 Settings layout while preserving every existing typed setting action and avoiding Explorer, `ms-settings`, and inbox Settings UI invocation.

## Decisions

1. Treat all GPUI `WindowOptions` bounds as logical pixels. Convert monitor work-area coordinates to logical units exactly once; never multiply the window size by DPI before passing it to GPUI.
2. Make the settings root fill the actual window (`w_full`, `h_full`) instead of using a fixed `900×760` surface.
3. Center one responsive content column with 32 logical pixel outer padding and a maximum 1000 logical pixel width. Small windows reduce padding to 16 through geometry-derived layout tokens rather than clipping the column.
4. Use a Windows 11 light/dark/high-contrast token set: Settings background, subtle card background, one-pixel card borders, eight-pixel radii, 64-pixel section headers, 56-pixel rows, 44×24 switches, and visible focus rings.
5. Preserve one vertical scroll owner for the whole page. Every expanded section and the Automatically hide the taskbar control must be reachable at 100–225% DPI and at the minimum supported window size.
6. Preserve pointer, keyboard, UIA, localized labels, atomic saves, errors, and owned navigation. No row invokes `explorer.exe`, `Shell_TrayWnd`, `ms-settings`, or system Settings.

## Architecture

- `taskbar-ui::taskbar_settings` owns pure responsive layout metrics and visual tokens plus the GPUI render tree.
- `superdesktop-app::surface_runtime` owns monitor-relative logical window bounds and centers the popup within the current work area.
- Existing `TaskbarSettingsModel` remains the sole behavioral model; layout changes cannot mutate settings or add authority.

## Error and lifecycle behavior

Invalid monitor DPI falls back to 96. Window dimensions clamp to the available logical work area and never become negative. Save errors remain visible in the scroll flow and do not resize or dismiss the window. Escape still dismisses the owned surface and no fallback opens system Settings.

## Verification

- Pure tests cover 100/125/150/175/200/225% DPI, small work areas, negative monitor origins, logical sizing, content width, padding, card and switch metrics.
- Source and UIA tests prove full-window responsive layout, scroll ownership, localized controls, focus semantics, and absence of delegated Settings routes.
- Headful captures at 175% verify the top, middle, and bottom of the page, including the auto-hide row, with no right-side dead canvas or clipped cards.
- Workspace fmt, locked/offline tests, clippy warnings-as-errors, release build, both NSIS packages, unique evidence indexing, and strict OpenSpec validation remain blocking gates.

## Non-goals

This change does not add new taskbar settings, implement unavailable Widgets/pen/touch-keyboard ownership, or recreate the complete Windows Settings application shell. Those are separate capability changes.
