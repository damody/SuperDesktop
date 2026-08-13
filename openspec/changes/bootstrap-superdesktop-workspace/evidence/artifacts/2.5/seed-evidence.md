# 2.5 immutable validation seed

This immutable artifact anchors the valid evidence set copied into every 2.5
mutation fixture. Fixture outputs are written beneath `build/fixture-results`
and are only promoted after the entire matrix succeeds, so the seed hash cannot
be changed while a production validation is in progress.
