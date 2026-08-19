## Context

The current status provider enumerates `GetKeyboardLayoutList`, assigns locale tags as display names, and activates layouts by posting `WM_INPUTLANGCHANGEREQUEST`. This is safe for simple keyboard layouts but does not model Text Services Framework input processors, their authoritative descriptions, or multiple profiles within one language. The owned UI consequently guesses subtitles from language tags and implements its footer as a local detail toggle.

The implementation spans the versioned protocol, Windows COM and launch adapters, isolated status host, app reconciliation, GPUI presentation, and headful verification. Existing limits, generations, deadlines, focus-loss dismissal, Start-focus restoration, theme tokens, and Explorer-free ownership remain constraints.

## Goals / Non-Goals

**Goals:**

- Publish a bounded additive snapshot of enabled TSF input processors and keyboard layouts with authoritative names and stable exact identities.
- Activate only a freshly revalidated installed profile, preserving session, deadline, and authoritative observation fences.
- Provide a typed fixed Language preferences action without arbitrary launch input or fake visibility claims.
- Match the supplied Windows row/footer behavior with scrolling, localization, keyboard/UIA parity, and theme/DPI support.
- Produce privacy-safe evidence from real profile enumeration, controlled switching/restoration, Settings admission, and headful themes.

**Non-Goals:**

- Installing, removing, ordering, enabling, disabling, or configuring input methods.
- Editing the user language list, accepting caller-controlled URIs, invoking Explorer, or synthesizing Win+Space.
- Claiming that an Accepted activation or Settings launch is already authoritatively observed.

## Decisions

### Additive exact profile contract

`InputProfile` retains its legacy fields and gains defaulted `input_method_name`, `kind`, and exact TSF/HKL metadata. `InputProfileKind` is an additive enum. A stable opaque `id` encodes versioned profile type and exact identity; consumers display names but issue commands only with the opaque ID. Protocol validation bounds every string, requires identity fields appropriate to the kind, caps profiles at 64, and rejects duplicate IDs. Old JSON defaults to a legacy keyboard-layout representation.

Alternative: replace HKL IDs outright. Rejected because additive compatibility is required and stale clients/providers must fail safely rather than deserialize ambiguously.

### TSF-first enumeration with bounded HKL fallback

`ITfInputProcessorProfileMgr::EnumProfiles` enumerates enabled input processors and keyboard layouts for installed language IDs. `ITfInputProcessorProfiles::GetLanguageProfileDescription` supplies input processor descriptions. Keyboard layout text uses a bounded read-only registry lookup when TSF has no description. Every COM interface and BSTR remains scoped; native enumeration is capped before conversion. If TSF is unavailable, current HKL enumeration remains a truthful fallback and marks profiles as legacy keyboard layouts.

Alternative: registry-only lookup. Rejected because registry keyboard-layout keys do not describe all TSF profiles or activation identities.

### Fresh exact activation

The provider parses the opaque ID, re-enumerates current profiles, and requires one exact enabled match. Input processors use `ActivateProfile`; keyboard layouts use their exact HKL and the existing foreground-thread/session request plus observed-profile deadline. Stale or mismatched class/profile/HKL identities never reach mutation. The command terminal is `Observed` only after the status snapshot confirms the requested stable ID; otherwise it is a truthful provider failure or timeout.

### Fixed Language preferences admission

The protocol adds `OpenLanguagePreferences` without fields. The platform adapter calls `ShellExecuteExW` with a compile-time `ms-settings:regionlanguage` target and no caller-controlled verb, arguments, directory, executable, or URI. The host returns `Accepted` without an observed generation and schedules no fabricated input snapshot. `explorer.exe` is never launched.

### Owned list and fixed footer

The input flyout renders a bounded scroll region for rows and a fixed footer. Language primary text comes from the provider language display name; method subtitle comes from the authoritative method description. Glyphs are selected from profile kind and known method description with a neutral keyboard fallback. The active accent/selection, Button roles, stable IDs, pointer/Enter/Space parity, focus styles, ellipsis, Escape and deactivation dismissal remain. The local `keyboard_settings_open` state and raw-ID page are removed.

## Data flow and component boundaries

Windows TSF/registry/HKL read → `platform-win` owned profile model → protocol DTO validation → status-host snapshot → app reconciler → `taskbar-ui` rows. Row activation → opaque ID command → host deadline/generation admission → platform fresh identity match → TSF/HKL operation → observed snapshot. Footer activation → fieldless typed command → fixed Settings URI adapter → Accepted terminal.

## Failure handling and observability

TSF service, registry description, or Settings launch failures return bounded provider messages. Registry-name failure falls back to a bounded language/keyboard label and does not remove an otherwise valid profile. Duplicate/oversized/malformed profiles reject the snapshot. Host restarts clear stale terminals; UI provider-unavailable state remains visible. Action traces distinguish activation observed, Settings accepted, rejected, timeout, and provider failure without logging raw profile IDs.

## Security and privacy gates

- **G-IME-IDENTITY:** stale, malformed, disabled, cross-session, or mismatched identities cannot mutate input state.
- **G-IME-LAUNCH:** production source contains only the fixed region-language URI and no Explorer or caller-provided launch field.
- **G-IME-BOUNDS:** every native list/string and protocol collection is bounded and owned.
- **G-IME-TRUTH:** Accepted never advances snapshot generation or claims Settings visibility.
- **G-IME-PRIVACY:** committed evidence contains no raw HKL, TSF GUID, profile ID, or user language-list identity.
- **G-IME-UI:** 0/1/64 rows, scroll/footer reachability, accessibility, locale, themes and DPI pass.

## Testing and evidence

Focused protocol, platform, host, app, and UI logs are hashed in per-layer evidence indexes. Live enumeration records counts/kinds only. Controlled switching uses two already-installed profiles, records only before/after equality and restoration, and is evidence-backed not-applicable if fewer than two profiles exist. Headful light/dark/high-contrast runs invoke rows and footer through UIA, restore the original profile, verify geometry/focus/dismissal, and commit only redacted reports. Full format, locked all-target check/test, warnings-as-errors Clippy, strict OpenSpec, privacy scan, and task-evidence completeness block completion.

## Migration Plan

1. Land additive DTO/command fields and protocol tests.
2. Land TSF enumeration/activation and fixed launch adapter behind current host boundaries.
3. Advance protocol minor version, route host/app actions, and retain legacy fallback.
4. Replace the local footer page and enable the scrollable owned list.
5. Run controlled real-profile and headful matrices, then full quality gates.

Rollback removes the additive producer/UI use while old JSON defaults preserve deserialization. No persistent settings or input configuration is migrated.

## Planning adjustments

- **A — task refinement:** task ordering, command, owner, or split may change without changing scope, contracts, gates, or evidence requirements.
- **B — design/spec correction:** an in-scope technical correction pauses affected work and updates design, spec, tasks, and stale evidence before revalidation.
- **C — material change:** new scope, public behavior, permission, platform, external write, arbitrary launch input, or weakened gate requires user approval. No gate or evidence threshold may be silently reduced.

## Open Questions

None. If TSF does not expose a usable description for a valid keyboard profile, the specified bounded fallback is authoritative for this change.
