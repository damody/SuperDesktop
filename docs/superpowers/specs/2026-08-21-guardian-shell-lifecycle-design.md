# Guardian and shell lifecycle stabilization

## Goal

SuperDesktop shall complete its guardian handshake before terminating Explorer, retain a recoverable route to the default Explorer shell, and treat expected AppBar unavailability under an owned shell as a traceable degraded mode rather than a console warning.

## Root causes

- The guardian validates both immutable file identity and an exact canonical-path string. Windows may expose the same executable with an extended `\\?\` prefix or different casing, producing a false `WrongExecutable` rejection.
- The parent waits only for an acknowledgement file and converts every early guardian rejection into `child-acceptance-timeout`, hiding the actionable cause.
- If the registry already contains SuperDesktop but the rollback record is absent, registration returns early and never reconstructs a safe default-Explorer rollback record.
- Owned-shell geometry already handles unavailable AppBar registration correctly, but the expected fallback is still written to stderr as a warning.

## Design

### Guardian identity and acceptance

Immutable volume/file identity remains mandatory. Path comparison is retained as defense in depth but compares normalized Windows path identities: extended prefixes are removed and comparison is case-insensitive. The acknowledgement deadline becomes five seconds. While waiting, the parent also observes the guardian process handle and reports an early child exit distinctly instead of waiting to a misleading timeout. The guardian preserves the typed `LeaseReject` reason in its console error.

### Rollback reconstruction

When the rollback record is missing, SuperDesktop may reconstruct it only when the observed registry value exactly identifies the current SuperDesktop executable and explicit owned-shell arguments. The reconstructed prior value is `explorer.exe`, matching the user-facing “Return to default Explorer” command. Unknown or third-party shell values remain fail-closed and are never overwritten. Restoration is idempotent: an already-default Explorer value succeeds even without a record, and a successful restore removes the record.

### AppBar degraded mode

AppBar failure keeps the existing monitor-bounds geometry and trace markers. It no longer emits a console warning because this is an expected owned-shell capability outcome. True taskbar configuration failures continue to reach the console.

## Failure handling and observability

- Guardian errors expose the precise lease rejection and the parent distinguishes early exit from timeout.
- Rollback reconstruction records an explicit trace and never applies to an unrecognized shell.
- No lifecycle failure may close Explorer before guardian acceptance.
- Normal AppBar fallback remains visible in the action trace for tests and diagnostics.

## Verification

- Unit tests cover extended-prefix/case path equivalence, different-file rejection, early child exit, bounded timeout, missing-record reconstruction, unknown-shell refusal, idempotent Explorer restore, and trace-only AppBar fallback.
- A release guardian-parent fixture must create the acceptance record and verified recovery terminal without `guardian-lease-validation` or `child-acceptance-timeout`.
- A bounded owned-shell lifecycle run must leave Explorer recoverable, avoid all four reported console messages, and preserve the AppBar fallback trace.
- Workspace tests, Clippy with warnings denied, release build, strict OpenSpec validation, and installer build must pass.

## Scope

This change does not weaken guardian anti-spoof checks, delegate taskbar geometry to Explorer, or overwrite unknown third-party shell registrations.
