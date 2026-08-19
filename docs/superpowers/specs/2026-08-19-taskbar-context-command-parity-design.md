# Taskbar Context Command Parity Design

## Problem

The owned taskbar context menu exposes only Lock the taskbar, Task Manager, and Taskbar settings. Windows exposes common taskbar controls directly, including Search presentation, the Task View button, and Show desktop. The live resize UTIT case currently locates only the lock row and cannot detect missing commands, wrong ordering, or implausible menu geometry.

## Decision

Expand the owned context menu to six 32 DIP command rows in this order: Search presentation, Show Task View button, Show the desktop, Task Manager, Lock the taskbar, and Taskbar settings. Keep a 220 DIP Windows-style width and size the surface to its rows plus padding and gaps.

Search cycles deterministically through Hidden, Search icon only, and Search box. Task View and Lock are checked commands. Those three settings commands update a cloned settings document and use the existing atomic settings-store save path; failed saves leave the live settings unchanged. Show desktop invokes the existing owned `ShowDesktopSession` cycle. Task Manager continues to use its explicit System32 launch adapter, and Taskbar settings continues to open the owned settings window. No command launches or delegates UI to `explorer.exe`.

## Components and data flow

- `taskbar-ui::TaskbarContextModel` owns the stable six-command order and keyboard selection.
- `TaskbarContextView` receives current lock, search, and Task View state, renders localized labels and checked accessibility state, and emits typed commands.
- `superdesktop-app::surface_runtime` applies setting mutations through a pure helper, persists them atomically, routes Show desktop to the existing owned session, and keeps non-setting commands isolated.
- `capture-taskbar-resize-lock.ps1` records all UI Automation menu items in order, the popup bounds converted once from physical pixels to DIP, and rejects missing, duplicated, reordered, clipped, or incorrectly checked rows.

## Failure handling

Settings are saved with the existing revisioned store. A save failure records a rejected trace and does not update the live settings snapshot. Show desktop already fails closed on stale window identity. Menu geometry clamps through the existing popup placement function, and UTIT fails if any item has empty bounds or the popup leaves the monitor work area.

## Verification

1. Unit tests cover command order, keyboard wrap, Search cycling, Task View/Lock toggles, non-setting commands, and first-save behavior.
2. Focused Explorer-free resize UTIT verifies six rows, ordering, checked names, 200-240 DIP width, content-fit height, and successful lock action.
3. Full shell-parity reruns all 17 GUI/build/recovery cases.
4. Formatting, workspace tests, Clippy with warnings denied, release, architecture, source-boundary, and strict OpenSpec validation remain mandatory.

## Non-goals

This change does not add a nested Search submenu, window-arrangement commands, toolbar extension menus, or new taskbar settings fields.
