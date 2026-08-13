# Archived bootstrap contract verification

`scripts/verify-archived-bootstrap-contract.ps1 -WorkspaceRoot .` exited 0 at workspace revision `13b850493d329c04c4a8d5d7ab378796ce3efb48`.

It verified archive revision `9f115980af3804829fc156029ae3b22382c7a146`, archive-tree immutability, the accepted aggregate handoff hash `9C7B643D880EF4C7135F08D32222075D43428CBF1A50A53CBBFBA7405ED2622E`, and every input of the workspace, dependency, and source-boundary manifests. Its only path adaptation maps the archived active-change prefix to the immutable archive root.

Additional read-only checks passed:

- `cargo check -p platform-win --locked --offline`
- `scripts/check-dependency-architecture.ps1 -WorkspaceRoot .`

The verified inputs pin Windows 0.62.2 with all required features, Cargo.lock/vendor provenance, global `unsafe_code = "deny"`, and the sole `platform-win` crate-local `unsafe_code = "allow"` exception. Every Win32 FFI block in the capability-profile example is locally documented with a bounded SAFETY invariant.
