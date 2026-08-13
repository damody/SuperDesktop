# Workspace 1.1 evidence summary

Generated 2026-08-14T02:19:59+08:00 for `bootstrap-superdesktop-workspace/1.1`.

## Passed checks

- `cargo metadata --format-version 1 --no-deps` returned the nine approved workspace members and no production dependencies.
- `scripts/check-dependency-architecture.ps1` accepted the workspace graph, all Windows-only compile guards, and the UI public-type boundary.
- The `core-depends-on-gpui` fixture was rejected with `CORE_FORBIDDEN_DEPENDENCY`.
- The `ui-public-hwnd` fixture was rejected with `UI_PUBLIC_WINDOWS_OR_COM_TYPE`.
- `cargo check --workspace`, `cargo test --workspace`, and `cargo fmt --all -- --check` all returned zero on the installed Windows MSVC target.
- `openspec validate bootstrap-superdesktop-workspace --strict` and `git diff --check` returned zero.

## Windows-only disposition

Every crate root contains `#[cfg(not(windows))] compile_error!(...)`. The local Rust installation has only `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc` targets, so no non-Windows target was downloaded merely to exercise failure. The source guard is checked mechanically and supplies the deterministic refusal when a non-Windows target is selected.

## Contract

`workspace-contract-inputs.sha256` lists one input per line with the SHA-256 digest for the root manifest, all nine crate manifests/sources, and the architecture allowlist. Its own SHA-256, `C09A2FA13A5FD7D6194D28B7D009CD2C8C3633935FBC21761A02F918941700A5`, is the accepted 1.1 architecture-contract hash.

Cargo generated an empty-dependency `Cargo.lock` during verification, but it is not part of this L2 contract or accepted revision. Wave 1/1.2 owns the lockfile, pinned GPUI source, toolchain, dependency provenance, and offline-build proof; this L2 did not add a third-party dependency.
