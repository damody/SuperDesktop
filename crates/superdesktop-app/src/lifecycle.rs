use std::{
    ffi::OsString,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExecutionRequest {
    pub shell: bool,
}

impl ExecutionRequest {
    pub fn from_args(args: impl IntoIterator<Item = OsString>) -> Self {
        Self {
            shell: args.into_iter().any(|arg| arg == "--shell"),
        }
    }

    pub fn from_product_args(args: impl IntoIterator<Item = OsString>) -> Self {
        let preview = args.into_iter().any(|arg| arg == "--preview");
        Self { shell: !preview }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentFacts {
    pub safe_mode: bool,
    pub interactive: bool,
    pub session_active: bool,
    pub capability_go: bool,
}

impl EnvironmentFacts {
    pub fn supported_fixture() -> Self {
        Self {
            safe_mode: false,
            interactive: true,
            session_active: true,
            capability_go: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Preview,
    Shell,
}

impl Admission {
    pub fn evaluate(
        request: &ExecutionRequest,
        facts: &EnvironmentFacts,
    ) -> Result<Self, LifecycleError> {
        if !request.shell {
            return Ok(Self::Preview);
        }
        if facts.safe_mode {
            return Err(LifecycleError::SafeMode);
        }
        if !facts.interactive || !facts.session_active {
            return Err(LifecycleError::UnsupportedSession);
        }
        if !facts.capability_go {
            return Err(LifecycleError::CapabilityNotGo);
        }
        Ok(Self::Shell)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseIdentity {
    pub pid: u32,
    pub creation_time: u64,
    pub session_id: u32,
    pub user_token: String,
    pub executable_file: String,
    pub nonce: String,
}

#[derive(Clone, Default)]
pub struct LeaseRegistry(Arc<Mutex<Option<LeaseIdentity>>>);

impl LeaseRegistry {
    pub fn acquire(&self, identity: LeaseIdentity) -> Result<OwnerLease, LifecycleError> {
        let mut slot = self.0.lock().map_err(|_| LifecycleError::LeasePoisoned)?;
        if slot.is_some() {
            return Err(LifecycleError::AlreadyOwned);
        }
        *slot = Some(identity.clone());
        Ok(OwnerLease {
            registry: self.clone(),
            identity,
            released: false,
        })
    }

    pub fn current(&self) -> Option<LeaseIdentity> {
        self.0.lock().ok().and_then(|value| value.clone())
    }
}

pub struct OwnerLease {
    registry: LeaseRegistry,
    identity: LeaseIdentity,
    released: bool,
}

impl Drop for OwnerLease {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        if let Ok(mut slot) = self.registry.0.lock()
            && slot.as_ref() == Some(&self.identity)
        {
            *slot = None;
        }
    }
}

impl OwnerLease {
    pub fn revalidate(&self, observed: &LeaseIdentity) -> Result<(), LifecycleError> {
        if self.released
            || observed != &self.identity
            || self.registry.current().as_ref() != Some(&self.identity)
        {
            return Err(LifecycleError::NotOwner);
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        observed: &LeaseIdentity,
        mutation: Mutation,
    ) -> Result<Mutation, LifecycleError> {
        self.revalidate(observed)?;
        Ok(mutation)
    }

    pub fn release(&mut self, observed: &LeaseIdentity) -> Result<(), LifecycleError> {
        self.revalidate(observed)?;
        let mut slot = self
            .registry
            .0
            .lock()
            .map_err(|_| LifecycleError::LeasePoisoned)?;
        if slot.as_ref() != Some(&self.identity) {
            return Err(LifecycleError::NotOwner);
        }
        *slot = None;
        self.released = true;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mutation {
    RegisterAppBar,
    RemoveAppBar,
    SwitchExplorer,
    RestoreExplorer,
    RestoreWorkArea,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Phase {
    Prerequisites,
    Guardian,
    Surfaces,
    AppBarsAndHooks,
    InputHealth,
    ExplorerSwitch,
    Committed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalEntry {
    pub sequence: u64,
    pub phase: Phase,
    pub rollback_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthReport {
    pub elapsed: Duration,
    pub desktop_pointer: bool,
    pub desktop_keyboard: bool,
    pub taskbar_pointer: bool,
    pub taskbar_keyboard: bool,
    pub focus: bool,
    pub start_available: bool,
}

impl HealthReport {
    fn healthy(self) -> bool {
        self.elapsed <= Duration::from_secs(5)
            && self.desktop_pointer
            && self.desktop_keyboard
            && self.taskbar_pointer
            && self.taskbar_keyboard
            && self.focus
            && self.start_available
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownStep {
    StopCommands,
    CancelRequests,
    UnregisterHooks,
    RemoveAppBars,
    TeardownSurfaces,
    RestoreExplorer,
    ReleaseCom,
    FlushDurableState,
    GuardianTerminal,
    ReleaseLease,
}

#[derive(Default)]
pub struct TakeoverCoordinator {
    journal: Vec<JournalEntry>,
    mutations: Vec<Mutation>,
    shutdown: Vec<ShutdownStep>,
    committed: bool,
}

impl TakeoverCoordinator {
    pub fn execute(
        &mut self,
        lease: &OwnerLease,
        identity: &LeaseIdentity,
        health: HealthReport,
        fail_at: Option<Phase>,
    ) -> Result<(), LifecycleError> {
        self.journal.clear();
        self.mutations.clear();
        self.committed = false;
        for phase in [
            Phase::Prerequisites,
            Phase::Guardian,
            Phase::Surfaces,
            Phase::AppBarsAndHooks,
            Phase::InputHealth,
            Phase::ExplorerSwitch,
        ] {
            if fail_at == Some(phase) {
                self.rollback();
                return Err(LifecycleError::InjectedFailure(phase));
            }
            if phase == Phase::AppBarsAndHooks {
                self.mutations
                    .push(lease.authorize(identity, Mutation::RegisterAppBar)?);
            }
            if phase == Phase::InputHealth && !health.healthy() {
                self.rollback();
                return Err(LifecycleError::HealthFailed);
            }
            if phase == Phase::ExplorerSwitch {
                self.mutations
                    .push(lease.authorize(identity, Mutation::SwitchExplorer)?);
            }
            self.journal.push(JournalEntry {
                sequence: self.journal.len() as u64 + 1,
                phase,
                rollback_token: matches!(
                    phase,
                    Phase::Guardian | Phase::Surfaces | Phase::AppBarsAndHooks
                )
                .then(|| format!("rollback-{phase:?}")),
            });
        }
        self.committed = true;
        self.journal.push(JournalEntry {
            sequence: 7,
            phase: Phase::Committed,
            rollback_token: None,
        });
        Ok(())
    }

    fn rollback(&mut self) {
        if self.mutations.contains(&Mutation::RegisterAppBar) {
            self.mutations.push(Mutation::RemoveAppBar);
            self.mutations.push(Mutation::RestoreWorkArea);
        }
        self.mutations.retain(|m| *m != Mutation::SwitchExplorer);
    }

    pub fn shutdown(
        &mut self,
        lease: &mut OwnerLease,
        identity: &LeaseIdentity,
    ) -> Result<&[ShutdownStep], LifecycleError> {
        if !self.shutdown.is_empty() {
            return Ok(&self.shutdown);
        }
        lease.revalidate(identity)?;
        self.shutdown.extend([
            ShutdownStep::StopCommands,
            ShutdownStep::CancelRequests,
            ShutdownStep::UnregisterHooks,
            ShutdownStep::RemoveAppBars,
            ShutdownStep::TeardownSurfaces,
            ShutdownStep::RestoreExplorer,
            ShutdownStep::ReleaseCom,
            ShutdownStep::FlushDurableState,
            ShutdownStep::GuardianTerminal,
        ]);
        self.mutations.extend([
            lease.authorize(identity, Mutation::RemoveAppBar)?,
            lease.authorize(identity, Mutation::RestoreWorkArea)?,
            lease.authorize(identity, Mutation::RestoreExplorer)?,
        ]);
        lease.release(identity)?;
        self.shutdown.push(ShutdownStep::ReleaseLease);
        self.committed = false;
        Ok(&self.shutdown)
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }
    pub fn mutations(&self) -> &[Mutation] {
        &self.mutations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    SafeMode,
    UnsupportedSession,
    CapabilityNotGo,
    AlreadyOwned,
    NotOwner,
    LeasePoisoned,
    HealthFailed,
    InjectedFailure(Phase),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn id(nonce: &str) -> LeaseIdentity {
        LeaseIdentity {
            pid: 10,
            creation_time: 20,
            session_id: 1,
            user_token: "S-1-5-21-user/logon".into(),
            executable_file: "volume:7/file:8".into(),
            nonce: nonce.into(),
        }
    }
    fn health() -> HealthReport {
        HealthReport {
            elapsed: Duration::from_millis(25),
            desktop_pointer: true,
            desktop_keyboard: true,
            taskbar_pointer: true,
            taskbar_keyboard: true,
            focus: true,
            start_available: true,
        }
    }

    #[test]
    fn preview_is_default_and_rejections_are_zero_mutation() {
        assert_eq!(
            ExecutionRequest::default(),
            ExecutionRequest { shell: false }
        );
        assert_eq!(
            Admission::evaluate(
                &ExecutionRequest::default(),
                &EnvironmentFacts::supported_fixture()
            ),
            Ok(Admission::Preview)
        );
        for facts in [
            EnvironmentFacts {
                safe_mode: true,
                ..EnvironmentFacts::supported_fixture()
            },
            EnvironmentFacts {
                interactive: false,
                ..EnvironmentFacts::supported_fixture()
            },
            EnvironmentFacts {
                session_active: false,
                ..EnvironmentFacts::supported_fixture()
            },
        ] {
            assert!(Admission::evaluate(&ExecutionRequest { shell: true }, &facts).is_err());
        }
    }

    #[test]
    fn normal_product_launch_is_shell_and_preview_requires_explicit_opt_out() {
        assert_eq!(
            ExecutionRequest::from_product_args(Vec::<OsString>::new()),
            ExecutionRequest { shell: true }
        );
        assert_eq!(
            ExecutionRequest::from_product_args([OsString::from("--shell")]),
            ExecutionRequest { shell: true }
        );
        assert_eq!(
            ExecutionRequest::from_product_args([OsString::from("--preview")]),
            ExecutionRequest { shell: false }
        );
        assert_eq!(
            ExecutionRequest::from_args(Vec::<OsString>::new()),
            ExecutionRequest { shell: false }
        );
    }

    #[test]
    fn simultaneous_owner_race_has_one_winner_and_crash_drop_transfers() {
        let registry = LeaseRegistry::default();
        let handles: Vec<_> = (0..2)
            .map(|n| {
                let registry = registry.clone();
                thread::spawn(move || registry.acquire(id(&format!("nonce-{n}"))).ok())
            })
            .collect();
        let leases = handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(leases.len(), 1);
        drop(leases);
        assert!(
            registry.current().is_none(),
            "winning lease drops after owner crash/exit"
        );
        assert!(registry.acquire(id("transfer")).is_ok());
    }

    #[test]
    fn every_identity_field_and_non_owner_cleanup_is_fenced() {
        let registry = LeaseRegistry::default();
        let owner = id("owner");
        let lease = registry.acquire(owner.clone()).unwrap();
        let mut attacks = Vec::new();
        let mut v = owner.clone();
        v.pid += 1;
        attacks.push(v);
        let mut v = owner.clone();
        v.creation_time += 1;
        attacks.push(v);
        let mut v = owner.clone();
        v.session_id += 1;
        attacks.push(v);
        let mut v = owner.clone();
        v.user_token.push_str("-wrong");
        attacks.push(v);
        let mut v = owner.clone();
        v.executable_file.push_str("-replaced");
        attacks.push(v);
        let mut v = owner.clone();
        v.nonce.push_str("-wrong");
        attacks.push(v);
        for attack in attacks {
            assert_eq!(
                lease.authorize(&attack, Mutation::RemoveAppBar),
                Err(LifecycleError::NotOwner)
            );
        }
    }

    #[test]
    fn all_phase_failpoints_rollback_and_health_never_switches_explorer() {
        for phase in [
            Phase::Prerequisites,
            Phase::Guardian,
            Phase::Surfaces,
            Phase::AppBarsAndHooks,
            Phase::InputHealth,
            Phase::ExplorerSwitch,
        ] {
            let registry = LeaseRegistry::default();
            let identity = id("owner");
            let lease = registry.acquire(identity.clone()).unwrap();
            let mut tx = TakeoverCoordinator::default();
            assert_eq!(
                tx.execute(&lease, &identity, health(), Some(phase)),
                Err(LifecycleError::InjectedFailure(phase))
            );
            assert!(!tx.mutations().contains(&Mutation::SwitchExplorer));
        }
        let registry = LeaseRegistry::default();
        let identity = id("owner");
        let lease = registry.acquire(identity.clone()).unwrap();
        let mut bad = health();
        bad.elapsed = Duration::from_millis(5_001);
        let mut tx = TakeoverCoordinator::default();
        assert_eq!(
            tx.execute(&lease, &identity, bad, None),
            Err(LifecycleError::HealthFailed)
        );
        assert!(!tx.mutations().contains(&Mutation::SwitchExplorer));
    }

    #[test]
    fn commit_and_shutdown_are_strictly_ordered_and_idempotent() {
        let registry = LeaseRegistry::default();
        let identity = id("owner");
        let mut lease = registry.acquire(identity.clone()).unwrap();
        let mut tx = TakeoverCoordinator::default();
        tx.execute(&lease, &identity, health(), None).unwrap();
        assert_eq!(tx.journal().last().unwrap().phase, Phase::Committed);
        let first = tx.shutdown(&mut lease, &identity).unwrap().to_vec();
        let second = tx.shutdown(&mut lease, &identity).unwrap().to_vec();
        assert_eq!(first, second);
        assert_eq!(first.last(), Some(&ShutdownStep::ReleaseLease));
        assert!(registry.current().is_none());
    }
}
