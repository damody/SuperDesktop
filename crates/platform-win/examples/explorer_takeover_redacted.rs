use platform_win::common::explorer_recovery::{
    ShellRecoveryOutcome, ShellShutdownOutcome, recover_explorer_shell,
    shutdown_trusted_explorer_shell,
};

fn main() -> Result<(), &'static str> {
    match std::env::args().nth(1).as_deref() {
        Some("recover") => {
            let disposition = match recover_explorer_shell()? {
                ShellRecoveryOutcome::ShownExisting { .. } => "shown_existing",
                ShellRecoveryOutcome::SpawnedVerified { .. } => "spawned_verified",
            };
            println!("explorer_recovery={disposition};identity_redacted=true");
        }
        _ => {
            let disposition = match shutdown_trusted_explorer_shell()? {
                ShellShutdownOutcome::AlreadyAbsent => "already_absent",
                ShellShutdownOutcome::ClosedGracefully { .. } => "closed_gracefully",
                ShellShutdownOutcome::Terminated { .. } => "terminated",
            };
            println!("explorer_shutdown={disposition};identity_redacted=true");
        }
    }
    Ok(())
}
