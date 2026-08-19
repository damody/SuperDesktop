//! SuperDesktop composition-root entry point.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

use superdesktop_app::{ExecutionRequest, run_preflight, run_product, run_product_for};

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|arg| arg == "--guardian-parent-fixture")
    {
        let Some(path) = args.get(1).and_then(|value| value.to_str()) else {
            eprintln!("missing guardian terminal path");
            std::process::exit(2);
        };
        match superdesktop_app::run_guardian_parent_fixture(path) {
            Ok(pid) => println!("guardian-child-pid={pid}"),
            Err(reason) => {
                eprintln!("guardian parent fixture failed: {reason}");
                std::process::exit(3);
            }
        }
        return;
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--verification-hold-ms")
    {
        let Some(milliseconds) = args
            .get(1)
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<u64>().ok())
        else {
            eprintln!("invalid verification hold duration");
            std::process::exit(2);
        };
        if milliseconds > 60_000 {
            eprintln!("verification hold duration exceeds bound");
            std::process::exit(2);
        }
        if let Err(reason) = run_preflight(ExecutionRequest::default()) {
            eprintln!("verification preview failed: {reason}");
            std::process::exit(3);
        }
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
        return;
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--verification-capture-ms")
    {
        let Some(milliseconds) = args
            .get(1)
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<u64>().ok())
        else {
            eprintln!("invalid verification capture duration");
            std::process::exit(2);
        };
        if !(250..=60_000).contains(&milliseconds) {
            eprintln!("verification capture duration outside bounds");
            std::process::exit(2);
        }
        if let Err(reason) = run_product_for(
            ExecutionRequest::from_args(args.iter().skip(2).cloned()),
            Some(std::time::Duration::from_millis(milliseconds)),
        ) {
            eprintln!("verification capture failed: {reason}");
            std::process::exit(3);
        }
        return;
    }
    let request = ExecutionRequest::from_product_args(args);
    if let Err(reason) = run_product(request) {
        eprintln!("SuperDesktop admission failed: {reason}");
        std::process::exit(2);
    }
}
