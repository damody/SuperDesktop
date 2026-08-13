# SuperDesktop source and dependency boundary policy

## Permitted research boundary

`D:\SuperDesktop\PExplorer` may be read only to understand observable behavior, Windows API usage, message flow, and failure cases. It is an LGPL-2.1-or-later reference project, not a SuperDesktop source input.

`D:\SuperExplorer` is a separately built external product. SuperDesktop may later launch its installed executable as an external process, but MUST NOT depend on its worktree, internal crates, or `vendor/gpui-ce` path.

## Prohibited production inputs

- Copying or mechanical translation of PExplorer/ReactOS source into `crates/**`.
- A Cargo `path` dependency, patch, include, build-script input, or generated source that resolves into `D:\SuperExplorer` or `D:\SuperDesktop\PExplorer`.
- An unrecorded third-party source, revision, checksum, or license.

## Approved production dependency sources

Production dependencies must resolve only through `Cargo.lock` and the project-local `vendor/` mirror. The current GPUI-CE exception is the approved clean remote `https://github.com/damody/gpui-ce-explorer.git` at `8945e2981b9fd00ca887e042d8adb9acc241b168`; its exact Cargo source signature is `git+https://github.com/damody/gpui-ce-explorer.git?rev=8945e2981b9fd00ca887e042d8adb9acc241b168#8945e2981b9fd00ca887e042d8adb9acc241b168`. It is recorded in the inventory and lockfile.

## Enforcement

`scripts/generate-license-inventory.ps1` derives the machine-readable inventory from `cargo metadata --locked --offline` and vendor checksums. `scripts/audit-source-boundary.ps1` rejects missing inventory coverage or licenses, external local path dependencies, and PExplorer/ReactOS derivation markers in production crate sources. The negative fixtures are intentional audit inputs only and are never workspace members.
