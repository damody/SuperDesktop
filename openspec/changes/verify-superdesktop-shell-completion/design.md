## Context

Eight completion changes now implement shared provider contracts, desktop file operations, isolated context menus, Start/search, advanced taskbar interactions, notification-area hosting, documented virtual-desktop controls, and a transactional installer. Their local evidence uses intentionally different domain-specific shapes. A final verifier must prove coverage without converting missing physical or independent evidence into a pass.

## Goals / Non-Goals

**Goals:** Define a versioned roll-up contract, validate every child artifact, execute cross-domain tests, preserve raw commands and capability limitations, and compute a fail-closed release disposition.

**Non-Goals:** Invent evidence for unavailable hardware, claim undocumented virtual-desktop APIs, treat an owned notification host as legacy Explorer tray compatibility, mutate the login shell, or archive changes.

## Decisions

1. Add a deterministic `CompletionRollup` model in `superdesktop-test-support`; every required child, local gate, and external gate has an explicit disposition.
2. A PowerShell collector validates child identity/result fields and emits one JSON roll-up. Missing, malformed, duplicate, or contradictory evidence is terminal failure.
3. Local automated verification may pass functional, accessibility/i18n fixtures, virtual DPI geometry, performance/resource bounds, safety, architecture, and traceability gates.
4. Windows 10 build 19045 shell takeover/recovery/installer reboot, physical mixed-DPI interaction, and independent review remain blocking `external_pending` gates until their artifacts exist.
5. `release_allowed` is derived only: every required gate must be `passed`; optional unsupported features are recorded as limitations, not failures or parity claims.

## Risks / Trade-offs

- [Evidence formats differ] → Normalize only the small common envelope and preserve source paths/hashes in the roll-up.
- [Green unit tests hide hardware gaps] → External gates are mandatory and cannot be waived by the local collector.
- [Feature overclaim] → Publish an explicit capability/limitation ledger beside gate dispositions.
- [Stale child evidence] → Validate the exact expected change set and rerun workspace/OpenSpec commands before roll-up.

## Migration Plan

Land the model, schema, collector, and local roll-up first. Run the collector after all child changes validate. Attach physical and reviewer artifacts later without changing gate semantics; recompute the derived disposition after each addition.

## Open Questions

None.
