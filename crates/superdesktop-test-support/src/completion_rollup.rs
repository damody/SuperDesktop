use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const REQUIRED_COMPLETION_CHILDREN: [&str; 8] = [
    "add-superdesktop-desktop-file-operations",
    "add-superdesktop-notification-area-host",
    "add-superdesktop-shell-context-menu-host",
    "add-superdesktop-shell-installer",
    "add-superdesktop-start-search",
    "add-superdesktop-taskbar-advanced-interactions",
    "add-superdesktop-virtual-desktops",
    "extend-superdesktop-shell-contracts",
];

pub const REQUIRED_COMPLETION_GATES: [&str; 13] = [
    "G-A11Y-I18N",
    "G-ARCH",
    "G-DESKTOP",
    "G-DPI-MONITOR-PHYSICAL",
    "G-DPI-MONITOR-VIRTUAL",
    "G-GUARDIAN-RECOVERY",
    "G-INSTALL-ROLLBACK",
    "G-PERF",
    "G-REVIEW",
    "G-SAFETY",
    "G-SHELL-TAKEOVER",
    "G-TASKBAR",
    "G-TRACE",
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSource {
    pub change: String,
    pub relative_path: String,
    pub sha256: String,
    pub local_result: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalEvidenceSource {
    pub kind: String,
    pub relative_path: String,
    pub sha256: String,
    pub status: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDisposition {
    Passed,
    ExternalPending,
    Failed,
}

impl GateDisposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::ExternalPending => "external_pending",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityLimitation {
    pub capability: String,
    pub disposition: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RollupDecision {
    pub release_allowed: bool,
    pub disposition: String,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompletionRollup {
    pub schema_version: u32,
    pub generated_at_utc: String,
    pub sources: BTreeMap<String, EvidenceSource>,
    pub external_sources: BTreeMap<String, ExternalEvidenceSource>,
    pub gates: BTreeMap<String, GateDisposition>,
    pub limitations: Vec<CapabilityLimitation>,
    pub commands: Vec<String>,
    pub decision: RollupDecision,
}

impl CompletionRollup {
    pub fn build(
        generated_at_utc: String,
        sources: impl IntoIterator<Item = EvidenceSource>,
        gates: BTreeMap<String, GateDisposition>,
        mut limitations: Vec<CapabilityLimitation>,
        commands: Vec<String>,
    ) -> Result<Self, Vec<String>> {
        let mut source_map = BTreeMap::new();
        let mut structural_errors = Vec::new();
        for source in sources {
            if source.sha256.len() != 64
                || !source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                structural_errors.push(format!("invalid-source-hash:{}", source.change));
            }
            if source.local_result != "passed" {
                structural_errors.push(format!("source-not-passed:{}", source.change));
            }
            let key = source.change.clone();
            if source_map.insert(key.clone(), source).is_some() {
                structural_errors.push(format!("duplicate-source:{key}"));
            }
        }
        check_exact_set(
            "source",
            source_map.keys().map(String::as_str),
            &REQUIRED_COMPLETION_CHILDREN,
            &mut structural_errors,
        );
        check_exact_set(
            "gate",
            gates.keys().map(String::as_str),
            &REQUIRED_COMPLETION_GATES,
            &mut structural_errors,
        );
        if generated_at_utc.trim().is_empty() {
            structural_errors.push("missing-generated-at".into());
        }
        if commands.is_empty() {
            structural_errors.push("missing-command-log".into());
        }
        if !structural_errors.is_empty() {
            structural_errors.sort();
            return Err(structural_errors);
        }
        limitations.sort_by(|left, right| left.capability.cmp(&right.capability));
        let blockers = gates
            .iter()
            .filter(|(_, disposition)| **disposition != GateDisposition::Passed)
            .map(|(gate, disposition)| {
                format!("{}:{}", gate.to_ascii_lowercase(), disposition.as_str())
            })
            .collect::<Vec<_>>();
        let decision = RollupDecision {
            release_allowed: blockers.is_empty(),
            disposition: if blockers.is_empty() {
                "passed".into()
            } else {
                "blocked".into()
            },
            blockers,
        };
        Ok(Self {
            schema_version: 1,
            generated_at_utc,
            sources: source_map,
            external_sources: BTreeMap::new(),
            gates,
            limitations,
            commands,
            decision,
        })
    }

    pub fn verify_derived_decision(&self) -> bool {
        let expected = self
            .gates
            .iter()
            .filter(|(_, disposition)| **disposition != GateDisposition::Passed)
            .map(|(gate, disposition)| {
                format!("{}:{}", gate.to_ascii_lowercase(), disposition.as_str())
            })
            .collect::<Vec<_>>();
        self.decision.blockers == expected
            && self.decision.release_allowed == expected.is_empty()
            && self.decision.disposition
                == if expected.is_empty() {
                    "passed"
                } else {
                    "blocked"
                }
    }
}

fn check_exact_set<'a>(
    kind: &str,
    actual: impl Iterator<Item = &'a str>,
    expected: &[&str],
    errors: &mut Vec<String>,
) {
    let actual = actual.collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    for missing in expected.difference(&actual) {
        errors.push(format!("missing-{kind}:{missing}"));
    }
    for unexpected in actual.difference(&expected) {
        errors.push(format!("unexpected-{kind}:{unexpected}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(change: &str) -> EvidenceSource {
        EvidenceSource {
            change: change.into(),
            relative_path: format!("openspec/changes/{change}/evidence/verification.json"),
            sha256: "a".repeat(64),
            local_result: "passed".into(),
        }
    }

    fn gates(disposition: GateDisposition) -> BTreeMap<String, GateDisposition> {
        REQUIRED_COMPLETION_GATES
            .iter()
            .map(|gate| ((*gate).into(), disposition))
            .collect()
    }

    fn build(gates: BTreeMap<String, GateDisposition>) -> CompletionRollup {
        CompletionRollup::build(
            "2026-08-17T00:00:00Z".into(),
            REQUIRED_COMPLETION_CHILDREN.map(source),
            gates,
            vec![CapabilityLimitation {
                capability: "virtual-desktop-undocumented-operations".into(),
                disposition: "unavailable".into(),
                reason: "documented adapter exposes query and move only".into(),
            }],
            vec!["cargo test --workspace --offline".into()],
        )
        .unwrap()
    }

    #[test]
    fn exact_pass_set_is_deterministic_and_release_allowed() {
        let rollup = build(gates(GateDisposition::Passed));
        assert!(rollup.decision.release_allowed);
        assert!(rollup.verify_derived_decision());
        assert_eq!(
            serde_json::to_string(&rollup).unwrap(),
            serde_json::to_string(&rollup).unwrap()
        );
    }

    #[test]
    fn every_pending_or_failed_gate_blocks_release() {
        for gate in REQUIRED_COMPLETION_GATES {
            for disposition in [GateDisposition::ExternalPending, GateDisposition::Failed] {
                let mut matrix = gates(GateDisposition::Passed);
                matrix.insert(gate.into(), disposition);
                let rollup = build(matrix);
                assert!(!rollup.decision.release_allowed, "{gate}");
                assert_eq!(rollup.decision.blockers.len(), 1);
            }
        }
    }

    #[test]
    fn missing_duplicate_unexpected_and_invalid_hash_sources_are_rejected() {
        let mut sources = REQUIRED_COMPLETION_CHILDREN.map(source).to_vec();
        sources.pop();
        sources.push(source(REQUIRED_COMPLETION_CHILDREN[0]));
        sources.push(source("unexpected-change"));
        sources[0].sha256 = "not-a-hash".into();
        let errors = CompletionRollup::build(
            "2026-08-17T00:00:00Z".into(),
            sources,
            gates(GateDisposition::Passed),
            vec![],
            vec!["test".into()],
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("missing-source:"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("duplicate-source:"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("unexpected-source:"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("invalid-source-hash:"))
        );
    }

    #[test]
    fn missing_or_unexpected_gate_is_structural_failure() {
        let mut matrix = gates(GateDisposition::Passed);
        matrix.remove(REQUIRED_COMPLETION_GATES[0]);
        matrix.insert("G-INVENTED".into(), GateDisposition::Passed);
        let errors = CompletionRollup::build(
            "2026-08-17T00:00:00Z".into(),
            REQUIRED_COMPLETION_CHILDREN.map(source),
            matrix,
            vec![],
            vec!["test".into()],
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("missing-gate:"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("unexpected-gate:"))
        );
    }

    #[test]
    fn checked_in_rollup_has_a_valid_derived_blocked_decision() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json",
        );
        let rollup: CompletionRollup =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(rollup.schema_version, 1);
        assert!(rollup.verify_derived_decision());
        assert!(!rollup.decision.release_allowed);
        assert!(
            rollup
                .external_sources
                .keys()
                .all(|kind| kind != "windows10-lifecycle-installer")
        );
        assert!(rollup.limitations.iter().any(|limitation| {
            limitation.capability == "windows-10-compatibility"
                && limitation.disposition == "not-claimed"
        }));
    }
}
