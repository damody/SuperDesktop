# Toolchain and dependency source evidence

This L2 pins Rust `1.97.1`, target `x86_64-pc-windows-msvc`, the project-local GPUI-CE remote, and the full dependency closure.

## Source and lock disposition

- `Cargo.toml` pins `https://github.com/damody/gpui-ce-explorer.git` at commit `8945e2981b9fd00ca887e042d8adb9acc241b168`; the lockfile records the same immutable source identifier.
- `gpui-remote-commit.txt` captures the resolved remote commit. `dependency-provenance.json` records source URL, revision, Apache-2.0 license, and the prohibition on `D:\SuperExplorer\vendor\gpui-ce`.
- `cargo vendor --locked vendor` fetched the clean remote and created the 390-package project-local vendor mirror. `.cargo/config.toml` replaces both crates.io and the pinned GPUI git source with that mirror.
- `dependency-inputs.sha256` hashes `Cargo.toml`, `Cargo.lock`, the toolchain/configuration/assertion inputs, and all 390 vendored `.cargo-checksum.json` records. The Primary integrator corrected the generated relative path from `cargo/config.toml` to `.cargo/config.toml` before acceptance and revalidated every input hash.

## Build disposition

- The online source-resolution step (`cargo generate-lockfile`) and vendor creation completed from the approved remote before project-local source replacement was activated.
- `online-locked-check.txt` records a successful locked workspace check after resolution.
- `offline-isolated-locked-check.txt` records a clean-source compilation with `CARGO_HOME=build/isolated-cargo-home`, `CARGO_TARGET_DIR=build/isolated-target`, `CARGO_NET_OFFLINE=true`, and `--locked --offline`; it never reads the user Cargo home registry or git cache.
- `profile-assertion.txt` verifies explicit unwind in dev/release and the absence of an ignored/abort test panic override, following adjustment `B-W1-1.2-002`.

## Scope

No Shell mode, UI process, Explorer mutation, or SuperExplorer/PExplorer source change was invoked. The GPUI crate is imported only as a dependency pin in the two designated future UI crates; no product UI behavior is implemented here.
