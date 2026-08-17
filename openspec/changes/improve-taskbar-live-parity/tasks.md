## 1. Never-Combine Defaults

- [x] 1.1 Default new and missing `combine_groups` to false.
- [x] 1.2 Preserve explicit true and add round-trip/default tests.

## 2. Multi-Row Packing

- [x] 2.1 Change the task region to column-direction wrapping.
- [x] 2.2 Add one/two/three-row packing and source-contract tests.

## 3. Live Clock

- [x] 3.1 Add and test the Win32 owned local-date/time adapter.
- [x] 3.2 Replace the production fixed clock with the platform value.
- [x] 3.3 Refresh taskbar status on minute/date changes without redundant frames.
- [x] 3.4 Add composition/source tests rejecting fixture-clock regression.

## 4. Verification

- [x] 4.1 Run formatting, targeted tests, and locked offline workspace check.
- [x] 4.2 Capture and inspect two-row headful evidence at 175% DPI.
- [x] 4.3 Record `G-TASKBAR-LIVE-PARITY` evidence and pass strict validation.
