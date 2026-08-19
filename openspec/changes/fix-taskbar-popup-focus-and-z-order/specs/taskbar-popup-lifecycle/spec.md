## ADDED Requirements

### Requirement: Context menu dismisses on window deactivation
The owned taskbar context menu SHALL dismiss when its popup window loses activation and SHALL remain open while focus moves between descendants inside the active menu.

#### Scenario: User activates another window
- **WHEN** the owned taskbar context menu is open and another SuperDesktop or external application window becomes active
- **THEN** the context menu window is removed and its shared window slot is cleared

#### Scenario: User navigates within the menu
- **WHEN** keyboard focus moves between command rows while the context-menu window remains active
- **THEN** the menu remains visible and the selected command continues to update

#### Scenario: Deactivation races with another dismissal route
- **WHEN** deactivation occurs while Escape, command activation, replacement, or teardown also requests dismissal
- **THEN** the popup is removed at most once without a panic or stale shared handle

### Requirement: Task preview occupies the topmost band without hover activation
Every owned task hover or click preview SHALL be placed in the Windows topmost band before it is exposed; hover-triggered previews MUST NOT change the foreground window or take keyboard focus.

#### Scenario: Hover preview overlaps an ordinary application window
- **WHEN** the hover delay expires for a task and an ordinary non-topmost window overlaps the preview bounds
- **THEN** the preview is visible above that window while the previously active foreground window remains active

#### Scenario: Click preview opens with keyboard interaction
- **WHEN** the user explicitly clicks a grouped task to open its preview
- **THEN** the preview is topmost and retains the existing activated keyboard-focus behavior

#### Scenario: Pointer leaves taskbar and preview
- **WHEN** the pointer leaves both the taskbar and topmost preview for the configured grace period
- **THEN** the preview is removed and no topmost popup remains

### Requirement: Popup promotion validates HWND ownership and fails closed
The Windows popup adapter SHALL accept only a nonzero live HWND owned by the current SuperDesktop process and SHALL use non-moving, non-sizing, non-activating topmost flags. Preview composition SHALL remove the popup if this contract cannot be established.

#### Scenario: Current-process preview HWND is valid
- **WHEN** preview composition supplies a live HWND owned by the current process
- **THEN** the adapter applies `HWND_TOPMOST` with `SWP_NOMOVE`, `SWP_NOSIZE`, and `SWP_NOACTIVATE` and reports success

#### Scenario: Preview HWND is invalid or foreign
- **WHEN** preview composition supplies zero, a retired HWND, or a window owned by another process
- **THEN** the adapter rejects the operation without changing that window and the preview is removed with rejection trace evidence

#### Scenario: Explorer-free boundary remains intact
- **WHEN** either popup lifecycle is exercised in Preview or Shell mode
- **THEN** no Explorer window, `Shell_TrayWnd`, foreground-window mutation, or global input hook is used to dismiss or promote the popup
