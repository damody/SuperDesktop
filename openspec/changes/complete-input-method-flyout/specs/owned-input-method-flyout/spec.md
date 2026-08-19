## ADDED Requirements

### Requirement: Bounded authoritative input profile snapshot
The system SHALL publish at most 64 enabled Windows input profiles with an opaque stable ID, localized language display name, authoritative input-method description, profile kind, and kind-appropriate exact TSF or HKL metadata. Every string and collection SHALL be bounded and old payloads without additive fields SHALL remain valid.

#### Scenario: Distinct methods share one language
- **WHEN** two enabled input processors share a language but have different TSF profile identities
- **THEN** the snapshot contains two unique IDs and their distinct authoritative method descriptions

#### Scenario: Legacy payload
- **WHEN** a reader deserializes an older profile containing only ID, language tag, and display name
- **THEN** additive fields receive safe legacy defaults and validation succeeds

#### Scenario: Maximum and malformed input
- **WHEN** native or serialized input exceeds 64 profiles, text limits, kind-specific identity rules, or repeats a stable ID
- **THEN** the provider truncates before publication or protocol validation rejects the snapshot without partial ambiguous state

#### Scenario: TSF unavailable
- **WHEN** TSF enumeration is unavailable but bounded HKL enumeration succeeds
- **THEN** the snapshot truthfully publishes legacy keyboard-layout profiles with fallback descriptions

### Requirement: Exact observed profile activation
The system MUST activate only an enabled profile whose complete opaque identity still matches a fresh enumeration in the current interactive session. Input processors SHALL use TSF activation; keyboard layouts SHALL retain exact foreground-thread activation and bounded observation. Success SHALL be reported as observed only after the authoritative active profile ID matches.

#### Scenario: Exact input processor activation
- **WHEN** a valid installed enabled TSF profile ID is requested before its deadline
- **THEN** the provider activates that exact class/profile/language identity and returns an observed terminal only after the active snapshot matches

#### Scenario: Exact keyboard activation
- **WHEN** a valid enabled keyboard profile ID is requested
- **THEN** the provider requests that exact HKL for the foreground thread and confirms the authoritative active ID before success

#### Scenario: Stale or malformed identity
- **WHEN** the ID is malformed, disabled, removed, cross-session, mismatched, oversized, expired, or stale across a host restart
- **THEN** the request fails before mutation and no other profile is selected

#### Scenario: Observation timeout
- **WHEN** Windows accepts an activation request but the active profile does not match before the deadline
- **THEN** the terminal is timeout/provider failure and the UI does not optimistically mark the target active

### Requirement: Fixed Language preferences action
The system SHALL expose one fieldless typed Language preferences command that can launch only the compile-time Windows region-and-language Settings URI. The system MUST NOT accept an executable, verb, URI, arguments, or working directory from the UI or protocol and MUST NOT invoke Explorer. Launch admission SHALL return Accepted without an observed snapshot generation.

#### Scenario: Settings launch accepted
- **WHEN** the user invokes Language preferences and Windows accepts the fixed URI launch
- **THEN** the host returns Accepted with no observed generation and the app does not claim Settings visibility

#### Scenario: Settings launch rejected
- **WHEN** Windows rejects the fixed URI launch or the command is expired/stale
- **THEN** the host returns a truthful failure terminal and no alternative executable or URI is attempted

#### Scenario: Arbitrary launch input absent
- **WHEN** protocol and production source contracts are inspected
- **THEN** no caller-controlled launch target exists and `explorer.exe` is absent from the route

### Requirement: Complete owned input-method presentation
The owned flyout SHALL render a scrollable bounded profile list and fixed Language preferences footer. Each profile row SHALL display method-specific glyph, localized language name, authoritative input-method description, and active state, with stable accessibility identity and equivalent pointer, Enter, and Space activation. The flyout SHALL retain Escape/focus-loss dismissal and light, dark, and high-contrast behavior.

#### Scenario: Populated duplicate-language list
- **WHEN** the snapshot contains multiple methods for one language
- **THEN** separate rows show their distinct method names and only the authoritative active ID has selected styling

#### Scenario: Empty or unavailable provider
- **WHEN** no profiles exist or the provider is unavailable
- **THEN** the flyout shows a truthful localized empty/unavailable state and does not expose fake activation

#### Scenario: Maximum list and long text
- **WHEN** 64 profiles or long bounded names are supplied
- **THEN** rows remain inside a scroll viewport, text ellipsizes, and the footer remains reachable without changing popup bounds

#### Scenario: Footer interaction
- **WHEN** the footer is invoked by pointer, Enter, Space, or UI Automation
- **THEN** all routes emit the same fieldless Language preferences action exactly once

#### Scenario: Theme DPI and dismissal
- **WHEN** the flyout is rendered in light, dark, or high contrast at supported DPI/taskbar-row combinations
- **THEN** content remains contained and legible, focus is visible, Escape dismisses, and window deactivation dismisses

### Requirement: Privacy-safe integrated evidence
Completion SHALL require passing protocol/platform/host/app/UI tests, controlled real-profile enumeration and switching/restoration where applicable, fixed Settings UIA admission, themed headful geometry/accessibility checks, full format/check/test/Clippy, strict OpenSpec validation, and privacy scanning. Committed evidence MUST NOT contain raw profile IDs, HKLs, TSF GUIDs, or user language-list identities.

#### Scenario: Controlled profile switch fixture exists
- **WHEN** at least two real installed profiles are available to the controlled headful fixture
- **THEN** the fixture switches to an alternate profile, observes it, restores the original, and records only redacted pass state

#### Scenario: Controlled profile switch not applicable
- **WHEN** fewer than two suitable profiles exist
- **THEN** mutation evidence is marked not-applicable with enumerated counts while all command-safety and UI gates still pass

#### Scenario: Privacy scan
- **WHEN** staged evidence and reports are scanned before completion
- **THEN** no raw input identity or user language-list value is present and live screenshots containing such data are not committed
