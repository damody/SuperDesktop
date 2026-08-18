# Explorer-free Windows 11 GUI convergence

## Outcome

SuperDesktop and SuperExplorer form the complete interactive shell. Normal product behavior must not launch, address, or delegate presentation to `explorer.exe`, `Shell_TrayWnd`, `StartMenuExperienceHost`, `ShellExperienceHost`, Windows Settings URI handlers, or equivalent system-owned shell surfaces. The only permitted Explorer launch is the explicit guardian rollback path after takeover failure or crash recovery.

The visual target is the host Windows 11 build 26200 desktop: taskbar, Start, desktop, notification area, IME/status controls, context menus, overflow surfaces, settings cards, hover/focus/pressed states, progress indicators, shadows, typography, spacing, DPI scaling, high contrast, and Traditional Chinese layout.

## Delivery order

1. Complete `make-owned-start-exclusive`. Prove every mode opens the owned Start renderer and package the verified binaries.
2. Complete `add-shell-notifyicon-compatibility`. Own ordinary application NotifyIcon ingestion, lifecycle, callbacks, recovery, overflow, and compact Windows 11 rendering without Explorer.
3. Run a visual convergence pass across taskbar, Start, notification area, system flyouts, desktop, and owned settings using shared Windows 11 tokens and host screenshots at 175% DPI.
4. Close the parent completion and release-verification gates only after Explorer-free process observations, mixed-DPI evidence, accessibility checks, installers, and rollback evidence pass.

## Architecture

Product UI uses GPUI-owned surfaces. Windows APIs may provide data, icons, locale, input state, window state, accessibility integration, documented appbar behavior, and application callbacks; they may not provide the visible shell surface. Providers publish bounded immutable snapshots to pure UI models. UI activation emits typed commands back to provider hosts. Each host uses generation, session, HWND/PID identity, deadline, queue capacity, and exactly-once terminal fences.

Visual values are centralized by surface family: dimensions, corner radii, margins, icon sizes, typography, colors, shadows, animation timing, focus geometry, indicator geometry, and DPI conversions. Logical GPUI dimensions and physical Win32 window bounds remain explicit and separately tested.

## Explorer independence

Source guards scan production composition and reject system Start hosts, `Shell_TrayWnd`, taskbar Settings delegation, and Explorer-owned tray identities. Explorer recovery modules are allowlisted by exact module and call path. Preview mode must coexist without registering compatibility identities. Committed Shell mode must not depend on an existing Explorer process. Provider loss renders truthful unavailable state and bounded restart behavior; it never silently falls back to Explorer.

## Visual acceptance

Reference captures use the real host at 175% DPI, light/dark/high-contrast themes, Traditional Chinese, and representative active/minimized/attention/progress/grouped/unavailable states. Geometry assertions cover taskbar height, row packing, labeled long indicators, icon-only short indicators, system-area spacing, popup anchoring, work-area clamping, Start placement, card sizes, and hit targets. UIA evidence must identify every actionable or unavailable control correctly.

## Failure handling and rollback

Malformed, stale, cross-session, reused-window, oversized, timed-out, or duplicate provider input fails closed. A host crash clears stale UI state, restarts within a bounded policy, reconciles from an authoritative snapshot, and ignores stale generations. Shell takeover failure invokes the guardian rollback contract; ordinary feature failure does not start Explorer.

## Verification

Every implementation batch runs formatting, locked/offline workspace tests, clippy with warnings denied, strict OpenSpec validation, detailed task validation, release builds, standalone and combined installer builds without launch, artifact hashing, UIA inspection, host screenshots, process observations, and source delegation guards. Changes remain unarchived until the user explicitly requests archival.

## Decisions

- Windows 11 build 26200 is the visual and physical validation baseline.
- Functional shell ownership and visual parity advance together; screenshot similarity alone is insufficient.
- Documented Windows APIs are allowed as data/protocol providers, but system-owned Explorer UI is not.
- Existing active OpenSpec changes remain the execution units; this design fixes their order and shared acceptance boundary instead of duplicating their scope.
