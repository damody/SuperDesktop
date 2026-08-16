//! Restricted SuperDesktop recovery guardian.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

use superdesktop_guardian::{GuardianInvocation, probe_recovery_readiness, run_guardian};

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["--installer-recovery-probe"] {
        match probe_recovery_readiness() {
            Ok(()) => {
                println!("{{\"guardian_recovery_admitted\":true,\"mutations_attempted\":false}}");
                return;
            }
            Err(reason) => {
                eprintln!("guardian recovery probe rejected: {reason}");
                std::process::exit(4);
            }
        }
    }
    match GuardianInvocation::from_args(arguments) {
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
