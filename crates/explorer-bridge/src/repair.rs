use std::path::Path;

use explorer_i18n::Catalog;

use crate::{AdmissionTerminal, resolver::redact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairAction {
    Retry,
    OpenSettings,
    Dismiss,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairModel {
    pub title: String,
    pub message: String,
    pub actions: Vec<RepairAction>,
    pub accessible_role: &'static str,
    pub fallback_to_windows_explorer: bool,
}

pub fn repair_model(terminal: AdmissionTerminal, catalog: Catalog) -> Option<RepairModel> {
    if terminal == AdmissionTerminal::Launched {
        return None;
    }
    let (title_key, message_key, actions) = match terminal {
        AdmissionTerminal::ValidationFailed => (
            "desktop-repair-unavailable-title",
            "desktop-repair-unavailable-message",
            vec![
                RepairAction::OpenSettings,
                RepairAction::Retry,
                RepairAction::Dismiss,
            ],
        ),
        AdmissionTerminal::SpawnFailed => (
            "desktop-repair-spawn-title",
            "desktop-repair-spawn-message",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        AdmissionTerminal::Cancelled => (
            "desktop-repair-cancelled-title",
            "desktop-repair-cancelled-message",
            vec![RepairAction::Retry, RepairAction::Dismiss],
        ),
        AdmissionTerminal::TimedOut => (
            "desktop-repair-timeout-title",
            "desktop-repair-timeout-message",
            vec![
                RepairAction::Retry,
                RepairAction::OpenSettings,
                RepairAction::Dismiss,
            ],
        ),
        AdmissionTerminal::Launched => unreachable!(),
    };
    Some(RepairModel {
        title: catalog.t(title_key),
        message: catalog.t(message_key),
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
    use explorer_i18n::{AppLocale, Catalog};

    use super::*;

    fn strip_isolates(value: &str) -> String {
        value
            .chars()
            .filter(|ch| !matches!(*ch, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'))
            .collect()
    }

    #[test]
    fn every_failure_has_localized_keyboard_repair_without_explorer_fallback() {
        for terminal in [
            AdmissionTerminal::ValidationFailed,
            AdmissionTerminal::SpawnFailed,
            AdmissionTerminal::Cancelled,
            AdmissionTerminal::TimedOut,
        ] {
            for locale in AppLocale::ALL {
                let model = repair_model(terminal, Catalog::new(locale)).unwrap();
                assert_eq!(model.accessible_role, "alert");
                assert!(!model.actions.is_empty());
                assert!(!model.fallback_to_windows_explorer);
                assert_ne!(model.title, "");
                assert_ne!(model.message, "");
            }
        }
        let zh = repair_model(
            AdmissionTerminal::ValidationFailed,
            Catalog::new(AppLocale::ZhTw),
        )
        .unwrap();
        assert_eq!(strip_isolates(&zh.title), "無法找到 SuperExplorer");
        let ja = repair_model(
            AdmissionTerminal::TimedOut,
            Catalog::new(AppLocale::Ja),
        )
        .unwrap();
        assert_eq!(strip_isolates(&ja.title), "起動がタイムアウトしました");
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
