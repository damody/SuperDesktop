# SuperDesktop Taskbar Readable Labels Design

## Problem

The production taskbar defaults `show_labels` to `false`, but `AccessibleTask` currently has no renderable task icon. `TaskbarView` responds by rendering only the first Unicode scalar of every window title. On a two-row taskbar this produces isolated characters such as `d`, `r`, `C`, and `好`, which are neither readable labels nor truthful icons.

Long labels are also inserted directly into the task button container. They rely on parent overflow clipping instead of owning a shrinkable, ellipsis-aware text region, so badges and narrow layouts can reduce the readable area unpredictably.

## Goals

- Every task without a real renderable icon displays its complete window/application label, even when the stored preference says labels are hidden.
- New and partially specified settings default to readable labels.
- Long English, Traditional Chinese, and grouped labels remain one line and truncate with an ellipsis inside the available task-button width.
- Accessibility names and task activation behavior remain unchanged.
- Existing explicit settings remain parseable and serializable.

## Non-Goals

- This change does not implement Win32 icon extraction, caching, or rendering.
- It does not change task grouping, ordering, AppBar geometry, or Shell takeover.
- It does not remove the `show_labels` setting; future real-icon rendering can honor `false` without inventing character placeholders.

## Design

Introduce a small pure display-label policy in `taskbar-ui`:

- grouped tasks append `(<count>)`;
- when labels are enabled, the full label is returned;
- when labels are disabled but no real icon exists, the full label is still returned as a truthful fallback;
- only a future path that supplies a real icon may omit the text label.

`TaskbarView` will render the chosen label in a child element with `flex_1`, `min_w_0`, single-line whitespace, hidden overflow, and GPUI ellipsis behavior. Badges remain flex-none siblings, so they cannot cause the label to collapse to a single character.

`TaskbarSettings::default()` and the decoder fallback for a missing `taskbar.show_labels` field become `true`. An explicitly stored `false` remains `false`; the render policy still guarantees readable fallback until a real icon is present.

## Alternatives

1. **Only change the default to `true`:** rejected because existing settings with `false` would keep reproducing the screenshot.
2. **Add real task icons now:** deferred because it requires HWND/application identity icon extraction, image ownership, caching, DPI invalidation, and fallback contracts beyond this bug.
3. **Keep first-character placeholders and add tooltips:** rejected because the visible taskbar remains unreadable and the characters falsely imply icon identity.

## Verification

- Unit-test the display-label policy for label-enabled, legacy label-disabled, grouped, empty, English, and Traditional Chinese titles.
- Unit-test settings defaults and missing-field decode behavior.
- Add a render/source contract ensuring the label child owns shrink/ellipsis styling and the first-character fallback is absent.
- Run `cargo fmt`, targeted `settings-store` and `taskbar-ui` tests, workspace check, and a headful taskbar capture.
- Inspect the resulting screenshot at the active 175% DPI profile and confirm full labels or ellipses appear instead of isolated first characters.

## Rollback

Reverting the display-label helper, label container, and two settings defaults restores the prior behavior. No persisted schema migration or external system mutation is required.
