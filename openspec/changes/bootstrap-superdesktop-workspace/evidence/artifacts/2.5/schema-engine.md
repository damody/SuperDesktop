# Draft 2020-12 schema execution

The production validator invokes `superdesktop-test-support validate-json-schema`,
which uses `jsonschema 0.37.1` with `Draft202012`, format assertions enabled, and
workspace `default-features = false`. The engine accepted both the evidence-record
schema instance and the coverage manifest (`jsonschema-engine-record.txt` and
`jsonschema-engine-coverage.txt`). Mutation cases prove required fields, type,
pattern, date-time format, arrays and `additionalProperties` are enforced by that
same production path.
