# Owned Show Desktop Corner Design

## Intent

Add the Windows 11 taskbar's far-right Show desktop corner to the SuperDesktop-owned taskbar. The feature must continue to work when Explorer and the Windows shell UI are absent, must preserve windows that were already minimized, and must provide a reversible second-click restore without invoking Win+D or shell automation.

## Chosen design

SuperDesktop owns a small reducer-backed session. On the first activation it snapshots eligible top-level task windows, records each exact `(HWND, process id, window identity)` that is not minimized, and minimizes only those records. On the second activation it takes a fresh authoritative snapshot and restores only records whose complete identity still matches. Destroyed, replaced, cloaked, transient, tool, or otherwise ineligible windows are ignored. The session then clears.

The taskbar renders an 8-DIP full-height button after the clock/status content. It is flush with the monitor's far-right client edge, uses a subtle Windows-style hover/pressed fill and inner edge accent, exposes a localized Button role/name, and supports pointer, Enter, and Space activation. Multi-row taskbars keep one uninterrupted corner strip; no horizontal row separators are introduced.

## Alternatives considered

- Sending Win+D or using shell COM would produce familiar behavior but delegates authority to Explorer/shell components and violates the Explorer-free requirement.
- Minimizing every visible window without retaining exact identities is simple but cannot safely implement the Windows second-click restore behavior.
- Retaining HWND alone is unsafe because Windows may reuse handles after a window closes; the complete process and stable window identity is required.

## Data flow

1. The GPUI corner control emits one typed callback.
2. The app asks `platform-win` for an authoritative top-level window snapshot.
3. A pure taskbar reducer selects either `Minimize` targets or exact `Restore` targets.
4. The app applies validated Win32 window actions directly and reports successful targets back to the reducer.
5. The taskbar refresh observes the resulting window states without optimistic task-model mutation.

## Eligibility and safety

Eligible targets are visible, non-tool, non-cloaked, non-owned-transient windows. The SuperDesktop taskbar and other GPUI utility surfaces are already excluded by those native styles. The minimize pass excludes windows already minimized. The restore pass requires an exact identity match in a new snapshot and never activates a restored window, avoiding focus theft and preserving z-order as far as documented Win32 behavior allows.

No undocumented API, Explorer HWND, shell URI, synthetic keyboard chord, registry mutation, or settings migration is introduced. A failed or stale target is skipped; it cannot widen the target set.

## Verification

- Reducer tests cover empty sets, pre-minimized preservation, deterministic ordering, partial failures, stale handles, PID/identity mismatch, repeated activation, and topology changes.
- Native tests cover the non-activating restore action and live-HWND rejection.
- GPUI source/UI tests cover exact far-edge geometry, multi-row height, light/dark/high-contrast states, Button semantics, localization, Enter, and Space.
- A controlled Explorer-absent headful fixture proves first-click minimize, second-click exact restore, pre-minimized preservation, UIA invocation, 175% DPI bounds, and absence of forbidden system UI processes.
- Full workspace, Clippy, release, traceability, and both NSIS package gates run before completion. The OpenSpec change remains unarchived.
