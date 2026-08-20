# Windows GUI Parity Automation Design

## Intent

Turn UTIT into the authoritative automated GUI parity gate for the final SuperDesktop + SuperExplorer shell. Every owned surface must be measurable without Explorer and must converge toward Windows 11 geometry, proportions, interaction, theme, locale, DPI, accessibility, and popup-lifecycle behavior.

## Delivery waves

1. **Parity foundation:** one typed manifest enumerates every owned surface, expected Windows geometry tokens, supported rows/DPI/theme/locale variants, required controls, interactions, and Explorer-free disposition. UTIT rejects missing, duplicate, or untested manifest entries.
2. **Shell chrome:** taskbar, Start, input/volume/network/calendar flyouts, notification overflow, context menus, Jump Lists, hover previews, task view, and Alt-Tab consume shared Windows geometry tokens and emit normalized measurement reports.
3. **Desktop and Explorer integration:** desktop icons, selection, context menus, SuperExplorer launch/focus, and file-operation UI use the same manifest and measurement contract.
4. **Release closure:** complete Explorer-free matrix, physical mixed-DPI when available, installer/reboot recovery, performance, accessibility, and independent visual review.

This change implements wave 1 and the first shell-chrome corrections. Later waves cannot redefine or weaken its gates.

## Architecture

`superdesktop-utit` gains a compiled `GuiParityManifest`. A surface entry declares stable identity, owner, reference family, canonical logical size or bounded formula, internal regions, taskbar gap, supported variants, required UIA controls, required pointer/keyboard actions, screenshot/report artifacts, Explorer-free policy, and conditional hardware requirements. Catalog validation proves every GUI case maps to manifest coverage and every manifest entry maps to an executable or evidence-backed conditional case.

Headful scripts emit one normalized `gui-parity-measurement/v1` JSON shape. All coordinates are physical screen bounds plus the window DPI; UTIT derives DIP exactly once. Measurements include outer bounds, content bounds, named regions, control hit targets, spacing, ratios, popup ownership, theme/locale, Explorer absence, and action terminals. Thresholds come from the manifest, never from script-local magic numbers.

Production UI consumes shared `WindowsGuiMetrics` constants and bounded formulas. Preview and committed-shell anchors are separate, but both are explicit and testable. Recovery code may launch Explorer only after a test/product failure or explicit user return-to-default action; normal composition, UI actions, Settings launches, and SuperExplorer activation must never invoke Explorer.

## First-wave reference contract

- Taskbar row height: 40 DIP; primary icon/button target: 44×40 DIP; status icon target: 36×36 DIP; taskbar edge and popup gap: 8 DIP nominal, 2–16 DIP accepted.
- Start: 640 DIP preferred width, bottom anchored above the owned taskbar, 720 DIP maximum height, contained in the monitor work area.
- System flyouts: input/volume/network 360 DIP preferred width; calendar/notification 380 DIP; 8 DIP taskbar gap; height content-bounded and monitor-clamped.
- Notification overflow: 344 DIP width, 48 DIP grid cells, six columns, 12 DIP panel padding.
- Context menus: 220–240 DIP width, 44 DIP command rows, 4 DIP inner padding, 8 DIP corner radius.
- Jump Lists and previews: source-button centered when possible, monitor-clamped, 8 DIP taskbar gap, content-sized within bounded minima/maxima.

These values are reference tokens, not screenshot-only assertions. Text scaling may increase required height but must not reduce hit targets, overlap regions, or move a popup outside its monitor.

## Testing and failure handling

UTIT adds manifest unit tests, a source gate for forbidden Explorer dependencies, normalized report validation, and a `gui-parity` tag. A failed measurement includes the exact token, expected range, actual physical/DIP values, scale, surface, variant, screenshot hash, and recovery state. Missing controls, unsupported local assumptions, stale UIA objects, duplicate callbacks, popup overlap, wrong owner, or Explorer presence fail the case. Real hardware absence is `blocked` or evidence-backed `not-applicable`, never passed.

## Safety and scope

No arbitrary executable/URI input is introduced. Explorer remains available only to the guardian, installer rollback, test watchdog, and explicit Return to default Explorer flow. Existing unrelated working-tree changes remain untouched. Wave 1 does not claim physical mixed-DPI, reboot, or independent visual review passed when unavailable.
