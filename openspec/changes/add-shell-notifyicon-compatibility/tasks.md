# Shell NotifyIcon Compatibility Implementation Tasks

## 1. Contracts and Ingress Guards

### 1.1 Define bounded compatibility DTOs

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Normalize supported native notification operations without pointers or handles.
**Inputs:** Approved design, existing notification DTOs and Windows structure versions.
**Outputs:** Versioned compatibility request, identity, operation and terminal DTOs.
**Dependencies:** None.
**Owner/Wave:** Primary agent / wave 1.
**Gate/Evidence:** `G-NOTIFY-ISOLATION`, `G-TRACE`; protocol test record.
**Completion threshold:** Every field is bounded and malformed/version/stale cases round-trip or reject deterministically.

- [x] 1.1.1 Add supported layout/version and operation DTOs.
- [x] 1.1.2 Add process/session/window/icon identity and callback-route DTOs.
- [x] 1.1.3 Add generation, deadline, overflow and exactly-once terminal DTOs.
- [x] 1.1.4 Add serialization bounds, duplicate, malformed and stale tests.

### 1.2 Guard preview non-interference

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Prevent compatibility identity creation while Explorer owns the shell.
**Inputs:** Lifecycle authority and composition source.
**Outputs:** Admission policy and product source guard.
**Dependencies:** 1.1.
**Owner/Wave:** Primary agent / wave 1.
**Gate/Evidence:** `G-SHELL-NONINTERFERENCE`; source/admission fixture record.
**Completion threshold:** Preview fixtures prove zero compatibility classes, broadcasts and registry mutations.

- [x] 1.2.1 Add explicit preview/shell compatibility admission states.
- [x] 1.2.2 Add negative fixtures for Explorer-present identity collision.
- [x] 1.2.3 Add product source guard forbidding preview compatibility creation.

## 2. Native Compatibility Adapter

### 2.1 Parse and copy native notification data

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Convert supported native inputs into owned validated values.
**Inputs:** 1.1 contracts and documented `NOTIFYICONDATAW` layouts.
**Outputs:** `platform-win` parser/copier and deterministic fixtures.
**Dependencies:** 1.1.
**Owner/Wave:** Primary agent / wave 2.
**Gate/Evidence:** `G-NOTIFY-ISOLATION`; native adapter test record.
**Completion threshold:** Supported versions copy exact identity/tooltip/state/icon and invalid inputs cause zero mutation.

- [x] 2.1.1 Define supported cbSize/version/flag matrix.
- [x] 2.1.2 Copy bounded tooltip, GUID/uID identity, callback message and state.
- [x] 2.1.3 Copy HICON pixels to owned RGBA with exact destruction ownership.
- [x] 2.1.4 Validate live same-session PID/HWND ownership and reuse resistance.
- [x] 2.1.5 Add null, truncated, oversize, wrong-session, dead-window and icon-failure tests.

### 2.2 Own the exclusive compatibility window

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Receive supported Shell notification traffic outside GPUI.
**Inputs:** 1.2 admission and 2.1 parser.
**Outputs:** Compatibility thread/window lease with bounded ingress queue.
**Dependencies:** 1.2, 2.1.
**Owner/Wave:** Primary agent / wave 2.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-NOTIFY-ISOLATION`; HWND/class trace.
**Completion threshold:** Exactly one admitted identity receives traffic; callback panic and teardown never unwind or double-release.

- [x] 2.2.1 Register the owned compatibility class/window only in committed Shell mode.
- [x] 2.2.2 Add no-unwind window procedure and copied-message handoff.
- [x] 2.2.3 Add bounded coalescing ingress with protected terminal capacity.
- [x] 2.2.4 Fence shutdown, unregister class and destroy HWND idempotently.
- [x] 2.2.5 Add collision, panic, storm and teardown-race tests.

## 3. Host Registry, Callbacks and Recovery

### 3.1 Map native operations into the registry

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Produce monotonic visible state for complete icon lifecycles.
**Inputs:** 2.2 normalized ingress and existing registry.
**Outputs:** Generation-bound add/modify/delete/focus/version mapping.
**Dependencies:** 2.2.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-TRACE`; lifecycle matrix.
**Completion threshold:** Full lifecycle is deterministic; stale/duplicate operations cannot resurrect icons.

- [x] 3.1.1 Add client lease and host/icon generation tracking.
- [x] 3.1.2 Map add and newer modify with stable ordering.
- [x] 3.1.3 Map delete, set-focus and version negotiation terminals.
- [ ] 3.1.4 Remove all icons for dead/disconnected client identity.
- [ ] 3.1.5 Add capacity, stale, duplicate, disconnect and restart tests.

### 3.2 Deliver validated callback messages

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Return SuperDesktop interactions only to the owning client.
**Inputs:** 3.1 leases and existing typed notification events.
**Outputs:** Revalidated callback delivery and terminal diagnostics.
**Dependencies:** 3.1.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-NOTIFY-ISOLATION`; callback trace.
**Completion threshold:** Activate/context/focus routes post once to the correct live HWND; stale owners receive nothing.

- [ ] 3.2.1 Translate negotiated pointer/context/focus events to native callback payloads.
- [ ] 3.2.2 Revalidate PID/session/HWND immediately before delivery.
- [ ] 3.2.3 Add exactly-once terminal, timeout and cancellation handling.
- [ ] 3.2.4 Add dead-client, HWND-reuse, wrong-session and callback-panic tests.

### 3.3 Recover registrations after takeover or restart

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Restore clients through documented taskbar recreation behavior.
**Inputs:** 3.1 generations and compatibility lease.
**Outputs:** `TaskbarCreated` recovery broadcast and authoritative cleanup.
**Dependencies:** 3.1.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-TRACE`; restart/re-registration record.
**Completion threshold:** Old icons clear, one recovery broadcast occurs and only new-generation re-registrations return.

- [ ] 3.3.1 Resolve/register the documented recovery message safely.
- [ ] 3.3.2 Emit recovery only after admitted ownership or successful restart.
- [ ] 3.3.3 Add dead-client timer and authoritative full reconciliation.
- [ ] 3.3.4 Add overflow/restart/stale re-registration tests.

## 4. Product Integration and Owned UI

### 4.1 Supervise compatibility health in composition

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Make notification compatibility lifecycle-aware without destabilizing other providers.
**Inputs:** Wave 3 host and lifecycle health contracts.
**Outputs:** Start/supervise/clear/restart integration and health gates.
**Dependencies:** 3.2, 3.3.
**Owner/Wave:** Primary agent / wave 4.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-SHELL-NONINTERFERENCE`; integration record.
**Completion threshold:** Shell mode requires healthy compatibility before Explorer exit; runtime failure clears only icons and restarts boundedly.

- [ ] 4.1.1 Start compatibility ownership only after committed Shell admission.
- [ ] 4.1.2 Gate Explorer-free transition on handshake and identity health.
- [ ] 4.1.3 Clear stale icons and bound restart after host loss.
- [ ] 4.1.4 Add preview/shell/crash/restart mode-independent integration tests.

### 4.2 Complete accessible notification rendering

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Present supported client icons like the Windows 11 notification area.
**Inputs:** 4.1 live snapshots and existing notification model.
**Outputs:** Compact tray grouping, owned overflow and typed actions.
**Dependencies:** 4.1.
**Owner/Wave:** Primary agent / wave 4.
**Gate/Evidence:** `G-NOTIFY-A11Y`; UIA and headful records.
**Completion threshold:** Real copied icons, tooltip, stable order, overflow, pointer/keyboard/UIA and unavailable behavior pass.

- [x] 4.2.1 Render copied icon pixels with truthful fallback and compact Windows 11 spacing.
- [x] 4.2.2 Route activate/context/focus through one typed action path.
- [x] 4.2.3 Add one owned overflow popup with Escape/outside dismissal and focus return.
- [x] 4.2.4 Add ordering, tooltip, pointer, keyboard, UIA, overflow and unavailable tests.

## 5. Verification and Packaging

### 5.1 Run compatibility gates

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Prove ordinary client behavior and process isolation on Windows 11.
**Inputs:** Integrated release binaries and controlled legacy client fixture.
**Outputs:** Automated logs, HWND/process traces, screenshots and evidence index.
**Dependencies:** 4.2.
**Owner/Wave:** Primary agent / wave 5.
**Gate/Evidence:** All change gates; `evidence/evidence-index.json`.
**Completion threshold:** Every leaf has unique evidence, controlled lifecycle passes without Explorer and strict validation succeeds.

- [ ] 5.1.1 Run fmt, locked/offline workspace check/tests and clippy warnings-as-errors.
- [ ] 5.1.2 Build release binaries and controlled ordinary NotifyIcon fixture with hashes.
- [ ] 5.1.3 Prove Explorer-present preview creates no compatibility identity.
- [ ] 5.1.4 Capture Explorer-free add/modify/tooltip/activate/context/delete and overflow UIA evidence.
- [ ] 5.1.5 Force host crash/restart and record resource/re-registration traces.
- [ ] 5.1.6 Create unique task-linked evidence and pass strict validation.

### 5.2 Package compatibility support

**目的：** outcome
**輸入：** inputs
**產出：** outputs
**依賴：** dependencies
**Owner／Wave：** Primary agent
**Gate／Evidence：** change evidence
**完成門檻：** package threshold

**?桃?嚗?* outcome
**頛詨嚗?* inputs
**?Ｗ嚗?* outputs
**靘陷嚗?* dependencies
**Owner嚗ave嚗?* Primary agent
**Gate嚗vidence嚗?* change evidence
**摰??瑼鳴?** package threshold

**Purpose:** Ship the updated host in standalone and combined products.
**Inputs:** Gate-passing release revision and existing NSIS manifests.
**Outputs:** Hashed standalone/combined installers and uninstall proof.
**Dependencies:** 5.1.
**Owner/Wave:** Primary agent / wave 6.
**Gate/Evidence:** `G-NOTIFY-COMPAT`, `G-TRACE`; packaging record.
**Completion threshold:** Both installers contain the updated host, build without launch and clean up all compatibility artifacts.

- [ ] 5.2.1 Update release/package/NSIS manifests and uninstall cleanup if required.
- [ ] 5.2.2 Build and hash standalone and combined installers without launching them.
