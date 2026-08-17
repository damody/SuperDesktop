# Desktop Marquee and Start Menu Parity Design

## Outcome

SuperDesktop supports Windows-style pointer marquee selection on empty desktop space and renders an owned Start menu that follows the active Windows 11 information architecture: Search, Pinned, Recommended, All apps, Account, Settings, and Power.

## Desktop marquee architecture

`DesktopView` owns a transient marquee gesture with anchor/current window points, the modifier mode, and the selection snapshot captured at pointer-down. Empty-space left down begins the gesture; item left down stops propagation so item drag, double-click, and selection remain authoritative. Mouse move normalizes both drag directions, intersects the rectangle with the actual 104×112 logical item hitboxes, and updates accessible selected/focused state. Ctrl unions hits with the baseline; an ordinary gesture replaces selection. Mouse up clears transient capture while retaining selection.

The marquee is painted above the wallpaper and below desktop items as a normalized absolute rectangle with a translucent Windows blue fill and one-pixel outline. A movement threshold suppresses a visible rectangle for an empty-space click. Losing the pressed-button state cancels the transient gesture to prevent a stuck selection surface.

## Start menu architecture

The existing `StartModel` remains the source of search generation, stale-result rejection, pinned/recent persistence, keyboard focus, and activation. It gains an explicit home/all-apps view mode and power-flyout state. It exposes bounded, deduplicated slices for up to 12 home pins, six recommendations, an alphabetical all-apps view, and ranked search results.

`StartView` renders a light Windows 11 surface with these modes:

- Home: search, Pinned heading plus All apps action, six-column pin grid, Recommended two-column rows.
- All apps: back action, alphabetical installed-app list, and persistent search.
- Search: icon-bearing ranked rows with title and subtitle.
- Footer: account identity, Settings, and one Power button whose menu exposes admitted power actions.

Application results resolve their existing `open:<path>` activation path through the shared Shell icon and BC7 render cache. Non-path settings use a stable semantic tile instead of a fabricated application icon. Start opens centered above the bottom work-area edge with a small gap, clamps to smaller monitors, and keeps Escape, arrows, Enter, IME, UI Automation, and first-terminal behavior.

## Alternatives

Restyling the existing flat list would leave the structure unlike Windows 11. Delegating to the host Start menu would fail when SuperDesktop owns the shell or the host is absent. The chosen owned surface provides deterministic behavior and can be verified without depending on Explorer.

## Failure handling

Missing icons retain readable labels and semantic fallback tiles. Empty catalogs render an explicit message. Failed searches retain the current deterministic provider state. Power actions stay behind the flyout and existing confirmation boundary. Closing Start clears its window slot exactly once. Marquee selection never mutates files or persisted item positions.

## Verification

Blocking gate `G-DESKTOP-START-PARITY` requires geometry/unit tests for normal, reverse, Ctrl-additive, empty-click, and item-propagation selection; model/view tests for home, all-apps, search, power, keyboard, and icon fallback; 175% DPI headful screenshots of an active marquee and Start home/all-apps; UI Automation names and hit targets; complete locked/offline workspace tests; clippy with warnings denied; strict OpenSpec validation; and rebuilt standalone/combined installers.

## Rollback

The feature changes transient view/model state and presentation only. Reverting the desktop gesture fields/listeners and Start view-mode/rendering changes restores prior behavior without settings migration or registry cleanup.
