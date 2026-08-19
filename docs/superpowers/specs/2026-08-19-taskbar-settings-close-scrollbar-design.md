# Taskbar settings close and scrollbar design

## Problem

The owned taskbar settings window is intentionally borderless and therefore has no native title-bar close button. Its root uses GPUI `overflow_y_scroll`, which accepts wheel input but does not paint a visible scrollbar thumb in this GPUI build. Users cannot discover how to close the surface or their position in the long settings page.

## Design

Keep the borderless Windows 11 settings presentation. Convert the settings root into fixed window chrome containing a separately scrollable content viewport. Add a fixed 36 DIP close button at the top-right, above the viewport, with a localized accessible name, hover/pressed/focus styling, the multiplication-sign glyph, and the existing dismiss callback. Escape continues to use the same callback.

Track the content viewport with a GPUI `ScrollHandle`. Render a fixed 12 DIP vertical scrollbar track at the right edge below the close button. Derive thumb height from viewport height divided by total content height, enforce a 48 DIP minimum, and derive thumb position from the current scroll offset. Wheel and keyboard scrolling update the tracked handle automatically. Pointer drag records the pointer's position within the thumb, maps movement across the track to the handle's bounded negative offset, and clears drag state on mouse-up. The scrollbar exposes `Role::ScrollBar`, a localized accessible name, and a 0–100 numeric value.

When content fits, the scrollbar is omitted. The content reserves right-side space so neither cards nor the close button overlap the thumb. Light, dark, and high-contrast colors come from the existing settings tokens.

## Alternatives

Restoring a native title bar was rejected because it changes the established owned Windows 11 settings geometry and duplicates custom chrome. Setting only GPUI `scrollbar_width` was rejected because it reserves layout space but does not paint or operate a thumb. A page-specific fake position indicator was rejected because it would not support dragging or remain synchronized with wheel scrolling.

## Verification

Pure/source-contract tests cover fixed close chrome, shared dismissal, tracked scrolling, thumb geometry bounds, drag-to-offset mapping, accessibility metadata, and the absence of a scrollbar when content fits. Focused crate tests, locked compilation, formatting, and warnings-as-errors clippy must pass. Headful validation captures the top of the page with the X and scrollbar, invokes the X through UI Automation, reopens the page, drags the thumb downward, and proves both the UIA percentage and visible content changed.

## Scope

This change affects only the owned taskbar settings surface and its headful validation. It does not alter settings fields, persistence, window size, section expansion, taskbar context commands, or other popup chrome.
