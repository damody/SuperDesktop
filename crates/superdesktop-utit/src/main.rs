use std::{
    env, fs,
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use superdesktop_utit::{
    CommandLine, ExecutionOptions, GuiMeasurement, RunDecision, UtitCommand, catalog, execute_run,
    gui_parity_manifest, observe_host, parse_args, select_cases, validate_catalog,
    validate_gui_measurement, validate_report, write_report_bundle,
};

fn fail(message: impl AsRef<str>) -> ExitCode {
    eprintln!("UTIT error: {}", message.as_ref());
    ExitCode::from(2)
}

fn run_id() -> String {
    format!(
        "run-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

fn main() -> ExitCode {
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => return fail(error.to_string()),
    };
    let CommandLine { workspace, command } = match parse_args(env::args().skip(1), current_dir) {
        Ok(command) => command,
        Err(error) => return fail(error),
    };
    let workspace = match workspace.canonicalize() {
        Ok(path) if path.join("Cargo.toml").is_file() && path.join("scripts").is_dir() => path,
        Ok(_) => return fail("workspace lacks Cargo.toml or scripts"),
        Err(error) => return fail(format!("workspace:{error}")),
    };
    let cases = catalog();
    if let Err(errors) = validate_catalog(&cases, &workspace) {
        return fail(errors.join("\n"));
    }
    match command {
        UtitCommand::List { json } => {
            if json {
                match serde_json::to_string_pretty(&cases) {
                    Ok(value) => println!("{value}"),
                    Err(error) => return fail(error.to_string()),
                }
            } else {
                for case in cases {
                    println!(
                        "{}\t{:?}\t{}s\t{}\t{:?}",
                        case.id,
                        case.tier,
                        case.timeout_seconds,
                        case.tags.join(","),
                        case.recovery
                    );
                }
            }
            ExitCode::SUCCESS
        }
        UtitCommand::ValidateReport { path } => match validate_report(&path) {
            Ok(report) => {
                println!("validated {}: {:?}", report.run_id, report.decision);
                ExitCode::SUCCESS
            }
            Err(errors) => fail(errors.join("\n")),
        },
        UtitCommand::ValidateGuiMeasurement { path } => {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) => return fail(format!("measurement-read:{error}")),
            };
            let measurement: GuiMeasurement = match serde_json::from_slice(&bytes) {
                Ok(measurement) => measurement,
                Err(error) => return fail(format!("measurement-json:{error}")),
            };
            let manifest = gui_parity_manifest();
            let Some(spec) = manifest
                .iter()
                .find(|spec| spec.id == measurement.surface_id)
            else {
                return fail(format!("unknown GUI surface: {}", measurement.surface_id));
            };
            match validate_gui_measurement(spec, &measurement) {
                Ok(()) => {
                    println!("validated GUI measurement: {}", measurement.surface_id);
                    ExitCode::SUCCESS
                }
                Err(errors) => fail(
                    errors
                        .into_iter()
                        .map(|error| {
                            format!(
                                "{}:{} expected={} actual={}",
                                error.surface_id, error.rule, error.expected, error.actual
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            }
        }
        UtitCommand::Run {
            suite,
            cases: filters,
            tags,
            output,
            dry_run,
            fail_fast,
            replace_run,
        } => {
            let selected = match select_cases(&cases, suite, &filters, &tags) {
                Ok(selected) => selected,
                Err(error) => return fail(error),
            };
            if dry_run {
                println!(
                    "suite={suite} partial={} selected={}",
                    selected.partial,
                    selected.cases.len()
                );
                for case in selected.cases {
                    println!("{}\t{:?}\t{}", case.id, case.program, case.timeout_seconds);
                }
                return ExitCode::SUCCESS;
            }
            let host = match observe_host(&workspace) {
                Ok(host) => host,
                Err(error) => return fail(error),
            };
            let run_dir = output.unwrap_or_else(|| PathBuf::from("utit-results").join(run_id()));
            let run_dir = if run_dir.is_absolute() {
                run_dir
            } else {
                workspace.join(run_dir)
            };
            if run_dir.exists() {
                if !replace_run {
                    return fail(format!("run directory exists: {}", run_dir.display()));
                }
                if !run_dir.join("report.json").is_file() {
                    return fail("replace-run requires an existing UTIT report directory");
                }
                if let Err(error) = fs::remove_file(run_dir.join("report.json")) {
                    return fail(format!("replace-report-remove:{error}"));
                }
            }
            if let Err(error) = fs::create_dir_all(&run_dir) {
                return fail(format!("run-dir:{error}"));
            }
            let report = execute_run(
                selected,
                suite,
                &workspace,
                &run_dir,
                host,
                ExecutionOptions { fail_fast },
            );
            if let Err(error) = write_report_bundle(&run_dir, &report) {
                return fail(error);
            }
            println!("UTIT {:?}: {}", report.decision, run_dir.display());
            match report.decision {
                RunDecision::Passed | RunDecision::Partial => ExitCode::SUCCESS,
                RunDecision::Failed | RunDecision::Incomplete => ExitCode::from(1),
            }
        }
    }
}
