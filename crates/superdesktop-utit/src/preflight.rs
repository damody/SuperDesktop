use std::{path::Path, process::Command};

use serde::Deserialize;

use crate::{HostFacts, Prerequisite, PrerequisiteDisposition};

#[derive(Deserialize)]
struct Probe {
    windows_build: u32,
    architecture: String,
    interactive: bool,
    monitor_count: u32,
    explorer_running: bool,
    tools: Vec<String>,
}

pub fn observe_host(workspace: &Path) -> Result<HostFacts, String> {
    let script = workspace.join("scripts/utit-host-facts.ps1");
    if !script.is_file() {
        return Err("host-facts-script-missing".into());
    }
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .output()
        .map_err(|error| format!("host-facts-spawn:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "host-facts-failed:{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let probe: Probe = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("host-facts-json:{error}"))?;
    let revision = Command::new("git.exe")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace)
        .output()
        .map_err(|error| format!("git-revision-spawn:{error}"))?;
    if !revision.status.success() {
        return Err("git-revision-failed".into());
    }
    Ok(HostFacts {
        windows_build: probe.windows_build,
        architecture: probe.architecture,
        interactive: probe.interactive,
        monitor_count: probe.monitor_count,
        explorer_running: probe.explorer_running,
        workspace_revision: String::from_utf8_lossy(&revision.stdout).trim().into(),
        tools: probe.tools,
    })
}

pub fn evaluate_prerequisite(
    prerequisite: &Prerequisite,
    host: &HostFacts,
    workspace: &Path,
) -> PrerequisiteDisposition {
    match prerequisite {
        Prerequisite::Tool(tool)
            if host
                .tools
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(tool.trim_end_matches(".exe"))) =>
        {
            PrerequisiteDisposition::Ready
        }
        Prerequisite::Tool(tool) => {
            PrerequisiteDisposition::Blocked(format!("missing-tool:{tool}"))
        }
        Prerequisite::File(relative) if workspace.join(relative).is_file() => {
            PrerequisiteDisposition::Ready
        }
        Prerequisite::File(relative) => {
            PrerequisiteDisposition::Blocked(format!("missing-file:{relative}"))
        }
        Prerequisite::Interactive if host.interactive => PrerequisiteDisposition::Ready,
        Prerequisite::Interactive => {
            PrerequisiteDisposition::Blocked("interactive-session-required".into())
        }
        Prerequisite::MultiDisplay if host.monitor_count >= 2 => PrerequisiteDisposition::Ready,
        Prerequisite::MultiDisplay => PrerequisiteDisposition::Blocked(format!(
            "physical-mixed-dpi-requires-two-displays:observed={}",
            host.monitor_count
        )),
        Prerequisite::RebootAuthority => {
            PrerequisiteDisposition::Blocked("controlled-reboot-authority-not-admitted".into())
        }
        Prerequisite::ExternalReview => {
            PrerequisiteDisposition::Blocked("independent-review-not-attached".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> HostFacts {
        HostFacts {
            windows_build: 26200,
            architecture: "AMD64".into(),
            interactive: true,
            monitor_count: 1,
            explorer_running: true,
            workspace_revision: "abc".into(),
            tools: vec!["cargo".into(), "powershell".into()],
        }
    }

    #[test]
    fn prerequisites_are_truthful_and_never_infer_external_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(
            evaluate_prerequisite(&Prerequisite::Tool("cargo".into()), &host(), &root),
            PrerequisiteDisposition::Ready
        );
        assert!(matches!(
            evaluate_prerequisite(&Prerequisite::Tool("missing".into()), &host(), &root),
            PrerequisiteDisposition::Blocked(_)
        ));
        assert!(matches!(
            evaluate_prerequisite(&Prerequisite::MultiDisplay, &host(), &root),
            PrerequisiteDisposition::Blocked(_)
        ));
        assert!(matches!(
            evaluate_prerequisite(&Prerequisite::RebootAuthority, &host(), &root),
            PrerequisiteDisposition::Blocked(_)
        ));
        assert!(matches!(
            evaluate_prerequisite(&Prerequisite::ExternalReview, &host(), &root),
            PrerequisiteDisposition::Blocked(_)
        ));
    }
}
