# Extend SuperDesktop Shell Contracts

## 1. Protocol foundation

### 1.1 Create the platform-neutral protocol crate

**目的：** Establish the single owned DTO and wire-contract boundary used by all completion features.
**輸入：** Approved design; `shell-core` identity conventions; workspace lint and Windows-only policy.
**產出：** `crates/shell-provider-protocol`; workspace dependency/member updates; public envelope and DTO modules.
**依賴：** Existing M0 shell-core contract is complete.
**Owner／Wave：** Primary integrator / Wave 1.
**Gate／Evidence：** `G-ARCH`, `G-SAFETY`; unit-test output and API surface review.
**完成門檻：** The crate builds offline, has no GPUI/Win32 dependency, and exports validated owned DTOs.

- [x] 1.1.1 Add the crate manifest, module structure, protocol constants, identifiers, and envelope types.
- [x] 1.1.2 Add shell item, command, search, notification, preview, and virtual-desktop DTOs.
- [x] 1.1.3 Add validation limits and typed validation failures.
- [x] 1.1.4 Add deterministic JSON round-trip and negative validation tests.

### 1.2 Define compatibility and lifecycle contracts

**目的：** Make protocol evolution, deadlines, cancellation, progress, and terminal outcomes mechanically testable.
**輸入：** 1.1 DTOs and validation rules.
**產出：** Compatibility helpers, lifecycle enums, handshake DTO, fixture manifest.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 1.
**Gate／Evidence：** `G-TRACE`, `G-SAFETY`; compatibility fixture tests.
**完成門檻：** Same-major additive frames are accepted, unsupported majors fail closed, and lifecycle invariants pass tests.

- [x] 1.2.1 Implement protocol-version negotiation and handshake limits.
- [x] 1.2.2 Implement request lifecycle and exactly-one-terminal validation.
- [x] 1.2.3 Add compatibility fixtures for supported, unsupported, malformed, and oversized input.

## 2. Isolated provider host

### 2.1 Add bounded host dispatch

**目的：** Keep provider failures and blocking work outside the GPUI shell process.
**輸入：** Protocol crate from 1.2; M0 process supervision conventions.
**產出：** `crates/shell-provider-host` library and executable with bounded stdin/stdout JSON framing.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / Wave 1.
**Gate／Evidence：** `G-ARCH`, `G-SAFETY`; host integration-test log.
**完成門檻：** The host handshakes, dispatches built-in capability requests, rejects invalid/capacity cases, and terminates cleanly on EOF.

- [x] 2.1.1 Add host manifest, library dispatcher, active-request registry, and capacity checks.
- [x] 2.1.2 Add newline-delimited JSON binary framing with maximum-frame enforcement.
- [x] 2.1.3 Add deadline, duplicate-ID, cancellation, and terminal-response behavior.
- [x] 2.1.4 Add host integration tests covering handshake, echo/health, invalid input, and EOF shutdown.

### 2.2 Integrate and verify the contract boundary

**目的：** Prove the new boundary is consumable and does not regress M0.
**輸入：** 2.1 crates; workspace CI and evidence conventions.
**產出：** Contract manifest, consumer smoke test, formatted and lint-clean workspace.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / Wave 1 exit.
**Gate／Evidence：** `G-ARCH`, `G-SAFETY`, `G-TRACE`; cargo fmt/check/clippy/test logs.
**完成門檻：** Strict OpenSpec validation and all relevant offline Rust checks pass with a clean contract manifest.

- [x] 2.2.1 Add a shell-core consumer smoke test and deterministic contract manifest generator.
- [x] 2.2.2 Run formatting, offline check, clippy, unit, and integration tests.
- [x] 2.2.3 Run strict OpenSpec and detailed-task validation and record terminal evidence.
