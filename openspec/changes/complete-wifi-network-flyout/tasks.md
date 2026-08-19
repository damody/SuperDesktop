## 1. Protocol foundation

### 1.1 Additive Wi-Fi snapshot and command contracts

**目的：** Define the bounded public data and command semantics consumed by every later layer.
**輸入：** Approved design, existing system-status protocol, collection/text limits.
**產出：** Wi-Fi protocol types, Accepted terminal, validation and round-trip tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** G-WIFI-PROTOCOL; `evidence/protocol/evidence-index.json`.
**完成門檻：** Additive old/new JSON, every bound, invalid identity, command, and terminal test passes with no credential/profile XML field.

- [x] 1.1.1 Add bounded `WifiNetwork` and `WifiStatus` types plus additive Wi-Fi availability on `NetworkStatus`.
- [x] 1.1.2 Add Refresh, saved-profile Connect, and Disconnect commands with exact identity validation.
- [x] 1.1.3 Add `Accepted` terminal validation without an observed snapshot generation.
- [x] 1.1.4 Add protocol round-trip, additive-default, maximum-bound, duplicate, and invalid-command tests.
- [x] 1.1.5 Run focused protocol tests and index the hashed log in `evidence/protocol/evidence-index.json`.

## 2. Native WLAN provider

### 2.1 Safe read-only WLAN enumeration

**目的：** Produce authoritative bounded Wi-Fi snapshots from Windows WLAN APIs.
**輸入：** Work package 1.1, Windows WLAN/NDIS generated bindings, existing platform status adapter.
**產出：** Feature flags, RAII WLAN client/list wrappers, decoder/reducer, snapshot integration.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** G-WLAN-MEMORY and G-WLAN-SNAPSHOT; `evidence/platform/enumeration.json`.
**完成門檻：** Malformed fixtures fail safely, no-adapter/service states are truthful, and live read-only enumeration completes with redacted evidence.

- [x] 2.1.1 Enable the exact Windows WiFi and Ndis feature modules without adding new crates.
- [x] 2.1.2 Implement scoped WLAN handle and returned-list memory ownership.
- [x] 2.1.3 Decode bounded interfaces, SSIDs, profiles, flags, signal, security, and connectability.
- [x] 2.1.4 Deduplicate and sort connected/saved/strongest networks deterministically at the 64-item cap.
- [x] 2.1.5 Integrate Wi-Fi availability into `network_status` without losing Network List Manager state.
- [x] 2.1.6 Add malformed length/count, duplicate, ordering, no-adapter, and service-failure tests.
- [x] 2.1.7 Run live read-only enumeration and save only redacted counts/state/hashes in `evidence/platform/enumeration.json`.

### 2.2 Identity-bound WLAN command adapter

**目的：** Admit only refresh, exact saved-profile connect, and exact interface disconnect operations.
**輸入：** Work package 2.1 current enumeration and protocol commands.
**產出：** Platform command functions and fail-closed unit/live-fixture evidence.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** G-WLAN-COMMAND-SAFETY; `evidence/platform/commands.json`.
**完成門檻：** Stale/mismatched/unsaved identities never invoke mutation; accepted operations call the exact WLAN API; conditional live mutation has an auditable passed or not-applicable disposition.

- [x] 2.2.1 Implement multi-interface Refresh admission and bounded result reporting.
- [x] 2.2.2 Implement exact current saved-profile Connect admission without credentials or profile XML.
- [x] 2.2.3 Implement exact current interface Disconnect admission.
- [x] 2.2.4 Add stale, mismatched, unsaved, empty, oversized, and provider-failure command tests.
- [x] 2.2.5 Run controlled saved-profile connect/disconnect validation when explicitly configured, otherwise record evidence-backed not-applicable without skipping command safety tests.

## 3. Host and application integration

### 3.1 System-status host command and reconciliation

**目的：** Route WLAN operations out of process and preserve generation/event authority.
**輸入：** Protocol 1.1 and platform packages 2.1–2.2.
**產出：** Host snapshot/command routing, Accepted terminals, Network event scheduling, tests.
**依賴：** 1.1, 2.1, 2.2.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** G-WIFI-HOST; `evidence/host/evidence-index.json`.
**完成門檻：** Refresh/connect/disconnect route exactly once, return Accepted, schedule reconciliation, and stale/restart/deadline paths fail safely.

- [x] 3.1.1 Route Wi-Fi commands to the platform adapter and enqueue Network reconciliation after acceptance.
- [x] 3.1.2 Advance protocol minor compatibility and retain major-version admission.
- [x] 3.1.3 Add accepted, rejected, stale-generation, timeout, restart, and snapshot-refresh host tests.
- [x] 3.1.4 Run focused host tests and index the hashed log.

### 3.2 App action mapping and open-flyout refresh

**目的：** Map UI actions to typed commands and update live flyouts only from reconciled snapshots.
**輸入：** Work package 3.1 and existing status client/reconciler/flyout lifecycle.
**產出：** UI action variants, app command mapping, Accepted handling, immediate/periodic refresh tests.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** G-WIFI-COMPOSITION; `evidence/app/evidence-index.json`.
**完成門檻：** Exact actions serialize correctly, Accepted never claims connection, refreshed snapshots update the open flyout, and failures preserve provider truth.

- [x] 3.2.1 Add Refresh/Connect/Disconnect UI action variants with exact identities.
- [x] 3.2.2 Map actions to protocol commands and handle Accepted terminals within bounded client timeouts.
- [x] 3.2.3 Refresh the open NetworkPower view from immediate and periodic authoritative snapshots.
- [x] 3.2.4 Add app source/model tests for mapping, no fake success, failure, and lifecycle exclusivity.

## 4. Complete owned Wi-Fi UI

### 4.1 Network panel model, geometry, and interactions

**目的：** Deliver the complete truthful Wi-Fi workflow in the owned flyout.
**輸入：** Reconciled Wi-Fi snapshots/actions and existing flyout theme/accessibility tokens.
**產出：** 640-DIP bounded geometry, connected card, scroll list, action gating, refresh/unsupported tiles, tests.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator / wave 6.
**Gate／Evidence：** G-WIFI-UI and G-WIFI-A11Y; `evidence/ui/evidence-index.json`.
**完成門檻：** 0/1/64-network, saved/unsaved/connected/unavailable, theme, locale, keyboard/UIA, scrolling, and geometry tests pass with no fake unsupported mutation.

- [x] 4.1.1 Increase and clamp NetworkPower preferred geometry for the bounded scroll panel.
- [x] 4.1.2 Render connected summary, security/Internet detail, information state, and Disconnect.
- [x] 4.1.3 Render sorted scrollable network rows with signal, lock, connected, saved, and connectability states.
- [x] 4.1.4 Gate Connect to exact saved profiles and show password-required/unavailable status for unsaved secured networks.
- [x] 4.1.5 Add Refresh plus truthful disabled Wi-Fi-radio/Airplane/Mobile-hotspot capability tiles and retain power summary.
- [x] 4.1.6 Add stable IDs, localization, roles, focus, pointer/Enter/Space parity, and error/unavailable states.
- [x] 4.1.7 Add 0/1/64 list, long SSID, theme, DPI, taskbar-row, scrolling, action-gating, and accessibility tests.

## 5. Integrated validation and privacy evidence

### 5.1 Quality and headful matrix

**目的：** Prove the cross-layer feature on the real Windows host without leaking machine network identity.
**輸入：** All implementation packages, release/headful fixtures, redaction rules.
**產出：** Quality logs, redacted live report, themed screenshots, final evidence indexes.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 7.
**Gate／Evidence：** G-WIFI-QUALITY, G-WIFI-HEADFUL, G-WIFI-PRIVACY; `evidence/quality/evidence-index.json`, `evidence/headful/evidence-index.json`.
**完成門檻：** Format/check/tests/clippy and strict OpenSpec pass; real enumeration/Refresh and themed UI pass; identifiers are hashed/redacted; controlled mutation is passed or evidence-backed not-applicable; no unresolved P0/P1 remains.

- [x] 5.1.1 Run format, locked all-target compilation, affected/full tests, and warnings-as-errors clippy with hashed logs.
- [x] 5.1.2 Run the real Wi-Fi flyout read-only/Refresh scenario and save redacted count/state evidence.
- [x] 5.1.3 Capture light, dark, and high-contrast connected/list/unavailable fixtures with keyboard/UIA checks.
- [x] 5.1.4 Verify committed reports/screenshots contain no raw SSID, profile, interface identity, credential, or profile XML.
- [x] 5.1.5 Hash/index quality, platform, host, app, UI, and headful evidence with unique task links.
- [x] 5.1.6 Run strict OpenSpec and detailed-task validation; map every scenario to current evidence.
- [x] 5.1.7 Confirm every task is passed or valid conditional not-applicable with no failed, blocked, stale, P0, or P1 item.
