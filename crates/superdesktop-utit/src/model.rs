use std::{fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

pub const REPORT_SCHEMA: &str = "superdesktop-utit-run/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Suite {
    Smoke,
    ShellParity,
    Full,
}

impl Suite {
    pub const fn admits(self, tier: Tier) -> bool {
        match self {
            Self::Smoke => matches!(tier, Tier::Smoke),
            Self::ShellParity => matches!(tier, Tier::Smoke | Tier::ShellParity),
            Self::Full => true,
        }
    }
}

impl fmt::Display for Suite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Smoke => "smoke",
            Self::ShellParity => "shell-parity",
            Self::Full => "full",
        })
    }
}

impl FromStr for Suite {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "smoke" => Ok(Self::Smoke),
            "shell-parity" => Ok(Self::ShellParity),
            "full" => Ok(Self::Full),
            _ => Err(format!("unknown suite: {value}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Smoke,
    ShellParity,
    Full,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProgramSpec {
    Cargo { args: Vec<String> },
    OpenSpec { args: Vec<String> },
    PowerShell { script: String, args: Vec<String> },
    External { reason: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Prerequisite {
    Tool(String),
    File(String),
    Interactive,
    MultiDisplay,
    RebootAuthority,
    ExternalReview,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Recovery {
    None,
    ExplorerWatchdog { report: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedArtifact {
    pub path: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TestCase {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub tags: Vec<String>,
    pub timeout_seconds: u64,
    pub mandatory: bool,
    pub explorer_free: bool,
    pub program: ProgramSpec,
    pub prerequisites: Vec<Prerequisite>,
    pub recovery: Recovery,
    pub artifacts: Vec<ExpectedArtifact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HostFacts {
    pub windows_build: u32,
    pub architecture: String,
    pub interactive: bool,
    pub monitor_count: u32,
    pub explorer_running: bool,
    pub workspace_revision: String,
    pub tools: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalState {
    Passed,
    Failed,
    Blocked,
    Skipped,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactRecord {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub title: String,
    pub state: TerminalState,
    pub reason: String,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub argv: Vec<String>,
    pub stdout: Option<ArtifactRecord>,
    pub stderr: Option<ArtifactRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub recovery_verified: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunCounts {
    pub selected: usize,
    pub passed: usize,
    pub failed: usize,
    pub blocked: usize,
    pub skipped: usize,
    pub not_applicable: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunDecision {
    Passed,
    Failed,
    Incomplete,
    Partial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunReport {
    pub schema: String,
    pub run_id: String,
    pub suite: Suite,
    pub partial: bool,
    pub started_unix_ms: u128,
    pub finished_unix_ms: u128,
    pub workspace: String,
    pub host: HostFacts,
    pub counts: RunCounts,
    pub decision: RunDecision,
    pub cases: Vec<CaseResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedCases {
    pub cases: Vec<TestCase>,
    pub partial: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrerequisiteDisposition {
    Ready,
    Blocked(String),
    NotApplicable(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}
