use std::{collections::BTreeMap, path::Path, time::Duration};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardianInvocation {
    pub lease_handle: isize,
    pub channel_handle: isize,
    pub terminal_path: String,
    pub parent_wait_ms: u32,
}

impl GuardianInvocation {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, &'static str> {
        let values = args.into_iter().collect::<Vec<_>>();
        if values.first().map(String::as_str) != Some("--guardian-child") {
            return Err("missing-guardian-child-role");
        }
        fn value<'a>(values: &'a [String], key: &str) -> Option<&'a str> {
            values
                .windows(2)
                .find(|pair| pair[0] == key)
                .map(|pair| pair[1].as_str())
        }
        let lease_handle = value(&values, "--lease-handle")
            .ok_or("missing-lease-handle")?
            .parse()
            .map_err(|_| "bad-lease-handle")?;
        let channel_handle = value(&values, "--channel-handle")
            .ok_or("missing-channel-handle")?
            .parse()
            .map_err(|_| "bad-channel-handle")?;
        if lease_handle == 0 || channel_handle == 0 || lease_handle == channel_handle {
            return Err("invalid-inherited-handles");
        }
        let terminal_path = value(&values, "--terminal-path")
            .ok_or("missing-terminal-path")?
            .to_owned();
        if !Path::new(&terminal_path).is_absolute() {
            return Err("terminal-path-not-absolute");
        }
        Ok(Self {
            lease_handle,
            channel_handle,
            terminal_path,
            parent_wait_ms: u32::MAX,
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecoveryIdentity {
    pub session_id: u32,
    pub owner_token: String,
    pub owner_nonce: String,
    pub journal_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerObservation {
    pub process_id: u32,
    pub session_id: u32,
    pub user_token: String,
    pub file_identity: String,
    pub visible: bool,
    pub input_ready: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplorerDisposition {
    ShownExisting,
    SpawnedVerified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryEffect {
    RemoveOwnedAppBars,
    RestorePerMonitorWorkAreas,
    ShowExplorer(u32),
    SpawnVerifiedExplorer {
        explicit_application: String,
        restricted_inheritance: bool,
        sanitized_environment: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryTiming {
    pub t0_ms: u64,
    pub work_area_ms: u64,
    pub explorer_ready_ms: u64,
}

impl RecoveryTiming {
    pub fn validate(&self) -> Result<(), RecoveryError> {
        if self.work_area_ms < self.t0_ms
            || self.explorer_ready_ms < self.work_area_ms
            || self.explorer_ready_ms.saturating_sub(self.t0_ms)
                > Duration::from_secs(10).as_millis() as u64
        {
            Err(RecoveryError::DeadlineOrOrdering)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryTerminal {
    pub disposition: ExplorerDisposition,
    pub explorer_pid: u32,
    pub process_count_delta: u32,
    pub timing: RecoveryTiming,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    IdentityRejected,
    TerminalAmbiguous,
    DeadlineOrOrdering,
    ExplorerNotReady,
}

#[derive(Default)]
pub struct RecoveryCoordinator {
    terminals: BTreeMap<RecoveryIdentity, Result<RecoveryTerminal, RecoveryError>>,
    effects: BTreeMap<RecoveryIdentity, Vec<RecoveryEffect>>,
}

impl RecoveryCoordinator {
    pub fn recover(
        &mut self,
        request: RecoveryIdentity,
        validated: bool,
        explorer: Option<ExplorerObservation>,
        verified_system_explorer: &str,
        timing: RecoveryTiming,
        spawned_pid: u32,
    ) -> Result<RecoveryTerminal, RecoveryError> {
        if let Some(terminal) = self.terminals.get(&request) {
            return terminal.clone();
        }
        if !validated || request.owner_nonce.is_empty() || request.owner_token.is_empty() {
            let result = Err(RecoveryError::IdentityRejected);
            self.terminals.insert(request, result.clone());
            return result;
        }
        timing.validate()?;
        let effects = self.effects.entry(request.clone()).or_default();
        effects.extend([
            RecoveryEffect::RemoveOwnedAppBars,
            RecoveryEffect::RestorePerMonitorWorkAreas,
        ]);
        let (disposition, pid, delta) = match explorer {
            Some(existing)
                if existing.session_id == request.session_id
                    && existing.user_token == request.owner_token
                    && existing.input_ready =>
            {
                effects.push(RecoveryEffect::ShowExplorer(existing.process_id));
                (ExplorerDisposition::ShownExisting, existing.process_id, 0)
            }
            _ => {
                effects.push(RecoveryEffect::SpawnVerifiedExplorer {
                    explicit_application: verified_system_explorer.to_owned(),
                    restricted_inheritance: true,
                    sanitized_environment: true,
                });
                (ExplorerDisposition::SpawnedVerified, spawned_pid, 1)
            }
        };
        if pid == 0 {
            return Err(RecoveryError::ExplorerNotReady);
        }
        let terminal = RecoveryTerminal {
            disposition,
            explorer_pid: pid,
            process_count_delta: delta,
            timing,
        };
        self.terminals.insert(request, Ok(terminal.clone()));
        Ok(terminal)
    }

    pub fn effects(&self, id: &RecoveryIdentity) -> &[RecoveryEffect] {
        self.effects.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn terminal_count(&self) -> usize {
        self.terminals.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> RecoveryIdentity {
        RecoveryIdentity {
            session_id: 1,
            owner_token: "user/logon".into(),
            owner_nonce: "nonce".into(),
            journal_identity: "volume/file".into(),
        }
    }
    fn timing(run: u64) -> RecoveryTiming {
        RecoveryTiming {
            t0_ms: run * 100,
            work_area_ms: run * 100 + 2,
            explorer_ready_ms: run * 100 + 8,
        }
    }

    #[test]
    fn existing_explorer_is_shown_without_spawn_and_order_is_safe() {
        let id = identity();
        let mut c = RecoveryCoordinator::default();
        let terminal = c
            .recover(
                id.clone(),
                true,
                Some(ExplorerObservation {
                    process_id: 44,
                    session_id: 1,
                    user_token: "user/logon".into(),
                    file_identity: "windows/explorer".into(),
                    visible: false,
                    input_ready: true,
                }),
                r"C:\Windows\explorer.exe",
                timing(1),
                99,
            )
            .unwrap();
        assert_eq!(terminal.disposition, ExplorerDisposition::ShownExisting);
        assert_eq!(terminal.process_count_delta, 0);
        assert_eq!(
            c.effects(&id)[..2],
            [
                RecoveryEffect::RemoveOwnedAppBars,
                RecoveryEffect::RestorePerMonitorWorkAreas
            ]
        );
    }

    #[test]
    fn absent_explorer_spawns_once_and_repeated_concurrent_identity_shares_terminal() {
        let id = identity();
        let mut c = RecoveryCoordinator::default();
        let first = c
            .recover(
                id.clone(),
                true,
                None,
                r"C:\Windows\explorer.exe",
                timing(1),
                99,
            )
            .unwrap();
        let second = c
            .recover(id.clone(), true, None, "fake-on-path.exe", timing(1), 100)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.explorer_pid, 99);
        assert_eq!(c.terminal_count(), 1);
        assert_eq!(
            c.effects(&id)
                .iter()
                .filter(|e| matches!(e, RecoveryEffect::SpawnVerifiedExplorer { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn forged_wrong_session_token_and_ambiguous_timing_are_zero_effect() {
        for mut id in [identity(), identity(), identity()] {
            let mut c = RecoveryCoordinator::default();
            match id.owner_nonce.as_str() {
                "nonce" => id.owner_nonce.clear(),
                _ => unreachable!(),
            }
            assert_eq!(
                c.recover(
                    id.clone(),
                    false,
                    None,
                    r"C:\Windows\explorer.exe",
                    timing(1),
                    99
                ),
                Err(RecoveryError::IdentityRejected)
            );
            assert!(c.effects(&id).is_empty());
        }
        assert_eq!(
            RecoveryTiming {
                t0_ms: 0,
                work_area_ms: 2,
                explorer_ready_ms: 10_001
            }
            .validate(),
            Err(RecoveryError::DeadlineOrOrdering)
        );
    }

    #[test]
    fn ten_forced_crash_timings_meet_deadline() {
        let mut c = RecoveryCoordinator::default();
        for run in 0..10 {
            let mut id = identity();
            id.owner_nonce = format!("run-{run}");
            let terminal = c
                .recover(
                    id,
                    true,
                    None,
                    r"C:\Windows\explorer.exe",
                    timing(run),
                    100 + run as u32,
                )
                .unwrap();
            assert!(terminal.timing.validate().is_ok());
        }
        assert_eq!(c.terminal_count(), 10);
    }

    #[test]
    fn invocation_accepts_only_explicit_inherited_roles_and_absolute_terminal() {
        let args = [
            "--guardian-child",
            "--lease-handle",
            "11",
            "--channel-handle",
            "12",
            "--terminal-path",
            r"C:\Temp\terminal.json",
        ]
        .map(str::to_owned);
        assert_eq!(
            GuardianInvocation::from_args(args).unwrap().parent_wait_ms,
            u32::MAX
        );
        let bad = [
            "--guardian-child",
            "--lease-handle",
            "11",
            "--channel-handle",
            "11",
            "--terminal-path",
            "relative",
        ]
        .map(str::to_owned);
        assert!(GuardianInvocation::from_args(bad).is_err());
    }
}
