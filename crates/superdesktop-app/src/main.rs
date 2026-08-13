//! Composition-root placeholder. It deliberately starts no UI or Shell integration.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

fn main() {
    // Wave 1 establishes only a buildable binary identity boundary.
}
