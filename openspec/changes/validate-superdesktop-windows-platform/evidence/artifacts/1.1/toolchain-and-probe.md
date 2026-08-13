# Read-only profile/admission probe contract

- Rust toolchain: `1.97.1`; `rust-toolchain.toml` SHA-256 is recorded by `frozen-profile-contract.json`.
- GPUI-CE: `gpui` 0.2.2 is locked to revision `8945e2981b9fd00ca887e042d8adb9acc241b168`.
- Source: `crates/platform-win/examples/capability_profile.rs`; Cargo.lock, source, and the persisted binary hashes are authoritative fields of `frozen-profile-contract.json`.
- Reproducible build: `cargo build -p platform-win --example capability_profile --locked --offline`; the resulting exact executable is copied to `evidence/artifacts/1.1/bin/capability_profile.exe` before trace capture.
- Admission reads only SM_CLEANBOOT, current token session/user/logon identities, OS shell HWND owner PID, the verified `%WINDIR%\\explorer.exe` image, that owner's token identities, WTS state, and WinSta0. It rejects typed API failures and identity/session mismatches with exit 2.
- Aligned owned token buffers validate returned sizes, raw record offsets, SID structure/extents, and all queried handles/buffers are released. Unit fixtures cover Safe Mode, wrong session, service/system identity, same-session foreign user/logon identity, noninteractive station, and a same-named non-system Explorer path.
- No window, AppBar, hook, Explorer, or work-area mutation API is invoked.
