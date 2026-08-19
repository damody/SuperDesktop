## 1. Protocol foundation

### 1.1 Additive exact input-profile contracts

**目的：** Define compatible bounded DTOs and commands for authoritative TSF/HKL profiles and fixed Language preferences admission.
**輸入：** Approved design, existing system-status protocol, collection/text limits.
**產出：** Additive profile fields/kind, fieldless settings command, validation and round-trip tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** G-IME-BOUNDS, G-IME-TRUTH, G-IME-LAUNCH; `evidence/protocol/evidence-index.json`.
**完成門檻：** Old/new JSON, bounds, kind identities, duplicates, commands, and terminal semantics pass with no arbitrary launch field.

- [x] 1.1.1 Add defaulted input-method description, kind, and exact TSF/HKL metadata to `InputProfile`.
- [x] 1.1.2 Define versioned stable profile identity formatting and kind-specific protocol validation.
- [x] 1.1.3 Add fieldless `OpenLanguagePreferences` and retain opaque `ActivateInputProfile` command validation.
- [x] 1.1.4 Advance Accepted terminal tests so Settings admission cannot carry an observed generation.
- [x] 1.1.5 Add old/new JSON round-trip, 0/1/64/65, duplicate, malformed, oversized, and command tests.
- [x] 1.1.6 Run focused protocol tests and hash/index the log with unique task subchecks.

## 2. Native Windows profile provider

### 2.1 Bounded TSF and keyboard profile enumeration

**目的：** Publish authoritative enabled profile names and identities with safe fallback behavior.
**輸入：** Work package 1.1, Windows TextServices/Registry bindings, existing COM apartment/HKL adapter.
**產出：** Exact Windows feature flags, scoped TSF enumerator, registry-name fallback, snapshot integration and tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** G-IME-BOUNDS, G-IME-PRIVACY; `evidence/platform/enumeration.json` and `evidence/platform/evidence-index.json`.
**完成門檻：** Enabled TSF/keyboard profiles are unique, bounded, accurately named, active-ID complete, and fallback/unavailable states are truthful.

- [x] 2.1.1 Enable the exact Windows TextServices binding without adding a crate.
- [x] 2.1.2 Implement scoped `ITfInputProcessorProfileMgr` and `ITfInputProcessorProfiles` creation in the existing COM apartment.
- [x] 2.1.3 Enumerate bounded installed language IDs and enabled TSF input processor/keyboard profiles.
- [x] 2.1.4 Resolve authoritative TSF method descriptions and bounded keyboard-layout fallback names.
- [x] 2.1.5 Build deterministic exact stable IDs and deduplicate/sort profiles at the 64-item cap.
- [x] 2.1.6 Integrate TSF-first enumeration with truthful HKL fallback and exact active-profile selection.
- [x] 2.1.7 Add decoder/count, duplicate, disabled, name fallback, active, cap, and source-ownership tests.
- [x] 2.1.8 Run live read-only enumeration and save redacted counts/kinds/fallback state only.

### 2.2 Fresh exact activation and fixed Settings launch

**目的：** Admit only exact current input profiles and the one fixed region-language Settings action.
**輸入：** Work package 2.1 fresh enumeration, current foreground/session fences, fixed launch design.
**產出：** TSF/HKL activation adapter, observation logic, fixed ShellExecute adapter and fail-closed tests.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** G-IME-IDENTITY, G-IME-LAUNCH, G-IME-TRUTH; `evidence/platform/commands.json`.
**完成門檻：** Malformed/stale/mismatched profiles never mutate; exact activation observes the requested ID; launch target is compile-time fixed; controlled mutation is passed or evidence-backed not-applicable.

- [x] 2.2.1 Parse and fresh-match every versioned profile identity component before mutation.
- [x] 2.2.2 Activate exact TSF input processor identities with current-session admission.
- [x] 2.2.3 Preserve exact HKL foreground activation and confirm the new stable active ID before success.
- [x] 2.2.4 Implement fixed `ms-settings:regionlanguage` ShellExecute admission with no caller fields or Explorer route.
- [x] 2.2.5 Add empty, oversized, malformed, stale, disabled, mismatched, cross-session, timeout, and no-observation tests.
- [x] 2.2.6 Add fixed-launch source/adapter tests for URI, parameters, Accepted semantics, and rejection.
- [x] 2.2.7 Run controlled two-profile switch/restore when available, otherwise record redacted not-applicable evidence.

## 3. Host and app integration

### 3.1 Isolated host routing and reconciliation

**目的：** Route exact activation and fixed Settings admission while preserving host generations and terminal truth.
**輸入：** Protocol 1.1 and platform packages 2.1–2.2.
**產出：** Host routing, minor compatibility update, terminal/event handling and tests.
**依賴：** 1.1, 2.1, 2.2.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** G-IME-IDENTITY, G-IME-TRUTH, G-IME-LAUNCH; `evidence/host/evidence-index.json`.
**完成門檻：** Commands route once, activation reports only observed authoritative state, Settings reports Accepted, and stale/deadline/restart paths fail safely.

- [x] 3.1.1 Route exact activation to the fresh platform adapter and retain authoritative Input reconciliation.
- [x] 3.1.2 Route fixed Language preferences to the launch adapter and return Accepted without generation.
- [x] 3.1.3 Advance protocol minor compatibility while retaining major-version admission.
- [x] 3.1.4 Add observed/accepted/rejected/stale/deadline/restart/duplicate terminal host tests.
- [x] 3.1.5 Run focused host tests and hash/index the log.

### 3.2 UI action mapping and snapshot truth

**目的：** Map owned UI actions to typed commands and update selection only from reconciled snapshots.
**輸入：** Work package 3.1 and existing status client/flyout lifecycle.
**產出：** UI action variant, app mapper, Accepted/no-fake-success handling and tests.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** G-IME-TRUTH, G-IME-LAUNCH; `evidence/app/evidence-index.json`.
**完成門檻：** Footer maps to the fieldless command, activation keeps exact opaque ID, Accepted never changes active selection, and failures remain truthful.

- [x] 3.2.1 Add `OpenLanguagePreferences` UI action and map it to the fieldless protocol command.
- [x] 3.2.2 Keep exact activation IDs and bounded client deadlines across action mapping.
- [x] 3.2.3 Reconcile active selection only from immediate/periodic authoritative snapshots.
- [x] 3.2.4 Add mapper, no-fake-observation, provider-failure, timeout, and lifecycle tests.
- [x] 3.2.5 Run focused app tests and hash/index the log.

## 4. Complete owned input-method UI

### 4.1 Scroll list, authoritative rows, and working footer

**目的：** Deliver the supplied Windows-style workflow with complete accessible interaction.
**輸入：** Reconciled authoritative profile DTOs/actions and existing flyout theme/focus tokens.
**產出：** Scrollable profile list, exact method glyph/text/selection, fixed footer and UI tests.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator / wave 6.
**Gate／Evidence：** G-IME-UI, G-IME-TRUTH; `evidence/ui/evidence-index.json`.
**完成門檻：** Duplicate-language methods remain distinct; 0/1/64, long text, scroll/footer, theme, locale, keyboard/UIA, geometry, and unavailable tests pass.

- [x] 4.1.1 Remove the misleading local keyboard-settings state and raw-ID detail page.
- [x] 4.1.2 Render provider language names and authoritative method descriptions without tag-based guessing.
- [x] 4.1.3 Select method-specific glyphs from profile kind/description with a neutral fallback.
- [x] 4.1.4 Keep exact active accent/selection and pointer/Enter/Space activation on every row.
- [x] 4.1.5 Add a bounded scroll viewport for rows while keeping the footer reachable.
- [x] 4.1.6 Render localized `Language preferences` footer with stable Button/UIA identity and exact typed action.
- [x] 4.1.7 Preserve Escape/deactivation dismissal, focus visuals, ellipsis, themes, and provider-unavailable states.
- [x] 4.1.8 Add 0/1/64, duplicate-language, long-name, scroll, action, theme, locale, DPI/taskbar-row, and accessibility tests.
- [x] 4.1.9 Run focused UI tests and hash/index the log.

## 5. Integrated headful and privacy validation

### 5.1 Real Windows interaction matrix

**目的：** Prove exact switching, restoration, footer admission, geometry, focus and themes without leaking user input identities.
**輸入：** All implementation packages, release binaries, real installed profiles, headful script and redaction rules.
**產出：** Redacted controlled-switch report, light/dark/high-contrast UIA reports, privacy scan and evidence index.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 7.
**Gate／Evidence：** G-IME-IDENTITY, G-IME-LAUNCH, G-IME-UI, G-IME-PRIVACY; `evidence/headful/evidence-index.json`.
**完成門檻：** Original profile is restored, fixed footer UIA invokes once, popup/focus geometry passes in every theme, identities are redacted, and conditional mutation has a valid disposition.

- [x] 5.1.1 Build release app/host and record content hashes.
- [x] 5.1.2 Run real profile enumeration and record only count/kind/fallback summaries.
- [x] 5.1.3 Switch to an alternate real profile, observe it, and restore the original, or record valid not-applicable.
- [x] 5.1.4 Invoke Language preferences through UIA and verify one fixed admission without claiming visibility.
- [x] 5.1.5 Capture and inspect light, dark, and high-contrast row/footer/focus/scroll states at 168 DPI.
- [x] 5.1.6 Verify Escape and window-deactivation dismissal and single-owned-popup exclusivity.
- [x] 5.1.7 Scan staged reports/artifacts for raw HKL, TSF GUID, profile ID, language-list identity, and Explorer route.
- [x] 5.1.8 Hash/index headful reports with unique task subchecks and omit identity-bearing screenshots from commit.

## 6. Final quality and completion

### 6.1 Blocking quality gates and evidence closure

**目的：** Prove the full change is apply-complete with no weakened gate or unresolved high-priority issue.
**輸入：** Work packages 1.1–5.1 and all current evidence indexes.
**產出：** Full quality logs, strict validation, complete task/evidence trace and final commit.
**依賴：** 5.1.
**Owner／Wave：** Primary integrator / wave 8.
**Gate／Evidence：** All gates; `evidence/quality/evidence-index.json` and `evidence/completion.json`.
**完成門檻：** Format, locked all-target check/test, warnings-as-errors Clippy, strict OpenSpec, detailed-task validation, evidence hashes, and privacy scan pass with every leaf passed or valid conditional not-applicable.

- [x] 6.1.1 Run format check and locked workspace all-target compilation.
- [x] 6.1.2 Run affected and full locked workspace tests.
- [x] 6.1.3 Run locked workspace all-target Clippy with warnings denied.
- [x] 6.1.4 Validate every JSON report, artifact hash, task ID/subcheck, and gate mapping.
- [x] 6.1.5 Run strict OpenSpec and detailed-task validation with no incomplete marker or contradiction.
- [x] 6.1.6 Confirm no failed, blocked, stale, P0, or P1 item and every conditional leaf has evidence-backed disposition.
- [ ] 6.1.7 Commit the implementation/evidence without unrelated worktree files.
