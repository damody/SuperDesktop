use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    ArtifactRecord, CaseResult, HostFacts, PrerequisiteDisposition, ProgramSpec, Recovery,
    ResolvedCommand, RunCounts, RunDecision, RunReport, SelectedCases, Suite, TerminalState,
    TestCase, evaluate_prerequisite, hash_file, observe_host,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionOptions {
    pub fail_fast: bool,
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn windows_argv_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).into()
    }
}

fn replace_tokens(value: &str, workspace: &Path, case_dir: &Path) -> String {
    value
        .replace("{workspace}", &windows_argv_path(workspace))
        .replace("{out}", &windows_argv_path(case_dir))
}

pub fn resolve_command(
    case: &TestCase,
    workspace: &Path,
    case_dir: &Path,
) -> Result<ResolvedCommand, String> {
    let (program, args) = match &case.program {
        ProgramSpec::Cargo { args } => (PathBuf::from("cargo.exe"), args.clone()),
        ProgramSpec::OpenSpec { args } => (PathBuf::from("openspec.cmd"), args.clone()),
        ProgramSpec::PowerShell { script, args } => {
            if Path::new(script).is_absolute() || script.contains("..") {
                return Err("script-path-rejected".into());
            }
            let scripts = workspace
                .join("scripts")
                .canonicalize()
                .map_err(|error| error.to_string())?;
            let script = scripts
                .join(script)
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !script.starts_with(&scripts)
                || script.extension().and_then(|value| value.to_str()) != Some("ps1")
            {
                return Err("script-path-rejected".into());
            }
            let mut resolved = vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                windows_argv_path(&script),
            ];
            resolved.extend(args.clone());
            (PathBuf::from("powershell.exe"), resolved)
        }
        ProgramSpec::External { reason } => return Err(format!("external-case:{reason}")),
    };
    let args = args
        .into_iter()
        .map(|argument| replace_tokens(&argument, workspace, case_dir))
        .collect();
    Ok(ResolvedCommand { program, args })
}

fn record_file(path: &Path, run_dir: &Path) -> Result<ArtifactRecord, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("artifact-metadata:{error}"))?;
    let relative = path
        .strip_prefix(run_dir)
        .map_err(|_| "artifact-outside-run")?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(ArtifactRecord {
        path: relative,
        bytes: metadata.len(),
        sha256: hash_file(path)?,
    })
}

fn blocked_result(case: &TestCase, reason: String) -> CaseResult {
    CaseResult {
        id: case.id.clone(),
        title: case.title.clone(),
        state: TerminalState::Blocked,
        reason,
        duration_ms: 0,
        exit_code: None,
        timed_out: false,
        argv: vec![],
        stdout: None,
        stderr: None,
        artifacts: vec![],
        recovery_verified: !case.explorer_free,
    }
}

fn verify_recovery(
    case: &TestCase,
    case_dir: &Path,
    workspace: &Path,
    host: &HostFacts,
) -> Result<bool, String> {
    let Recovery::ExplorerWatchdog { report } = &case.recovery else {
        return Ok(true);
    };
    let report_path = case_dir.join(report);
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(&report_path).map_err(|error| format!("recovery-report-read:{error}"))?,
    )
    .map_err(|error| format!("recovery-report-json:{error}"))?;
    fn contains_absent(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                map.get("explorer_absent_during_capture")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                    || map.values().any(contains_absent)
            }
            serde_json::Value::Array(values) => values.iter().any(contains_absent),
            _ => false,
        }
    }
    if !contains_absent(&value) {
        return Err("recovery-report-lacks-explorer-absence".into());
    }
    if host.explorer_running {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if observe_host(workspace).is_ok_and(|facts| facts.explorer_running) {
                return Ok(true);
            }
            thread::sleep(Duration::from_millis(200));
        }
        return Err("explorer-not-restored".into());
    }
    Ok(true)
}

pub fn execute_case(
    case: &TestCase,
    workspace: &Path,
    run_dir: &Path,
    host: &HostFacts,
) -> CaseResult {
    for prerequisite in &case.prerequisites {
        match evaluate_prerequisite(prerequisite, host, workspace) {
            PrerequisiteDisposition::Ready => {}
            PrerequisiteDisposition::Blocked(reason) => return blocked_result(case, reason),
            PrerequisiteDisposition::NotApplicable(reason) => {
                let mut result = blocked_result(case, reason);
                result.state = TerminalState::NotApplicable;
                return result;
            }
        }
    }
    if let ProgramSpec::External { reason } = &case.program {
        return blocked_result(case, reason.clone());
    }
    let case_dir = run_dir.join("cases").join(&case.id);
    if let Err(error) = fs::create_dir_all(&case_dir) {
        return blocked_result(case, format!("case-dir:{error}"));
    }
    let command = match resolve_command(case, workspace, &case_dir) {
        Ok(command) => command,
        Err(error) => return blocked_result(case, error),
    };
    let stdout_path = case_dir.join("stdout.log");
    let stderr_path = case_dir.join("stderr.log");
    let stdout = match File::create(&stdout_path) {
        Ok(file) => file,
        Err(error) => return blocked_result(case, format!("stdout-create:{error}")),
    };
    let stderr = match File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => return blocked_result(case, format!("stderr-create:{error}")),
    };
    let argv = std::iter::once(command.program.to_string_lossy().into_owned())
        .chain(command.args.iter().cloned())
        .collect::<Vec<_>>();
    let started = Instant::now();
    let mut child = match Command::new(&command.program)
        .args(&command.args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return blocked_result(case, format!("spawn:{error}")),
    };
    let deadline = started + Duration::from_secs(case.timeout_seconds);
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break (None, true);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break (None, false);
            }
        }
    };
    let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let mut reasons = Vec::new();
    if timed_out {
        reasons.push("timeout".into());
    } else if !status.is_some_and(|status| status.success()) {
        reasons.push("nonzero-exit".into());
    }
    let mut artifacts = Vec::new();
    for expected in &case.artifacts {
        let path = case_dir.join(&expected.path);
        if path.is_file() {
            match record_file(&path, run_dir) {
                Ok(record) => artifacts.push(record),
                Err(error) => reasons.push(error),
            }
        } else if expected.required {
            reasons.push(format!("missing-artifact:{}", expected.path));
        }
    }
    let recovery_verified = match verify_recovery(case, &case_dir, workspace, host) {
        Ok(value) => value,
        Err(error) => {
            reasons.push(error);
            false
        }
    };
    let stdout_record = record_file(&stdout_path, run_dir).ok();
    let stderr_record = record_file(&stderr_path, run_dir).ok();
    CaseResult {
        id: case.id.clone(),
        title: case.title.clone(),
        state: if reasons.is_empty() {
            TerminalState::Passed
        } else {
            TerminalState::Failed
        },
        reason: if reasons.is_empty() {
            "passed".into()
        } else {
            reasons.join(";")
        },
        duration_ms,
        exit_code: status.and_then(|status| status.code()),
        timed_out,
        argv,
        stdout: stdout_record,
        stderr: stderr_record,
        artifacts,
        recovery_verified,
    }
}

pub fn derive_counts(cases: &[CaseResult]) -> RunCounts {
    let mut counts = RunCounts {
        selected: cases.len(),
        ..RunCounts::default()
    };
    for case in cases {
        match case.state {
            TerminalState::Passed => counts.passed += 1,
            TerminalState::Failed => counts.failed += 1,
            TerminalState::Blocked => counts.blocked += 1,
            TerminalState::Skipped => counts.skipped += 1,
            TerminalState::NotApplicable => counts.not_applicable += 1,
        }
    }
    counts
}

pub fn derive_decision(counts: &RunCounts, partial: bool) -> RunDecision {
    if counts.failed != 0 {
        RunDecision::Failed
    } else if counts.blocked != 0 || counts.skipped != 0 {
        RunDecision::Incomplete
    } else if partial {
        RunDecision::Partial
    } else {
        RunDecision::Passed
    }
}

pub fn execute_run(
    selected: SelectedCases,
    suite: Suite,
    workspace: &Path,
    run_dir: &Path,
    host: HostFacts,
    options: ExecutionOptions,
) -> RunReport {
    let started_unix_ms = unix_ms();
    let run_id = run_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("run")
        .into();
    let mut results = Vec::new();
    let mut failed = false;
    for case in selected.cases {
        if failed && options.fail_fast {
            results.push(CaseResult {
                id: case.id,
                title: case.title,
                state: TerminalState::Skipped,
                reason: "fail-fast".into(),
                duration_ms: 0,
                exit_code: None,
                timed_out: false,
                argv: vec![],
                stdout: None,
                stderr: None,
                artifacts: vec![],
                recovery_verified: !case.explorer_free,
            });
            continue;
        }
        let result = execute_case(&case, workspace, run_dir, &host);
        failed |= result.state == TerminalState::Failed;
        results.push(result);
    }
    let counts = derive_counts(&results);
    let decision = derive_decision(&counts, selected.partial);
    RunReport {
        schema: crate::REPORT_SCHEMA.into(),
        run_id,
        suite,
        partial: selected.partial,
        started_unix_ms,
        finished_unix_ms: unix_ms(),
        workspace: workspace.to_string_lossy().into_owned(),
        host,
        counts,
        decision,
        cases: results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExpectedArtifact, Prerequisite, ProgramSpec, Recovery, TestCase, Tier};

    fn fixture(mode: &str, timeout_seconds: u64) -> TestCase {
        TestCase {
            id: format!("fixture-{mode}"),
            title: mode.into(),
            tier: Tier::Smoke,
            tags: vec!["fixture".into()],
            timeout_seconds,
            mandatory: true,
            explorer_free: false,
            program: ProgramSpec::PowerShell {
                script: "utit-fixture-case.ps1".into(),
                args: vec![
                    "-Mode".into(),
                    mode.into(),
                    "-Artifact".into(),
                    "{out}/result.json".into(),
                ],
            },
            prerequisites: vec![Prerequisite::Tool("powershell.exe".into())],
            recovery: Recovery::None,
            artifacts: vec![ExpectedArtifact {
                path: "result.json".into(),
                required: true,
            }],
        }
    }

    fn host() -> HostFacts {
        HostFacts {
            windows_build: 26200,
            architecture: "AMD64".into(),
            interactive: true,
            monitor_count: 1,
            explorer_running: true,
            workspace_revision: "test".into(),
            tools: vec!["powershell".into()],
        }
    }

    fn temp_run(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("superdesktop-utit-{label}-{}", unix_ms()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn pass_failure_missing_artifact_and_timeout_are_distinct() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let run = temp_run("executor");
        let passed = execute_case(&fixture("pass", 5), &workspace, &run, &host());
        assert_eq!(passed.state, TerminalState::Passed);
        assert_eq!(passed.artifacts.len(), 1);
        let failed = execute_case(&fixture("fail", 5), &workspace, &run, &host());
        assert_eq!(failed.state, TerminalState::Failed);
        assert_eq!(failed.exit_code, Some(7));
        assert!(failed.reason.contains("missing-artifact"));
        let malformed = execute_case(&fixture("malformed", 5), &workspace, &run, &host());
        assert_eq!(malformed.state, TerminalState::Failed);
        assert!(malformed.reason.contains("missing-artifact"));
        let timeout = execute_case(&fixture("timeout", 1), &workspace, &run, &host());
        assert_eq!(timeout.state, TerminalState::Failed);
        assert!(timeout.timed_out);
    }

    #[test]
    fn decision_never_hides_failure_blocked_skip_or_filtering() {
        let mut result = blocked_result(&fixture("pass", 5), "blocked".into());
        assert_eq!(
            derive_decision(&derive_counts(&[result.clone()]), false),
            RunDecision::Incomplete
        );
        result.state = TerminalState::Failed;
        assert_eq!(
            derive_decision(&derive_counts(&[result.clone()]), true),
            RunDecision::Failed
        );
        result.state = TerminalState::Passed;
        assert_eq!(
            derive_decision(&derive_counts(&[result.clone()]), true),
            RunDecision::Partial
        );
        assert_eq!(
            derive_decision(&derive_counts(&[result]), false),
            RunDecision::Passed
        );
    }

    #[test]
    fn powershell_argv_never_receives_extended_length_prefixes() {
        assert_eq!(
            windows_argv_path(Path::new(r"\\?\D:\workspace")),
            r"D:\workspace"
        );
        assert_eq!(
            windows_argv_path(Path::new(r"\\?\UNC\server\share\workspace")),
            r"\\server\share\workspace"
        );
    }

    #[test]
    fn explorer_free_case_rejects_a_report_without_absence_and_recovery_proof() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let run = temp_run("recovery");
        let mut case = fixture("pass", 5);
        case.explorer_free = true;
        case.recovery = Recovery::ExplorerWatchdog {
            report: "result.json".into(),
        };
        let result = execute_case(&case, &workspace, &run, &host());
        assert_eq!(result.state, TerminalState::Failed);
        assert!(!result.recovery_verified);
        assert!(
            result
                .reason
                .contains("recovery-report-lacks-explorer-absence")
        );
    }
}
