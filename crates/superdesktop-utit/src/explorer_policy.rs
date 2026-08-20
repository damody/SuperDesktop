use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerPolicyViolation {
    pub path: PathBuf,
    pub token: &'static str,
}

const FORBIDDEN: &[&str] = &["explorer.exe", "Shell_TrayWnd"];
const ALLOWED_SUFFIXES: &[&str] = &[
    "crates/platform-win/src/common/explorer_recovery.rs",
    "crates/superdesktop-guardian/src/recovery.rs",
    "crates/shell-installer/src/lib.rs",
    "crates/shell-installer/src/process_quiescence.rs",
];

pub fn audit_explorer_policy(workspace: &Path) -> Result<(), Vec<ExplorerPolicyViolation>> {
    let mut violations = Vec::new();
    for root in [
        "crates/superdesktop-app/src",
        "crates/explorer-bridge/src",
        "crates/taskbar-ui/src",
        "crates/desktop-ui/src",
        "crates/system-status-host/src",
        "crates/notification-area-host/src",
    ] {
        scan(workspace, &workspace.join(root), &mut violations);
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

fn scan(workspace: &Path, path: &Path, violations: &mut Vec<ExplorerPolicyViolation>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan(workspace, &path, violations);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let relative = path
            .strip_prefix(workspace)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED_SUFFIXES
            .iter()
            .any(|allowed| relative.ends_with(allowed))
        {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for token in FORBIDDEN {
            if production.contains(token) {
                violations.push(ExplorerPolicyViolation {
                    path: PathBuf::from(&relative),
                    token,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_gui_and_provider_paths_are_explorer_free() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        audit_explorer_policy(&workspace).unwrap();
    }

    #[test]
    fn forbidden_normal_path_fixtures_cover_every_owned_delegation_boundary() {
        let root = std::env::temp_dir().join(format!(
            "superdesktop-explorer-policy-{}",
            std::process::id()
        ));
        for (relative, body) in [
            (
                "crates/superdesktop-app/src/composition_bad.rs",
                "fn compose(){ launch(\"explorer.exe\"); }",
            ),
            (
                "crates/taskbar-ui/src/settings_bad.rs",
                "fn settings(){ launch(\"explorer.exe\"); }",
            ),
            (
                "crates/system-status-host/src/provider_bad.rs",
                "fn provider(){ find(\"Shell_TrayWnd\"); }",
            ),
            (
                "crates/notification-area-host/src/provider_bad.rs",
                "fn provider(){ launch(\"explorer.exe\"); }",
            ),
            (
                "crates/desktop-ui/src/superexplorer_bad.rs",
                "fn open_folder(){ launch(\"explorer.exe\"); }",
            ),
        ] {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
        let violations = audit_explorer_policy(&root).unwrap_err();
        assert_eq!(violations.len(), 5);
        assert_eq!(
            violations
                .iter()
                .filter(|violation| violation.token == "explorer.exe")
                .count(),
            4
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.token == "Shell_TrayWnd")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
