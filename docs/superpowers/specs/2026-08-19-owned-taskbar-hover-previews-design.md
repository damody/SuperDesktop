# Owned Taskbar Hover Previews Design

## Intent

Complete the Windows taskbar preview interaction by opening SuperDesktop-owned DWM live-thumbnail cards after a bounded task-button hover delay, keeping them open while the pointer moves into the popup, and dismissing them after a short leave grace. The behavior must support single and grouped windows without calling Explorer or system taskbar UI.

## Chosen design

`taskbar-ui` owns a pure `HoverPreviewController` with the currently hovered task identity, popup-hover state, and monotonically increasing generation. Task enter creates an open token; task leave, task switch, popup enter, and popup leave invalidate stale timers. A 400 ms open timer succeeds only when its exact task/token remains current. A 250 ms close timer succeeds only when neither the task nor popup is hovered and its token remains current.

Every available task button emits a typed `task_hover(stable_id, hovered)` callback through GPUI `on_hover`. The application resolves both single-window and grouped stable IDs into fresh authoritative window snapshots, admits existing DWM live thumbnails, and opens the existing owned `TaskFlyoutView`. Click behavior remains unchanged: single tasks activate/minimize and groups open the preview selector. Hover never activates, minimizes, closes, or changes foreground windows.

The popup root emits typed hover transitions back to the same controller. This permits the pointer to cross the taskbar-to-popup gap without dismissal. Leaving both surfaces schedules the grace close. Entering another task replaces the popup after its own 400 ms delay. Destroyed/reused windows are removed through fresh identity resolution; an empty card set closes or does not open.

## Alternatives considered

- Delegating to Explorer or `Shell_TrayWnd` would reproduce system behavior but violates the replacement-shell boundary.
- Opening immediately on hover is simpler but feels unlike Windows and produces noisy popups during pointer travel.
- Keeping the existing click-only group selector avoids timing work but leaves single-window previews and standard Windows hover behavior missing.

## Presentation

The existing DWM thumbnail surface remains owned by SuperDesktop. This change aligns the popup to Windows proportions: bounded card width, real title, close button, light/dark/high-contrast tokens, distinguishable focus/hover borders, and no fixed 480-pixel height when fewer cards fit. Cards remain keyboard/UIA actionable after opening; Escape closes and returns taskbar ownership.

## Safety and failure handling

Generation tokens make timer callbacks idempotent and stale-safe. All source HWNDs are freshly snapshotted and passed through existing DWM admission. Failed registration renders truthful `Preview unavailable`; it never delegates. Popup close unregisters every DWM thumbnail through RAII. Timer work carries no native handles and only updates the exact owned popup slot.

## Verification

- Pure tests cover enter/leave delay, rapid task switching, stale timers, popup crossing, close grace, disabled previews, empty groups, and repeated cycles.
- Source/UI tests cover typed hover callbacks, single/group resolution, unchanged click behavior, theme tokens, UIA roles, keyboard actions, and no Explorer/system UI path.
- A controlled UTIT case moves a real pointer over single and grouped fixture tasks, proves no popup before 400 ms, popup after delay, persistence during popup entry, close after grace, fresh switch content, Explorer absence, and recovery.
- Full workspace, Clippy, release, strict/detailed OpenSpec, UTIT smoke/shell-parity, and both NSIS package gates run before completion. The change remains unarchived.
