# Complete Input Method Flyout Design

## Goal

Make SuperDesktop's owned input-method flyout represent and activate the user's real Windows input profiles instead of approximating every profile as a keyboard layout. The result follows the supplied Windows language-switcher reference: distinct language and input-method names, recognizable glyphs, an active selection state, exact pointer/keyboard activation, and a working Language preferences footer.

## Chosen approach

Use the documented Text Services Framework (TSF) profile manager as the authoritative source for enabled input processor and keyboard-layout profiles. HKL-only enumeration remains a bounded fallback when TSF is unavailable, but it is no longer the preferred model. Registry-only name lookup was rejected because it cannot reliably distinguish multiple TSF profiles that share a language or substitute keyboard layout. Cosmetic-only changes were rejected because they would preserve incorrect names and ambiguous activation.

## Protocol model

`InputProfile` gains additive, defaulted fields for the authoritative input-method display name, profile kind, TSF class/profile identities, and substitute HKL where applicable. Existing `id`, `language_tag`, and `display_name` fields remain valid for old providers. Identifiers and text stay bounded, profile collections remain capped at 64 items, and duplicate stable IDs fail validation.

`ActivateInputProfile` continues to carry one opaque stable profile ID. A new `OpenLanguagePreferences` command has no caller-controlled path or URI. The host accepts only this fixed operation and returns an `Accepted` terminal because launching Settings is admission, not proof that the Settings UI became visible.

## Native provider

The Windows adapter creates `ITfInputProcessorProfileMgr` and `ITfInputProcessorProfiles` inside the existing scoped COM apartment. It enumerates enabled profiles for the installed languages, obtains TSF descriptions for input processors, derives localized language names from LANGID, and creates stable IDs from profile type, language, CLSID, profile GUID, and HKL identity. Keyboard layouts use a bounded registry display-name fallback only when TSF supplies no description.

Activation first re-enumerates the current profiles and matches the complete stable identity. TSF input processors are activated with `ActivateProfile`; keyboard-layout fallback retains the existing exact foreground-thread request and observation deadline. Stale, malformed, disabled, cross-session, or no-longer-installed identities fail before mutation.

Language preferences uses `ShellExecuteExW` with the compile-time constant `ms-settings:regionlanguage`. The adapter passes no user-controlled executable, verb, arguments, working directory, or URI. Failure remains a provider error and never delegates through `explorer.exe`.

## Owned UI

Each 72-DIP row shows an input-method-specific glyph, localized language name, and the authoritative input-method description. The active row retains the accent bar and selected surface. Rows keep stable IDs, Button semantics, accessible names including active state, pointer activation, Enter/Space parity, hover/pressed/focus visuals, long-text ellipsis, and light/dark/high-contrast tokens.

The footer is renamed to `Language preferences` / `語言喜好設定`, uses the Windows-style language icon, and emits the typed fixed command. The misleading local `keyboard_settings_open` page and raw profile-ID display are removed. Provider unavailable and launch failure remain explicit through the existing status reconciliation and trace paths.

Input flyout height remains content-driven but is clamped for one to six rows. If more profiles exist, the profile region scrolls while the Language preferences footer stays reachable.

## Data flow

TSF/keyboard snapshot → bounded protocol profile DTOs → reconciled app snapshot → owned GPUI profile rows → typed activation/settings action → isolated host → fresh identity admission → TSF activation or fixed Settings launch → terminal → authoritative snapshot refresh.

## Verification

Protocol tests cover old JSON defaults, new profile kinds/identities, maximum bounds, duplicates, malformed identities, command round trips, and Accepted validation. Platform tests cover TSF decoding, stable ID parse/round trip, fallback naming, disabled/stale rejection, exact activation source contracts, and real read-only profile enumeration. Host/app tests cover typed routing, deadlines, stale generations, Accepted-without-fake-observation, and snapshot reconciliation.

UI tests cover 0/1/64 profiles, duplicate-language methods, authoritative subtitles, glyph selection, long text, scrolling, selected state, footer action, pointer/Enter/Space parity, localization, themes, DPI, and accessibility. Headful verification at 168 DPI switches between two real profiles and restores the original, invokes Language preferences through UI Automation, verifies one owned flyout and focus behavior, and captures light/dark/high-contrast evidence without persisting user-specific input profile identities in committed reports.

## Scope limits

This change does not install, remove, reorder, or configure input methods; edit user language lists; accept arbitrary URIs; synthesize Win+Space; invoke `explorer.exe`; or claim that Settings opened merely because the launch was accepted.
