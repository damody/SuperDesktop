# Contract verifier

The verifier parses every SHA-256 manifest line, resolves the declared path below its canonical repository root, rejects escapes/missing inputs/hash drift, and reports a deterministic diagnostic.
