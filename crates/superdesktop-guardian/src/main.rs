//! Restricted SuperDesktop recovery guardian.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

use superdesktop_guardian::{GuardianInvocation, run_guardian};

fn main() {
    match GuardianInvocation::from_args(std::env::args().skip(1)) {
        Ok(invocation) => {
            if let Err(reason) = run_guardian(invocation) {
                eprintln!("guardian rejected: {reason}");
                std::process::exit(3);
            }
        }
        Err(reason) => {
            eprintln!("guardian invocation rejected: {reason}");
            std::process::exit(2);
        }
    }
}
