//! Dedicated binary carrying a machine-verifiable test-support identity.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod identity;

fn main() {
    let _ = (identity::APP_USER_MODEL_ID, identity::ORIGINAL_FILENAME);
}
