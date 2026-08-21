## Context

SuperDesktop registers itself as the per-user Windows shell, arms a restricted guardian, and only then closes Explorer. The guardian receives exactly two inherited handles, validates a sealed parent identity, acknowledges acceptance, waits for the exact parent, and recovers Explorer after abnormal termination. Current validation additionally compares canonical path strings byte-for-byte, which rejects equivalent Windows extended-path or casing variants. The parent polls only an acknowledgement file for two seconds, so an early child rejection becomes a misleading timeout. Separately, an already-owned shell with a missing rollback file cannot return to Explorer, and expected AppBar fallback is printed as a warning even though geometry remains valid.

The lifecycle is security-sensitive: implementation must preserve exact file identity, explicit handle inheritance, fail-closed behavior for third-party shell registrations, and the ordering registration → guardian acceptance → Explorer shutdown.

## Goals / Non-Goals

**Goals:**

- Accept path-spelling variants only when immutable executable file identity also matches.
- Distinguish guardian early exit from a genuine bounded acceptance timeout.
- Ensure an exact owned-shell registration always has an Explorer recovery record before guardian arming.
- Make default-Explorer restoration idempotent and refuse unknown shells.
- Keep AppBar fallback observable through traces without routine stderr noise.
- Prove lifecycle behavior against a release candidate and installer.

**Non-Goals:**

- Removing or bypassing guardian validation.
- Restoring an unknown previous third-party shell when no rollback record exists.
- Delegating taskbar geometry or shell UI back to Explorer.
- Hiding genuine registration, guardian, recovery, or taskbar configuration errors.

## Decisions

### Normalize path spelling but retain immutable identity

Path equality will strip Windows extended DOS prefixes and compare case-insensitively. Validation still requires matching volume serial and file index. This is preferred over dropping path checks entirely and over requiring raw string equality, which is incompatible with valid Win32 path representations.

### Observe both acknowledgement and child lifetime

The parent admission loop will use a five-second upper bound and test the guardian process handle while polling the nonce-bound acknowledgement. A signalled child before acceptance returns `child-exited-before-acceptance`; only a live child that misses the full bound returns `child-acceptance-timeout`. The guardian error retains the typed `LeaseReject` variant. This is preferred over merely increasing the old timeout because it exposes the actual failure class.

### Reconstruct only an exact owned-shell rollback

Before returning early for an already-correct registration, SuperDesktop will ensure the rollback store exists. If absent, it writes a record whose prior value is `explorer.exe` only when the observed value exactly equals the current admitted SuperDesktop shell command. Restoration without a record succeeds immediately when Explorer is already registered; it reconstructs the same record for the exact owned value; every other value is rejected. This provides deterministic “Return to default Explorer” behavior without overwriting unknown shells.

### Treat AppBar fallback as an expected capability result

The existing fallback trace markers and owned monitor geometry remain. The direct stderr warning is removed and a source test asserts that actual configuration failures still report through error paths.

### Evidence corrections

Implementation may refine test commands and split leaves as an A-level adjustment. A correction to normalization, rollback eligibility, handshake ordering, or console semantics is B-level and requires design/spec/task revalidation. Weakening anti-spoof identity checks, overwriting unknown shell values, extending permissions, or reducing blocking gates is C-level and requires user approval.

## Risks / Trade-offs

- **Risk: path normalization becomes too permissive** → Require immutable volume/file identity after normalization and test a distinct-file negative case.
- **Risk: reconstructed rollback discards a custom prior shell** → Reconstruct only for the exact owned SuperDesktop value and document that missing history means “default Explorer,” never an inferred third party.
- **Risk: longer acceptance wait delays failure** → Cap at five seconds and return immediately when the child exits.
- **Risk: AppBar failures become invisible** → Preserve deterministic action-trace markers; suppress only the expected direct warning.

## Migration Plan

1. Ship the guardian and app changes together in one installer.
2. On first startup after update, reconstruct the rollback record only if the registry already contains the exact installed SuperDesktop command.
3. Arm and verify the updated guardian before closing Explorer.
4. On any failure, restore Explorer from the verified or reconstructed record.
5. Rollback uses the same default-Explorer restoration path and does not require the updated guardian to remain running.

## Open Questions

None. The explicit product command is “Return to default Explorer,” so `explorer.exe` is the required missing-history fallback for an exact owned registration.
