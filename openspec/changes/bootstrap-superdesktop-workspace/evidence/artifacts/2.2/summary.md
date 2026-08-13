# Source and license boundary evidence

L2 2.2 defines a machine-auditable source boundary without changing production code, the root dependency contract, SuperExplorer, or PExplorer.

`source-boundary-policy.md` allows PExplorer only as read-only behavior/API/Win32-message research and treats SuperExplorer solely as an external process. It prohibits local paths to either repository, PExplorer/ReactOS derived source, and unrecorded third-party inputs.

`generate-license-inventory.ps1` obtains Cargo's locked offline package graph and writes 399 package records to `compliance/third-party-license-inventory.json`. Every record has a source and license; third-party packages additionally retain the SHA-256 of their vendored `.cargo-checksum.json`. The auditor independently regenerates Cargo metadata and checks each record and checksum.

The normal audit passes. The `superexplorer-path-dependency` fixture is rejected as `SUPEREXPLORER_PATH_DEPENDENCY`, and the `pexplorer-derived-source` fixture is rejected as `PEXPLORER_DERIVED_SOURCE`. The reviewer disposition is recorded in `compliance/reviewer-disposition.md` as passed pre-review pending Primary's Wave 1 exit acceptance.
