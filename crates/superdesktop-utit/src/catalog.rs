use std::{collections::BTreeSet, path::Path};

use crate::{
    ExpectedArtifact, Prerequisite, ProgramSpec, Recovery, SelectedCases, Suite, TestCase, Tier,
};

fn artifact(path: &str) -> ExpectedArtifact {
    ExpectedArtifact {
        path: path.into(),
        required: true,
    }
}

fn cargo_case(id: &str, title: &str, args: &[&str], timeout_seconds: u64) -> TestCase {
    TestCase {
        id: id.into(),
        title: title.into(),
        tier: Tier::Smoke,
        tags: vec!["automated".into()],
        timeout_seconds,
        mandatory: true,
        explorer_free: false,
        program: ProgramSpec::Cargo {
            args: args.iter().map(ToString::to_string).collect(),
        },
        prerequisites: vec![Prerequisite::Tool("cargo".into())],
        recovery: Recovery::None,
        artifacts: vec![],
    }
}

fn headful_case(
    id: &str,
    title: &str,
    script: &str,
    args: &[&str],
    artifacts: &[&str],
    explorer_free: bool,
    timeout_seconds: u64,
) -> TestCase {
    TestCase {
        id: id.into(),
        title: title.into(),
        tier: Tier::ShellParity,
        tags: vec!["headful".into(), "gui".into()],
        timeout_seconds,
        mandatory: true,
        explorer_free,
        program: ProgramSpec::PowerShell {
            script: script.into(),
            args: args.iter().map(ToString::to_string).collect(),
        },
        prerequisites: vec![
            Prerequisite::Tool("powershell.exe".into()),
            Prerequisite::Interactive,
            Prerequisite::File("target/release/superdesktop-app.exe".into()),
        ],
        recovery: if explorer_free {
            Recovery::ExplorerWatchdog {
                report: artifacts.first().copied().unwrap_or("report.json").into(),
            }
        } else {
            Recovery::None
        },
        artifacts: artifacts.iter().map(|path| artifact(path)).collect(),
    }
}

pub fn catalog() -> Vec<TestCase> {
    let mut cases = vec![
        cargo_case(
            "unit-utit",
            "UTIT unit and fixture tests",
            &["test", "-p", "superdesktop-utit", "--locked", "--offline"],
            180,
        ),
        cargo_case(
            "unit-taskbar-ui",
            "Taskbar model and GUI contracts",
            &["test", "-p", "taskbar-ui", "--locked", "--offline"],
            180,
        ),
        cargo_case(
            "unit-superdesktop-app",
            "SuperDesktop production composition contracts",
            &["test", "-p", "superdesktop-app", "--locked", "--offline"],
            180,
        ),
        cargo_case(
            "source-explorer-policy",
            "Normal product paths are independent from Windows Explorer",
            &[
                "test",
                "-p",
                "superdesktop-utit",
                "--locked",
                "--offline",
                "explorer_policy::tests::production_gui_and_provider_paths_are_explorer_free",
                "--",
                "--exact",
            ],
            180,
        ),
        TestCase {
            id: "openspec-strict".into(),
            title: "UTIT strict OpenSpec validation".into(),
            tier: Tier::Smoke,
            tags: vec!["automated".into(), "openspec".into()],
            timeout_seconds: 60,
            mandatory: true,
            explorer_free: false,
            program: ProgramSpec::OpenSpec {
                args: vec![
                    "validate".into(),
                    "add-superdesktop-utit-runner".into(),
                    "--strict".into(),
                    "--json".into(),
                ],
            },
            prerequisites: vec![Prerequisite::Tool("openspec".into())],
            recovery: Recovery::None,
            artifacts: vec![],
        },
        cargo_case(
            "workspace-tests",
            "Locked offline workspace tests",
            &["test", "--workspace", "--locked", "--offline"],
            600,
        ),
        cargo_case(
            "workspace-clippy",
            "Workspace Clippy warnings as errors",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--offline",
                "--",
                "-D",
                "warnings",
            ],
            600,
        ),
        cargo_case(
            "workspace-release",
            "Product workspace release build (running UTIT excluded)",
            &[
                "build",
                "--workspace",
                "--release",
                "--exclude",
                "superdesktop-utit",
                "--locked",
                "--offline",
            ],
            900,
        ),
        headful_case(
            "gui-taskbar-live",
            "Live taskbar rows and labels",
            "capture-taskbar-live-production.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-OutputPath",
                "{out}/taskbar/report.json",
                "-ScreenshotPath",
                "{out}/taskbar/taskbar.png",
            ],
            &["taskbar/report.json", "taskbar/taskbar.png"],
            false,
            45,
        ),
        headful_case(
            "gui-taskbar-window-actions",
            "Taskbar application left/right pointer parity",
            "capture-taskbar-window-actions.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/window-actions",
            ],
            &["window-actions/report.json"],
            false,
            45,
        ),
        headful_case(
            "gui-start",
            "Owned Start home and all apps",
            "capture-start-production.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-OutputPath",
                "{out}/start/report.json",
                "-HomeScreenshotPath",
                "{out}/start/home.png",
                "-AllAppsScreenshotPath",
                "{out}/start/all-apps.png",
                "-PowerScreenshotPath",
                "{out}/start/power.png",
                "-Locale",
                "zh-TW",
                "-SuppressExplorer",
            ],
            &["start/report.json", "start/home.png", "start/all-apps.png"],
            true,
            60,
        ),
        headful_case(
            "gui-notification-overflow",
            "Explorer-free visible and hidden notification icon pointer parity",
            "capture-notifyicon-compatibility.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-OutputPath",
                "{out}/notifyicon/report.json",
                "-ScreenshotPath",
                "{out}/notifyicon/overflow.png",
            ],
            &["notifyicon/report.json", "notifyicon/overflow.png"],
            true,
            75,
        ),
        headful_case(
            "gui-desktop-marquee",
            "Desktop marquee selection",
            "capture-desktop-marquee-production.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-OutputPath",
                "{out}/desktop/report.json",
                "-ScreenshotPath",
                "{out}/desktop/marquee.png",
            ],
            &["desktop/report.json", "desktop/marquee.png"],
            false,
            45,
        ),
        headful_case(
            "gui-show-desktop",
            "Explorer-free Show desktop cycles",
            "capture-show-desktop-corner.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/show-desktop",
                "-Theme",
                "light",
                "-Rows",
                "1",
                "-SuppressExplorer",
                "-ExerciseCycle",
            ],
            &["show-desktop/light-row1-report.json"],
            true,
            60,
        ),
        headful_case(
            "gui-notification-center",
            "Explorer-free notification center",
            "capture-notification-center.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/notifications",
                "-Theme",
                "light",
                "-ExerciseActions",
            ],
            &["notifications/light-report.json"],
            true,
            75,
        ),
        headful_case(
            "gui-system-status",
            "Explorer-free input and volume left/right pointer parity",
            "capture-system-status-production.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/system-status",
                "-SuppressExplorer",
                "-SkipProfileSwitch",
            ],
            &["system-status/headful-report.json"],
            true,
            75,
        ),
        headful_case(
            "gui-taskbar-resize",
            "Explorer-free taskbar resize and lock",
            "capture-taskbar-resize-lock.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/resize",
                "-SuppressExplorer",
            ],
            &["resize/headful-report.json"],
            true,
            60,
        ),
        headful_case(
            "gui-context-popup-topmost-runtime",
            "Owned taskbar context topmost and AppBar fallback survival",
            "capture-taskbar-resize-lock.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/topmost-runtime",
                "-ForceAppBarUnavailable",
            ],
            &["topmost-runtime/headful-report.json"],
            false,
            60,
        ),
        headful_case(
            "gui-system-context-topmost",
            "Owned input and volume context topmost",
            "capture-system-status-production.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/system-context-topmost",
                "-SkipProfileSwitch",
                "-SkipStartFocusVerification",
            ],
            &["system-context-topmost/headful-report.json"],
            false,
            75,
        ),
        headful_case(
            "gui-taskbar-auto-hide",
            "Explorer-free taskbar auto-hide",
            "capture-taskbar-auto-hide.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/auto-hide",
                "-Rows",
                "2",
                "-SuppressExplorer",
            ],
            &["auto-hide/headful-report.json"],
            true,
            60,
        ),
        headful_case(
            "gui-taskbar-hover-preview",
            "Explorer-free taskbar hover previews",
            "capture-taskbar-hover-preview.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/hover-preview",
            ],
            &[
                "hover-preview/headful-report.json",
                "hover-preview/hover-preview.png",
            ],
            true,
            75,
        ),
        headful_case(
            "gui-win-shift-s-snipping",
            "Owned-shell native Win+Shift+S with bounded inbox Explorer broker",
            "capture-win-shift-s-snipping.ps1",
            &[
                "-Workspace",
                "{workspace}",
                "-EvidenceDirectory",
                "{out}/screen-snip",
            ],
            &[
                "screen-snip/headful-report.json",
                "screen-snip/screen-snip.log",
            ],
            false,
            75,
        ),
    ];
    for case in &mut cases {
        if matches!(
            case.id.as_str(),
            "gui-taskbar-live"
                | "gui-taskbar-window-actions"
                | "gui-start"
                | "gui-notification-overflow"
                | "gui-notification-center"
                | "gui-system-status"
                | "gui-taskbar-resize"
                | "gui-taskbar-auto-hide"
                | "gui-taskbar-hover-preview"
                | "gui-win-shift-s-snipping"
                | "unit-taskbar-ui"
        ) {
            case.tags.push("gui-parity".into());
        }
        if matches!(
            case.id.as_str(),
            "gui-taskbar-window-actions" | "gui-notification-overflow" | "gui-system-status"
        ) {
            case.tags.push("pointer".into());
        }
    }
    cases.extend([
        TestCase {
            id: "physical-mixed-dpi".into(),
            title: "Physical mixed-DPI topology".into(),
            tier: Tier::Full,
            tags: vec!["hardware".into(), "external".into()],
            timeout_seconds: 300,
            mandatory: true,
            explorer_free: false,
            program: ProgramSpec::External {
                reason: "requires at least two physical displays with distinct DPI".into(),
            },
            prerequisites: vec![Prerequisite::MultiDisplay],
            recovery: Recovery::None,
            artifacts: vec![],
        },
        TestCase {
            id: "reboot-installer-rollback".into(),
            title: "Installer mutation, reboot, and exact rollback".into(),
            tier: Tier::Full,
            tags: vec!["reboot".into(), "installer".into()],
            timeout_seconds: 1800,
            mandatory: true,
            explorer_free: true,
            program: ProgramSpec::External {
                reason: "requires privileged mutation and a controlled reboot continuation".into(),
            },
            prerequisites: vec![Prerequisite::RebootAuthority],
            recovery: Recovery::ExplorerWatchdog {
                report: "installer-recovery.json".into(),
            },
            artifacts: vec![],
        },
        TestCase {
            id: "independent-release-review".into(),
            title: "Independent release review".into(),
            tier: Tier::Full,
            tags: vec!["external".into(), "review".into()],
            timeout_seconds: 300,
            mandatory: true,
            explorer_free: false,
            program: ProgramSpec::External {
                reason: "requires an independent reviewer".into(),
            },
            prerequisites: vec![Prerequisite::ExternalReview],
            recovery: Recovery::None,
            artifacts: vec![],
        },
    ]);
    cases
}

pub fn validate_catalog(cases: &[TestCase], workspace: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    let scripts = workspace.join("scripts");
    for case in cases {
        if case.id.is_empty()
            || !case.id.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
        {
            errors.push(format!("invalid-case-id:{}", case.id));
        }
        if !ids.insert(case.id.clone()) {
            errors.push(format!("duplicate-case-id:{}", case.id));
        }
        if case.timeout_seconds == 0 {
            errors.push(format!("zero-timeout:{}", case.id));
        }
        if case.explorer_free && !matches!(case.recovery, Recovery::ExplorerWatchdog { .. }) {
            errors.push(format!("missing-explorer-watchdog:{}", case.id));
        }
        match &case.program {
            ProgramSpec::PowerShell { script, args } => {
                let path = scripts.join(script);
                if Path::new(script).is_absolute()
                    || script.contains("..")
                    || !path.starts_with(&scripts)
                    || !path.is_file()
                {
                    errors.push(format!("invalid-script:{}:{script}", case.id));
                }
                if args.iter().any(|arg| arg.contains([';', '|', '\n', '\r'])) {
                    errors.push(format!("shell-token-rejected:{}", case.id));
                }
            }
            ProgramSpec::Cargo { args } | ProgramSpec::OpenSpec { args } => {
                if args.iter().any(|arg| arg.contains([';', '|', '\n', '\r'])) {
                    errors.push(format!("shell-token-rejected:{}", case.id));
                }
            }
            ProgramSpec::External { .. } => {}
        }
    }
    if let Err(manifest_errors) =
        crate::validate_gui_parity_manifest(&crate::gui_parity_manifest(), cases)
    {
        errors.extend(
            manifest_errors
                .into_iter()
                .map(|error| format!("gui-manifest:{error}")),
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn select_cases(
    cases: &[TestCase],
    suite: Suite,
    case_filters: &[String],
    tag_filters: &[String],
) -> Result<SelectedCases, String> {
    let known = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    for filter in case_filters {
        if !known.contains(filter.as_str()) {
            return Err(format!("unknown case: {filter}"));
        }
    }
    let eligible = cases.iter().filter(|case| suite.admits(case.tier));
    let selected = eligible
        .filter(|case| case_filters.is_empty() || case_filters.contains(&case.id))
        .filter(|case| {
            tag_filters.is_empty() || tag_filters.iter().all(|tag| case.tags.contains(tag))
        })
        .cloned()
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("no cases selected".into());
    }
    let mandatory_total = cases
        .iter()
        .filter(|case| suite.admits(case.tier) && case.mandatory)
        .count();
    let mandatory_selected = selected.iter().filter(|case| case.mandatory).count();
    Ok(SelectedCases {
        cases: selected,
        partial: mandatory_selected != mandatory_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_catalog_is_unique_closed_and_admitted() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cases = catalog();
        validate_catalog(&cases, &workspace).unwrap();
        assert!(cases.windows(2).all(|window| window[0].id != window[1].id));
        let smoke = select_cases(&cases, Suite::Smoke, &[], &[]).unwrap();
        let shell = select_cases(&cases, Suite::ShellParity, &[], &[]).unwrap();
        let full = select_cases(&cases, Suite::Full, &[], &[]).unwrap();
        assert!(smoke.cases.len() < shell.cases.len());
        assert!(shell.cases.len() < full.cases.len());
        assert!(!smoke.partial && !shell.partial && !full.partial);
    }

    #[test]
    fn screen_snipping_case_is_mandatory_broker_bounded_and_privacy_preserving() {
        let case = catalog()
            .into_iter()
            .find(|case| case.id == "gui-win-shift-s-snipping")
            .expect("screen snipping case");
        assert!(case.mandatory && !case.explorer_free);
        assert!(matches!(case.recovery, Recovery::None));
        assert!(
            case.artifacts
                .iter()
                .any(|artifact| artifact.path.ends_with("headful-report.json"))
        );
        assert!(
            case.artifacts
                .iter()
                .any(|artifact| artifact.path.ends_with("screen-snip.log"))
        );
        assert!(
            !case
                .artifacts
                .iter()
                .any(|artifact| artifact.path.ends_with(".png"))
        );
    }

    #[test]
    fn duplicate_zero_timeout_escape_and_missing_watchdog_fail() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cases = catalog();
        let mut bad = cases[0].clone();
        bad.timeout_seconds = 0;
        bad.explorer_free = true;
        bad.recovery = Recovery::None;
        bad.program = ProgramSpec::PowerShell {
            script: "../escape.ps1".into(),
            args: vec!["ok;bad".into()],
        };
        cases.push(bad);
        let errors = validate_catalog(&cases, &workspace).unwrap_err().join("|");
        for expected in [
            "duplicate-case-id",
            "zero-timeout",
            "missing-explorer-watchdog",
            "invalid-script",
            "shell-token-rejected",
        ] {
            assert!(errors.contains(expected), "missing {expected}: {errors}");
        }
    }

    #[test]
    fn filters_are_explicit_partial_and_unknown_fails() {
        let cases = catalog();
        let selected = select_cases(&cases, Suite::Smoke, &["unit-utit".into()], &[]).unwrap();
        assert!(selected.partial);
        assert_eq!(selected.cases.len(), 1);
        assert!(select_cases(&cases, Suite::Smoke, &["missing".into()], &[]).is_err());
        assert!(select_cases(&cases, Suite::Smoke, &[], &["headful".into()]).is_err());
    }

    #[test]
    fn pointer_parity_cases_cover_applications_notifications_input_and_volume() {
        let cases = catalog();
        let pointer = select_cases(&cases, Suite::ShellParity, &[], &["pointer".into()]).unwrap();
        assert_eq!(
            pointer
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "gui-notification-overflow",
                "gui-system-status",
                "gui-taskbar-window-actions",
            ])
        );
        assert!(pointer.cases.iter().all(|case| case.mandatory));
        assert!(
            pointer
                .cases
                .iter()
                .filter(|case| case.explorer_free)
                .count()
                >= 2
        );
    }

    #[test]
    fn production_crates_never_depend_on_utit_and_executor_has_no_shell_string_route() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for manifest in std::fs::read_dir(workspace.join("crates")).unwrap() {
            let manifest = manifest.unwrap().path().join("Cargo.toml");
            if manifest.starts_with(env!("CARGO_MANIFEST_DIR")) || !manifest.is_file() {
                continue;
            }
            let source = std::fs::read_to_string(&manifest).unwrap();
            assert!(
                !source.contains("superdesktop-utit"),
                "production dependency on UTIT: {}",
                manifest.display()
            );
        }
        let executor = include_str!("executor.rs");
        for forbidden in [
            "cmd.exe",
            "cmd /c",
            "-Command",
            "env::vars",
            "std::env::vars",
        ] {
            assert!(
                !executor.contains(forbidden),
                "forbidden executor route: {forbidden}"
            );
        }
    }
}
