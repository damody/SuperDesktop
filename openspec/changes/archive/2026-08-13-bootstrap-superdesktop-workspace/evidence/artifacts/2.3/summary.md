# Evidence schema and validator evidence

L2 2.3 establishes a change-local, append-only evidence system for all 26 bootstrap leaf tasks. `schema.json` defines globally namespaced task records and `coverage-schema.json` defines the versioned coverage model. `coverage.json` assigns each task a stable capability, requirement, scenario and gate mapping; all leaves are mandatory.

`index.jsonl` contains 28 records: the 26 completed leaves plus two passed replacements. The original evidence for `1.1.3` and `1.1.4` is marked stale and is linked to coverage-identical passed replacements, satisfying the two A-level adjustment ledger entries. All artifacts are SHA-256 checked.

`validate-evidence.ps1` validates schema fields, global IDs, append-only duplicate identities, coverage lookup/drift, artifact presence and hashes, checkbox consistency, mandatory status, replacement graph integrity, and A/B/C adjustment stale propagation. The positive invocation passes. The fixture matrix records 14 expected failures, including mandatory non-pass/stale conditions, missing artifact, coverage errors, dangling/cyclic/nonmandatory/unpassed replacements, duplicate identity, and stale-propagation failure.

G-TRACE is passed provisionally for this child change; Primary integrator must independently inspect the index, contract hash, and evidence before Wave 1 acceptance.
