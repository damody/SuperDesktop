## 1. Additive notification contracts

### 1.1 Windows event provider state

**目的：** Define compatible bounded access, synchronization, and change state consumed across host and UI.
**輸入：** Approved design, existing notification snapshot/validation limits.
**產出：** Additive protocol enums/status, defaults, validation and tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** G-WNE-PROTOCOL, G-WNE-ACCESS; `evidence/protocol/evidence-index.json`.
**完成門檻：** Old/new JSON, every state combination, bounds and round trips pass without changing existing notification semantics.

- [ ] 1.1.1 Add `WindowsNotificationAccess` and `WindowsNotificationChange` enums with safe defaults.
- [ ] 1.1.2 Add defaulted bounded `WindowsNotificationEventStatus` to `NotificationSnapshot`.
- [ ] 1.1.3 Validate synchronized/access/reason invariants and preserve old JSON compatibility.
- [ ] 1.1.4 Update all snapshot fixtures and add state/bound/round-trip tests.
- [ ] 1.1.5 Run focused protocol tests and hash/index the log.

## 2. Windows Runtime event source

### 2.1 Scoped listener access and callback lifecycle

**目的：** Own documented UserNotificationListener access, event subscription, dirty signaling and teardown safely.
**輸入：** Work package 1.1, Windows Runtime feature bindings, official listener contract.
**產出：** New platform adapter, access request policy, callback/rundown tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** G-WNE-RUNDOWN, G-WNE-ACCESS; `evidence/platform/access.json` and `evidence/platform/evidence-index.json`.
**完成門檻：** Allowed subscribes, unspecified requests once, denied/unavailable stay truthful, storms coalesce, and token/apartment teardown order passes.

- [ ] 2.1.1 Enable exact WinRT Foundation/ApplicationModel/Notifications/Management and Win32 WinRT features without a new crate.
- [ ] 2.1.2 Implement scoped MTA initialization and `UserNotificationListener::Current` activation.
- [ ] 2.1.3 Implement Allowed/Denied/Unavailable access mapping and one-shot Unspecified request.
- [ ] 2.1.4 Subscribe to `NotificationChanged` with no-unwind atomics-only callback state.
- [ ] 2.1.5 Coalesce Added/Removed event storms into one dirty reconciliation signal.
- [ ] 2.1.6 Revoke the event token exactly once before WinRT apartment teardown.
- [ ] 2.1.7 Add callback panic/rundown, repeated teardown, access, prompt-once and source-contract tests.
- [ ] 2.1.8 Run live non-mutating access preflight and record only redacted state/count capability.

### 2.2 Bounded authoritative Toast conversion

**目的：** Convert current Windows Toast snapshots into valid owned notifications without cross-item failure.
**輸入：** Work package 2.1 listener, protocol bounds, official ToastGeneric extraction contract.
**產出：** Snapshot query, conversion/reduction functions and tests.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** G-WNE-SNAPSHOT, G-WNE-PRIVACY; `evidence/platform/snapshot.json`.
**完成門檻：** App label/time/Toast text convert correctly; malformed items isolate; identities are stable/redacted; output is deduped/sorted/capped.

- [ ] 2.2.1 Join bounded `GetNotificationsAsync(NotificationKinds::Toast)` results.
- [ ] 2.2.2 Convert Windows 1601-based creation time safely to Unix milliseconds.
- [ ] 2.2.3 Extract App display label and ToastGeneric text with first-title/rest-body reduction.
- [ ] 2.2.4 Derive fixed-domain native notification ID and hashed AUMID icon key without raw identity exposure.
- [ ] 2.2.5 Isolate per-item failures and count skipped items without failing valid siblings.
- [ ] 2.2.6 Deduplicate/sort newest-first and cap native input/output before protocol publication.
- [ ] 2.2.7 Add time, text, empty-binding, malformed, duplicate, order, cap and privacy tests.
- [ ] 2.2.8 Run live read-only enumeration and save only access/count/skipped totals.

## 3. Host reconciliation and Windows mutation

### 3.1 Event-driven host lifecycle and merge

**目的：** Merge Windows and NotifyIcon origins from startup/events/periodic authority while preserving last-good state.
**輸入：** Platform packages 2.1–2.2 and existing `NativeCompatibilityRegistry`.
**產出：** Host-owned source, origin-aware reconciliation, generation/state logic and tests.
**依賴：** 2.1, 2.2.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** G-WNE-SNAPSHOT, G-WNE-ACCESS; `evidence/host/reconciliation.json` and `evidence/host/evidence-index.json`.
**完成門檻：** Startup/dirty/5s sync, event coalescing, generation stability, coexistence, transient failure retention and recovery pass.

- [ ] 3.1.1 Construct and retain the Windows event source for notification-host lifetime.
- [ ] 3.1.2 Reconcile before Snapshot/Health on startup, dirty event, or five-second deadline.
- [ ] 3.1.3 Replace only `windows:` records while preserving NotifyIcon history and combined 100-item cap.
- [ ] 3.1.4 Advance generation only on content/provider-state changes, not unchanged polling.
- [ ] 3.1.5 Preserve last-good Windows cards on transient query failure and publish unavailable reason.
- [ ] 3.1.6 Recover allowed/synchronized state and authority after a later successful query.
- [ ] 3.1.7 Add startup/add/remove/storm/periodic/failure/recovery/coexistence/cap tests.
- [ ] 3.1.8 Run focused host reconciliation tests and hash/index the log.

### 3.2 Exact synchronized dismiss and clear

**目的：** Apply Windows-origin remove/clear only to fresh exact authority and publish confirmed outcomes.
**輸入：** Work package 3.1 registry/source, existing generation-bound mutations.
**產出：** Origin-aware mutation routing, confirmation/rejection tests and controlled evidence.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** G-WNE-MUTATION; `evidence/host/mutations.json`.
**完成門檻：** Stale/malformed/absent IDs never call Windows; exact remove/clear precede local mutation and require authoritative confirmation.

- [ ] 3.2.1 Parse exact bounded `windows:<u32>` identities and distinguish NotifyIcon origin.
- [ ] 3.2.2 Validate expected generation and fresh native presence before `RemoveNotification`.
- [ ] 3.2.3 Reconcile and confirm absence before publishing a successful Windows dismiss.
- [ ] 3.2.4 Call `ClearNotifications` once only when current Windows records exist, then confirm empty authority.
- [ ] 3.2.5 Preserve prior local state on remove/clear/reconcile failure while retaining NotifyIcon local semantics.
- [ ] 3.2.6 Add stale, malformed, absent, wrong-origin, call-order, post-confirmation and failure tests.
- [ ] 3.2.7 Run controlled test-Toast dismiss when safely available, otherwise record evidence-backed not-applicable.

## 4. Client and owned center integration

### 4.1 Provider state and accessible Windows cards

**目的：** Present Windows-origin records and access/sync truth without unsupported actions.
**輸入：** Additive snapshot/status, existing center card/dismiss/clear UI and app polling.
**產出：** Status banner/empty semantics, source/model/UIA tests and themed fixtures.
**依賴：** 3.1, 3.2.
**Owner／Wave：** Primary integrator / wave 6.
**Gate／Evidence：** G-WNE-UI, G-WNE-ACCESS; `evidence/ui/evidence-index.json`.
**完成門檻：** Allowed/denied/unavailable/sync states, mixed origins, empty state, scroll, dismiss/Delete/Clear parity, themes and no-fake-action tests pass.

- [ ] 4.1.1 Reconcile additive Windows provider status through the existing notification client snapshots.
- [ ] 4.1.2 Render localized denied/unavailable/unspecified/synchronizing provider banner.
- [ ] 4.1.3 Distinguish true empty current notifications from inaccessible Windows events.
- [ ] 4.1.4 Render mixed Windows/NotifyIcon cards with existing bounded text/time/accessibility behavior.
- [ ] 4.1.5 Preserve pointer/Delete dismiss and pointer/Enter/Space clear-all exactly-once routing.
- [ ] 4.1.6 Add source guards proving no Toast reply/open/custom action or Explorer/system center route.
- [ ] 4.1.7 Add provider-state, mixed-origin, 0/1/100, long-text, scroll, locale, theme and UIA tests.
- [ ] 4.1.8 Run focused client/UI tests and hash/index logs.

## 5. Real Windows event and headful validation

### 5.1 Permission-safe live matrix

**目的：** Prove real access/events/UI without leaking or deleting existing user notifications.
**輸入：** Release binaries, listener access, controlled Toast fixture if available, themes and privacy rules.
**產出：** Redacted live/headful reports and hashes.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 7.
**Gate／Evidence：** G-WNE-LIVE, G-WNE-PRIVACY, G-WNE-UI; `evidence/live/evidence-index.json`, `evidence/headful/evidence-index.json`.
**完成門檻：** Real access/read sync passes; controlled add/remove/dismiss passes or valid N/A; no existing notification is cleared; themes/UIA/provider states pass; evidence is redacted.

- [ ] 5.1.1 Build release app/host and record content hashes.
- [ ] 5.1.2 Record real listener access/current count/skipped totals without content or identities.
- [ ] 5.1.3 Generate one controlled Toast when safe and observe Added/card appearance, otherwise record valid N/A.
- [ ] 5.1.4 Dismiss only the controlled Toast through SuperDesktop and observe Removed/absence, never clear existing records.
- [ ] 5.1.5 Capture light/dark/high-contrast center fixtures with mixed/provider/unavailable states and keyboard/UIA checks.
- [ ] 5.1.6 Verify event latency, five-second periodic reconciliation, one-popup focus/dismissal, and Explorer-free ownership.
- [ ] 5.1.7 Scan staged evidence for raw AUMID/native ID/app label/title/body or identity-bearing screenshots.
- [ ] 5.1.8 Hash/index every live/headful procedure with unique task subchecks.

## 6. Final quality and completion

### 6.1 Blocking gates and evidence closure

**目的：** Close every requirement/gate/task with reproducible evidence and no regression.
**輸入：** Work packages 1.1–5.1 and current evidence indexes.
**產出：** Full quality reports, strict validation, completion rollup and commits.
**依賴：** 5.1.
**Owner／Wave：** Primary integrator / wave 8.
**Gate／Evidence：** All gates; `evidence/quality/evidence-index.json`, `evidence/completion.json`.
**完成門檻：** Format/check/test/Clippy, strict OpenSpec, detailed-task validation, JSON/hash/privacy and 100% task dispositions pass with no failed/blocked/stale/P0/P1.

- [ ] 6.1.1 Run format and locked workspace all-target compilation.
- [ ] 6.1.2 Run affected/focused and full locked workspace tests.
- [ ] 6.1.3 Run locked workspace all-target Clippy with warnings denied.
- [ ] 6.1.4 Validate every JSON, hash, task subcheck and gate mapping.
- [ ] 6.1.5 Run strict OpenSpec and detailed-task validation with no incomplete marker/contradiction.
- [ ] 6.1.6 Confirm every task passed or valid conditional N/A with no failed/blocked/stale/P0/P1.
- [ ] 6.1.7 Commit implementation/evidence without unrelated worktree files.
