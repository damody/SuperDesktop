# Wave 1 corrective evidence

Corrective L2 2.4 replaces the P1 findings with machine-checked controls. The contract verifier reads every manifest line and rejects malformed, escaping, missing or hash-drifted repository-relative inputs. The corrective manifest passes.

The evidence validator has 31 coverage mappings and 71 append-only records. Existing legacy records are retained; schema-complete v2 corrective records provide passing coverage for all old 26 leaves and the five 2.4 leaves. The B-W1-EXIT-001 lineage is appended with immutable stale/replacement record IDs.

The architecture scanner recursively scans UI source modules and rejects nested `pub use HWND`. Positive and negative gates pass. No unresolved P0/P1 finding remains in this corrective work package; Primary retains independent Wave 1 exit acceptance.
