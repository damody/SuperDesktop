//! Guardian placeholder. Recovery behavior is intentionally deferred to Wave 5.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod identity;

fn main() {
    // Wave 1 establishes only a buildable binary boundary.
    let _ = (identity::APP_USER_MODEL_ID, identity::ORIGINAL_FILENAME);
}
