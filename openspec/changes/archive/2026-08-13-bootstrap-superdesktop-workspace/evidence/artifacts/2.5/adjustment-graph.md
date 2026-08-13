# Adjustment successor graph

`B-W1-2.5-SUCCESSOR` supersedes `A-W1-1.2-001`, `A-W1-1.2-003`,
`B-W1-EXIT-001`, `B-W1-EXIT-001-lineage`, `B-W1-EXIT-002`, and
`B-W1-EXIT-003`. It preserves the twelve immutable stale-to-passed mappings in
its record. The production validator resolves supersession by DFS, rejects
dangling edges and indirect cycles, verifies every stale/replacement backlink,
and requires every named legacy adjustment to reach a passed successor.
