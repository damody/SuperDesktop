# SuperDesktop evidence governance

Evidence is append-only JSONL. A record identity is `task_id#subcheck`; a later result MUST use a new subcheck and link through `replaces`/`superseded_by`, never overwrite an existing result.

Every task has a globally namespaced ID: `<change-name>/<L3-id>`. Coverage uses stable lowercase kebab-case capability, requirement, and scenario slugs. Each record must match its coverage mapping, reference an existing artifact whose SHA-256 matches, declare every mapped gate, name a reviewer, and use an ISO-8601 timestamp.

Mandatory tasks cannot be `not-applicable`, `blocked`, or `stale` unless a linked, mandatory, fully covered replacement has passed. A passed task checkbox is valid only when a passing record (or valid passed replacement) covers it. A/B/C adjustments declaring stale evidence force the affected record into stale lineage and require a passed replacement.
