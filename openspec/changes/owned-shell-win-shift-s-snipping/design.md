## Context

SuperDesktop installs its low-level Windows-key hook only in owned-shell mode. The reducer currently maps `Win+S` to owned search but treats `Win+Shift+S` as unsupported, so the chord is passed onward even when Explorer is absent. Microsoft specifies that chord as the entry to the built-in Snipping Tool image overlay.

The approved source design is `docs/superpowers/specs/2026-08-21-owned-shell-win-shift-s-snipping-design.md`.

## Goals / Non-Goals

**Goals:**

- route `Win+Shift+S` once and only once in owned-shell mode;
- activate Windows' registered built-in screen-clipping overlay without Explorer or executable-path discovery;
- preserve `Win+S`, unsupported modifier pass-through, bounded hook lifetime, and console-first failure reporting;
- prove the physical shortcut from the release candidate and include it in the installer.

**Non-Goals:**

- custom screenshot UI or clipboard processing;
- `Win+Shift+R`, Print Screen, delayed capture, or Snipping Tool preference changes;
- user-configurable protocols, executable paths, or third-party fallbacks.

## Decisions

### 1. Add a dedicated queued action

`ShellHotkeyAction::OpenScreenSnip` uses the next request bit and is included in the exact `from_code` round trip. `action_for_key` maps `VK_S` with Shift to it before the existing no-Shift search branch. The shared active-key fence suppresses repeats and consumes the matching key-up.

Launching inside the hook was rejected because a global low-level callback must remain bounded and unwind-fenced.

### 2. Use a fixed Windows protocol

`platform-win::common::shell_hotkey` exposes a narrow `open_screen_snipping_overlay()` helper. A live observation of Explorer's real chord on the target Windows 11 build produced Snipping Tool AUMID `Microsoft.ScreenSketch_8wekyb3d8bbwe!App` with command argument `ms-screenclip:///?source=HotKey`. Explorer-free evidence showed ShellExecute accepted without presenting the overlay and `ActivateForProtocol` returned `0x80270254` because the package does not declare this as a Windows.Protocol extension. The helper therefore uses documented local-server `IApplicationActivationManager::ActivateApplication` with the fixed observed AUMID, fixed argument, and `AO_NONE`. No caller data crosses this boundary.

The newer `ms-screenclip://capture/...` integration API is deliberately excluded because Microsoft requires a packaged caller and registered redirect URI, while the owned shell needs native hotkey behavior and must not receive captured media.

ShellExecute, `ActivateForProtocol`, hard-coded Store-app paths, `SnippingTool.exe` discovery, Explorer mediation, and key re-injection were rejected as Explorer-dependent, contract-incompatible, version-sensitive, unavailable in the owned shell, or recursion-prone.

### 3. Dispatch on GPUI's existing foreground refresh

The refresh loop handles the action outside the hook and starts one named activation worker per fenced physical press. The worker owns COM initialization and can wait for the packaged app without blocking GPUI. Success writes requested/accepted trace events. Failure calls `report_error("shell-hotkey:screen-snip", error)` and leaves SuperDesktop alive. The action does not require a taskbar handle or foreground activation.

### 4. Preserve the mode boundary

No hook is installed when `run(shell)` receives false. Therefore preview mode with Explorer continues to use Windows' native shortcut path and cannot double-launch. Owned-shell UTIT explicitly suppresses Explorer and passes `--shell`.

### 5. Gate the real chord without retaining captured screen content

Unit tests cover reducer/queue/protocol source contracts. A bounded verification-only entry retains normal session/owner admission but runs the real owned surfaces and hook without mutating Winlogon registration, arming the recovery guardian, or invoking UAC; the harness itself removes and restores Explorer. A new headful UTIT sends physical Windows, Shift, and S key events, waits for requested/accepted traces plus a built-in screen-clipping process or surface, sends Escape, and verifies both the overlay is dismissed and SuperDesktop remains alive. Reports contain identities, trace/hash data, and booleans but no screenshot of the user's desktop.

The focused case must pass twice. Workspace tests, Clippy with denied warnings, release, installer, and embedded candidate hash equality remain blocking.

## Failure handling and observability

Hook registration, protocol admission, runtime update, and headful observation failures remain distinct console/test failures. Production code adds no unwrap or panic. A missing protocol is truthful failure; no fallback executable or Explorer process is started.

## Risks / Trade-offs

- [Protocol registration is damaged or Snipping Tool is removed] → emit the exact console error and keep the shell alive; do not masquerade as success.
- [Windows changes the hosting process name or hotkey URI] → headful evidence compares the current signed Snipping Tool process/command identity; an observed contract change triggers a B-level design correction rather than a silent fallback.
- [Synthetic keys are blocked by desktop authority] → the focused run fails rather than using a source-only conditional pass.
- [The overlay can expose desktop content] → store no overlay screenshot; retain only process/window metadata and traces.

## Migration Plan

Land reducer, platform activation, runtime dispatch, UTIT, and evidence together. Build the signed/packaged release candidate only after source gates pass. There is no data migration. Rollback is a code revert and reinstall of the prior package.

## Plan correction policy

- **A — task refinement:** command, split, order, or evidence-location changes that preserve behavior and every blocking gate.
- **B — design/spec correction:** a discovered Windows protocol or test-observation constraint inside scope requires updating design/spec/tasks and invalidating dependent evidence before continuing.
- **C — material change:** a different protocol, executable fallback, weakened physical gate, new shortcut scope, permission, external write, or destructive action requires user approval.

## Open Questions

None. Protocol admission and overlay observability are implementation gates, not deferred requirements.
