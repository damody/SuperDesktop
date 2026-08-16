## Context

The completion program spans eight implementation changes plus one verification change layered over the existing M0 shell foundation. Production implementation is locally complete, but physical Windows 10, mixed-DPI, reboot rollback, and independent-review gates are not available on this workstation. The parent change must distinguish implementation completion from release approval.

## Goals / Non-Goals

**Goals:** Freeze dependency order and ownership, aggregate exact child status, publish a truthful capability ledger, preserve M0 safety invariants, and derive program/release dispositions.

**Non-Goals:** Duplicate child implementation, weaken external gates, claim undocumented Windows behavior, auto-install the shell, archive any change, or label locally tested behavior as physical evidence.

## Decisions

1. The fixed order is contracts → desktop operations/context menu → Start/taskbar → notification area/virtual desktops → installer → verification.
2. Implementation completion and release approval are separate derived fields. The former requires every production child locally complete; the latter additionally requires verification external gates.
3. The program ledger records three claim classes: implemented, implemented-owned-protocol, and unavailable/not-claimed.
4. Existing explicit takeover opt-in, guardian identity binding, Explorer recovery, bounded queues, isolated providers, and installer rollback remain non-negotiable invariants.
5. The parent stays unarchived until the user requests archive after every child and external gate is complete.

## Risks / Trade-offs

- [Parent says complete while verification is pending] → Separate `implementation_complete` and `release_allowed` fields.
- [Broad “same as Windows” language overclaims compatibility] → List exact limitations for legacy tray protocols and undocumented virtual-desktop operations.
- [Child drift] → Record exact expected change names, commits, tasks, and dispositions in a machine-readable roll-up.
- [Safety regression during final evidence] → Physical collectors retain explicit opt-in/fingerprint/host gates and exact rollback.

## Migration Plan

Commit the local program roll-up after every production child and local verification pass. Later attach external evidence, complete the remaining verification tasks, recompute both roll-ups, and only then set release approval. Archive remains a separate explicit action.

## Open Questions

None.
