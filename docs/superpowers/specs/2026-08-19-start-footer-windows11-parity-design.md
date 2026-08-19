# Start Footer Windows 11 Parity Design

## Problem

The owned Start footer renders a Settings gear beside Power. Windows 11 places the account control on the left and only Power on the right. The production Start UTIT currently requires both Settings and Power, so it certifies the visible mismatch.

## Decision

Remove the footer Settings button and its render-only activation clones. Keep the 52 DIP footer, account control, one 40 by 40 DIP Power button, existing focus styling, and owned power popup. Settings remains discoverable through Start search/pins and the owned taskbar settings surface; no capability or command protocol is removed.

The Start UTIT home contract will require Pinned, Recommended, and Power, require the Settings footer control to be absent, count exactly one footer action, measure the Power hit target at 38-42 DIP, and prove its outer-window right inset is 20-36 DIP (24 DIP authored content inset plus DPI-rounded non-client frame). Power expansion, three owned actions, dismissal, Explorer absence, and recovery remain mandatory.

## Failure handling and verification

Removing a render-only control has no persistence or migration path. Unit/source-contract tests reject `start-settings` returning to the footer. Focused Start capture and the full 17-case shell-parity suite provide live evidence. Workspace, Clippy, release, architecture, source-boundary, and strict OpenSpec gates remain required.

## Non-goals

This change does not redesign pinned applications, search providers, account behavior, power actions, Start window dimensions, or taskbar settings.
