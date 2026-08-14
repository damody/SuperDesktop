use std::path::Path;

use crate::{AdmissionTerminal, resolver::redact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    ZhTw,
    En,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairAction {
    Retry,
    OpenSettings,
    Dismiss,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairModel {
    pub title: &'static str,
    pub message: &'static str,
    pub actions: Vec<RepairAction>,
    pub accessible_role: &'static str,
    pub fallback_to_windows_explorer: bool,
}

pub fn repair_model(terminal: AdmissionTerminal, locale: Locale) -> Option<RepairModel> {
    if terminal == AdmissionTerminal::Launched {
        return None;
    }
    let (title, message, actions) = match (locale, terminal) {
        (Locale::ZhTw, AdmissionTerminal::ValidationFailed) => (
            "無法找到 SuperExplorer",
            "請在設定中選擇有效的 SuperExplorer 執行檔。",
            vec![
                RepairAction::OpenSettings,
                RepairAction::Retry,
                RepairAction::Dismiss,
            ],
        ),
        (Locale::En, AdmissionTerminal::ValidationFailed) => (
            "SuperExplorer unavailable",
            "Choose a valid SuperExplorer executable in Settings.",
            vec![
                RepairAction::OpenSettings,
                RepairAction::Retry,
                RepairAction::Dismiss,
            ],
        ),
        (Locale::ZhTw, AdmissionTerminal::SpawnFailed) => (
            "無法啟動 SuperExplorer",
            "啟動失敗；請重試或檢查設定。",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        (Locale::En, AdmissionTerminal::SpawnFailed) => (
            "Could not start SuperExplorer",
            "Launch failed; retry or review Settings.",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        (Locale::ZhTw, AdmissionTerminal::Cancelled) => (
            "已取消",
            "SuperExplorer 啟動已取消。",
            vec![RepairAction::Retry, RepairAction::Dismiss],
        ),
        (Locale::En, AdmissionTerminal::Cancelled) => (
            "Cancelled",
            "SuperExplorer launch was cancelled.",
            vec![RepairAction::Retry, RepairAction::Dismiss],
        ),
        (Locale::ZhTw, AdmissionTerminal::TimedOut) => (
            "啟動逾時",
            "SuperExplorer 未在五秒內回應。",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        (Locale::En, AdmissionTerminal::TimedOut) => (
            "Launch timed out",
            "SuperExplorer did not respond within five seconds.",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        (_, AdmissionTerminal::Launched) => unreachable!(),
    };
    Some(RepairModel {
        title,
        message,
        actions,
        accessible_role: "alert",
        fallback_to_windows_explorer: false,
    })
}
pub fn redacted_diagnostic(path: &Path, environment_keys: &[&str]) -> String {
    let keys = environment_keys
        .iter()
        .map(|key| {
            if key.eq_ignore_ascii_case("EXPLORER_INITIAL_PATH") {
                "EXPLORER_INITIAL_PATH=<redacted>"
            } else {
                "<environment-key-redacted>"
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("target={};environment={keys}", redact(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_failure_has_localized_keyboard_repair_without_explorer_fallback() {
        for terminal in [
            AdmissionTerminal::ValidationFailed,
            AdmissionTerminal::SpawnFailed,
            AdmissionTerminal::Cancelled,
            AdmissionTerminal::TimedOut,
        ] {
            for locale in [Locale::ZhTw, Locale::En] {
                let model = repair_model(terminal, locale).unwrap();
                assert_eq!(model.accessible_role, "alert");
                assert!(!model.actions.is_empty());
                assert!(!model.fallback_to_windows_explorer)
            }
        }
    }
    #[test]
    fn diagnostics_redact_profile_path_and_environment_value() {
        let path = Path::new(r"C:\Users\Private Name\SuperExplorer.exe");
        let diagnostic = redacted_diagnostic(path, &["EXPLORER_INITIAL_PATH", "TOKEN"]);
        assert!(!diagnostic.contains("Private Name"));
        assert!(!diagnostic.contains("TOKEN"));
        assert!(diagnostic.contains("<redacted>"))
    }
}
