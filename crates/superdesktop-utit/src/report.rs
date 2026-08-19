use std::{collections::BTreeSet, fs, path::Path, process::Command};

use crate::{
    RunReport, TerminalState,
    executor::{derive_counts, derive_decision},
};

pub fn hash_file(path: &Path) -> Result<String, String> {
    let output = Command::new("certutil.exe")
        .arg("-hashfile")
        .arg(path)
        .arg("SHA256")
        .output()
        .map_err(|error| format!("certutil-spawn:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "certutil-failed:{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| {
            line.len() == 64 && line.chars().all(|character| character.is_ascii_hexdigit())
        })
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "certutil-hash-missing".into())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn markdown_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}

pub fn junit(report: &RunReport) -> String {
    let failures = report.counts.failed;
    let skipped = report.counts.blocked + report.counts.skipped + report.counts.not_applicable;
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"SuperDesktop UTIT {}\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
        report.suite, report.counts.selected, failures, skipped
    );
    for case in &report.cases {
        xml.push_str(&format!(
            "  <testcase classname=\"superdesktop.utit\" name=\"{}\" time=\"{:.3}\">",
            xml_escape(&case.title),
            case.duration_ms as f64 / 1000.0
        ));
        match case.state {
            TerminalState::Failed => xml.push_str(&format!(
                "<failure message=\"{}\" />",
                xml_escape(&case.reason)
            )),
            TerminalState::Blocked | TerminalState::Skipped | TerminalState::NotApplicable => xml
                .push_str(&format!(
                    "<skipped message=\"{}\" />",
                    xml_escape(&case.reason)
                )),
            TerminalState::Passed => {}
        }
        xml.push_str("</testcase>\n");
    }
    xml.push_str("</testsuite>\n");
    xml
}

pub fn markdown(report: &RunReport) -> String {
    let mut output = format!(
        "# SuperDesktop UTIT {}\n\n- Run: `{}`\n- Decision: `{:?}`\n- Partial: `{}`\n- Host: Windows build {}, {} monitor(s)\n\n| Case | State | Duration | Reason |\n|---|---:|---:|---|\n",
        report.suite,
        markdown_escape(&report.run_id),
        report.decision,
        report.partial,
        report.host.windows_build,
        report.host.monitor_count,
    );
    for case in &report.cases {
        output.push_str(&format!(
            "| {} | `{:?}` | {} ms | {} |\n",
            markdown_escape(&case.title),
            case.state,
            case.duration_ms,
            markdown_escape(&case.reason)
        ));
    }
    output
}

pub fn write_report_bundle(run_dir: &Path, report: &RunReport) -> Result<(), String> {
    fs::create_dir_all(run_dir).map_err(|error| format!("run-dir-create:{error}"))?;
    let json = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(run_dir.join("report.json"), json).map_err(|error| error.to_string())?;
    fs::write(run_dir.join("junit.xml"), junit(report)).map_err(|error| error.to_string())?;
    fs::write(run_dir.join("summary.md"), markdown(report)).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn validate_report(path: &Path) -> Result<RunReport, Vec<String>> {
    let mut errors = Vec::new();
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return Err(vec![format!("report-read:{error}")]),
    };
    let report: RunReport = match serde_json::from_slice(&bytes) {
        Ok(report) => report,
        Err(error) => return Err(vec![format!("report-json:{error}")]),
    };
    if report.schema != crate::REPORT_SCHEMA {
        errors.push(format!(
            "schema:expected={}:actual={}",
            crate::REPORT_SCHEMA,
            report.schema
        ));
    }
    if report.finished_unix_ms < report.started_unix_ms {
        errors.push("timestamp-order".into());
    }
    let mut ids = BTreeSet::new();
    for case in &report.cases {
        if !ids.insert(&case.id) {
            errors.push(format!("duplicate-case:{}", case.id));
        }
        if case.state == TerminalState::Passed && !case.recovery_verified {
            errors.push(format!("unverified-recovery:{}", case.id));
        }
    }
    let counts = derive_counts(&report.cases);
    if counts != report.counts {
        errors.push("count-mismatch".into());
    }
    let decision = derive_decision(&counts, report.partial);
    if decision != report.decision {
        errors.push(format!(
            "decision-mismatch:expected={decision:?}:actual={:?}",
            report.decision
        ));
    }
    let Some(run_dir) = path.parent() else {
        return Err(vec!["report-parent-missing".into()]);
    };
    for record in report.cases.iter().flat_map(|case| {
        case.stdout
            .iter()
            .chain(case.stderr.iter())
            .chain(case.artifacts.iter())
    }) {
        let relative = Path::new(&record.path);
        if relative.is_absolute() || record.path.contains("..") {
            errors.push(format!("artifact-path-rejected:{}", record.path));
            continue;
        }
        let artifact = run_dir.join(relative);
        if !artifact.is_file() {
            errors.push(format!("artifact-missing:{}", record.path));
            continue;
        }
        match hash_file(&artifact) {
            Ok(hash) if hash == record.sha256 => {}
            Ok(_) => errors.push(format!("artifact-hash-drift:{}", record.path)),
            Err(error) => errors.push(format!("artifact-hash-error:{}:{error}", record.path)),
        }
    }
    if errors.is_empty() {
        Ok(report)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaseResult, HostFacts, RunCounts, RunDecision, Suite};

    fn report(title: &str, reason: &str) -> RunReport {
        RunReport {
            schema: crate::REPORT_SCHEMA.into(),
            run_id: "run-test".into(),
            suite: Suite::Smoke,
            partial: false,
            started_unix_ms: 1,
            finished_unix_ms: 2,
            workspace: "D:/workspace".into(),
            host: HostFacts {
                windows_build: 26200,
                architecture: "AMD64".into(),
                interactive: true,
                monitor_count: 1,
                explorer_running: true,
                workspace_revision: "abc".into(),
                tools: vec![],
            },
            counts: RunCounts {
                selected: 1,
                passed: 1,
                ..RunCounts::default()
            },
            decision: RunDecision::Passed,
            cases: vec![CaseResult {
                id: "case".into(),
                title: title.into(),
                state: TerminalState::Passed,
                reason: reason.into(),
                duration_ms: 5,
                exit_code: Some(0),
                timed_out: false,
                argv: vec![],
                stdout: None,
                stderr: None,
                artifacts: vec![],
                recovery_verified: true,
            }],
        }
    }

    #[test]
    fn projections_escape_unicode_xml_markdown_and_are_stable() {
        let report = report("繁體 <A&B> |", "line\nnext & more");
        let xml = junit(&report);
        assert!(xml.contains("繁體 &lt;A&amp;B&gt; |"));
        let markdown = markdown(&report);
        assert!(markdown.contains("繁體 <A&B> \\|"));
        assert!(!markdown.contains("line\nnext"));
        assert_eq!(xml, junit(&report));
        assert_eq!(markdown, super::markdown(&report));
    }

    #[test]
    fn validator_rejects_duplicate_count_decision_and_hash_drift() {
        let root = std::env::temp_dir().join(format!("utit-report-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("artifact.txt");
        fs::write(&artifact, "original").unwrap();
        let mut report = report("test", "passed");
        report.cases[0].artifacts.push(crate::ArtifactRecord {
            path: "artifact.txt".into(),
            bytes: 8,
            sha256: hash_file(&artifact).unwrap(),
        });
        report.cases.push(report.cases[0].clone());
        report.counts.selected = 1;
        report.decision = RunDecision::Failed;
        fs::write(
            root.join("report.json"),
            serde_json::to_vec(&report).unwrap(),
        )
        .unwrap();
        fs::write(&artifact, "changed").unwrap();
        let errors = validate_report(&root.join("report.json"))
            .unwrap_err()
            .join("|");
        assert!(errors.contains("duplicate-case"));
        assert!(errors.contains("count-mismatch"));
        assert!(errors.contains("decision-mismatch"));
        assert!(errors.contains("artifact-hash-drift"));
    }
}
