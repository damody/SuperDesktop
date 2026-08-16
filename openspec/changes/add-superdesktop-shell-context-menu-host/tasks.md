# Add SuperDesktop Shell Context Menu Host

## 1. Menu contracts and sanitization

### 1.1 Extend the provider protocol

**目的：** Define bounded enumeration and invocation messages without native pointers.
**輸入：** Provider protocol v1, command DTOs, desktop selection identity.
**產出：** Menu context, descriptors, tokens, enrichment and invocation responses.
**依賴：** `extend-superdesktop-shell-contracts`; desktop operation contracts.
**Owner／Wave：** Primary integrator / Wave 3.
**Gate／Evidence：** `G-CONTEXT-MENU-HOST`, `G-SAFETY`; protocol tests.
**完成門檻：** Round trips are deterministic and invalid depth/cardinality/text/tokens fail closed.

- [x] 1.1.1 Add menu enumeration/invocation DTOs and selection fingerprints.
- [x] 1.1.2 Add recursive descriptor sanitization and configured limits.
- [x] 1.1.3 Add compatibility and negative fixtures.

### 1.2 Build the GPUI-owned menu model

**目的：** Render and navigate sanitized menus independently of provider lifetime.
**輸入：** Validated command trees and existing desktop interaction model.
**產出：** Menu state, focus/submenu navigation, accessibility nodes, typed invocation effects.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 3.
**Gate／Evidence：** `G-CONTEXT-MENU-HOST`, `G-A11Y-I18N`; model tests.
**完成門檻：** Pointer, keyboard, and accessibility routes are equivalent and disabled commands never invoke.

- [x] 1.2.1 Add built-in command composition by selection capability.
- [x] 1.2.2 Add focus, submenu, dismissal, and accessibility semantics.
- [x] 1.2.3 Replace the desktop context-menu deferred action with a typed request.

## 2. Isolated host dispatch and verification

### 2.1 Implement host enumeration and invocation

**目的：** Keep optional provider work outside GPUI and reject stale invocation.
**輸入：** 1.1 protocol and provider-host dispatcher.
**產出：** Context-menu dispatch, generation/token registry, deadline/cancel/health behavior.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 3.
**Gate／Evidence：** `G-CONTEXT-MENU-HOST`, `G-SAFETY`; host integration tests.
**完成門檻：** Built-ins return immediately, optional enrichment is bounded, and stale tokens cannot execute.

- [x] 2.1.1 Add deterministic built-in enumeration in the provider host.
- [x] 2.1.2 Add generation-bound token registry and invocation admission.
- [x] 2.1.3 Add timeout, cancellation, crash/fallback, and stale-token tests.

### 2.2 Verify integration

**目的：** Prove menu behavior and isolation without regressing the workspace.
**輸入：** 1.2 UI model and 2.1 host behavior.
**產出：** Verification evidence and completed OpenSpec tasks.
**依賴：** 1.2 and 2.1.
**Owner／Wave：** Primary integrator / Wave 3 exit.
**Gate／Evidence：** `G-CONTEXT-MENU-HOST`, `G-TRACE`; cargo/OpenSpec logs.
**完成門檻：** Formatting, check, clippy, tests, strict spec, and task validation all pass.

- [x] 2.2.1 Add end-to-end enumerate/navigate/invoke/fallback contract tests.
- [x] 2.2.2 Run all Rust and OpenSpec validation and record evidence.
