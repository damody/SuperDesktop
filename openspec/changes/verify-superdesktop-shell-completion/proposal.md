## Why

The completed shell feature set requires one release decision that proves functional parity targets without regressing M0 safety, accessibility, DPI, performance, or recovery guarantees.

## What Changes

- Add end-to-end verification for all completion-program capabilities.
- Add automated evidence capture, schema validation, coverage roll-up, and performance budgets.
- Preserve fail-closed external gates for Windows 10, physical mixed-DPI, and independent review.

## Capabilities

### New Capabilities

- `shell-completion-verification`: Release scenarios, evidence requirements, gate aggregation, and final disposition.

### Modified Capabilities

None.

## Impact

- Affects integration tests, headful fixtures, evidence scripts/schemas, CI/release checks, and program roll-up state.
