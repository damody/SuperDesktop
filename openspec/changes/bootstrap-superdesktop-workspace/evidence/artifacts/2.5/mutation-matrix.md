# Isolated production mutation matrix

The immutable seed is copied once for run `recovery-audit-6`. Each of the 20
cases receives a new copy under `build/fixture-results/recovery-audit-6/cases/`.
The runner invokes `scripts/validate-evidence.ps1 -EvidenceRoot <case>` for every
case, requires exit code 1, and asserts the named semantic diagnostic. Raw results
are promoted only after all 20 pass; production evidence is never mutated during a
run.

Covered cases: mandatory blocked/not-applicable; missing artifact; missing,
unknown and drifted coverage; dangling, cyclic, nonmandatory, drifted and
unpassed replacements; duplicate identities; stale-propagation adjustments;
missing procedure, type, pattern and date-time schema errors; dangling and
malformed adjustment records.
