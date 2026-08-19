#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ShowDesktopTarget {
    pub hwnd_identity: isize,
    pub process_id: u32,
    pub window_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShowDesktopObservation {
    pub target: ShowDesktopTarget,
    pub visible: bool,
    pub tool_window: bool,
    pub cloaked: bool,
    pub owned_transient: bool,
    pub minimized: bool,
}

impl ShowDesktopObservation {
    fn eligible(&self) -> bool {
        self.visible && !self.tool_window && !self.cloaked && !self.owned_transient
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShowDesktopPlan {
    Minimize(Vec<ShowDesktopTarget>),
    Restore(Vec<ShowDesktopTarget>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShowDesktopSession {
    restore_targets: Vec<ShowDesktopTarget>,
}

impl ShowDesktopSession {
    pub fn is_active(&self) -> bool {
        !self.restore_targets.is_empty()
    }

    pub fn plan(&self, snapshot: &[ShowDesktopObservation]) -> ShowDesktopPlan {
        if self.is_active() {
            let mut targets = snapshot
                .iter()
                .filter(|window| window.eligible() && window.minimized)
                .filter(|window| self.restore_targets.binary_search(&window.target).is_ok())
                .map(|window| window.target.clone())
                .collect::<Vec<_>>();
            targets.sort();
            targets.dedup();
            ShowDesktopPlan::Restore(targets)
        } else {
            let mut targets = snapshot
                .iter()
                .filter(|window| window.eligible() && !window.minimized)
                .map(|window| window.target.clone())
                .collect::<Vec<_>>();
            targets.sort();
            targets.dedup();
            ShowDesktopPlan::Minimize(targets)
        }
    }

    pub fn complete_minimize(&mut self, mut succeeded: Vec<ShowDesktopTarget>) {
        succeeded.sort();
        succeeded.dedup();
        self.restore_targets = succeeded;
    }

    pub fn complete_restore(&mut self) {
        self.restore_targets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(hwnd: isize, process_id: u32, minimized: bool) -> ShowDesktopObservation {
        ShowDesktopObservation {
            target: ShowDesktopTarget {
                hwnd_identity: hwnd,
                process_id,
                window_identity: format!("win:{process_id}:{hwnd:X}"),
            },
            visible: true,
            tool_window: false,
            cloaked: false,
            owned_transient: false,
            minimized,
        }
    }

    #[test]
    fn first_cycle_selects_only_visible_eligible_non_minimized_windows() {
        let visible = observed(2, 20, false);
        let minimized = observed(1, 10, true);
        let mut tool = observed(3, 30, false);
        tool.tool_window = true;
        let mut cloaked = observed(4, 40, false);
        cloaked.cloaked = true;
        let mut transient = observed(5, 50, false);
        transient.owned_transient = true;
        let mut hidden = observed(6, 60, false);
        hidden.visible = false;
        let session = ShowDesktopSession::default();
        assert_eq!(
            session.plan(&[minimized, tool, cloaked, transient, hidden, visible.clone()]),
            ShowDesktopPlan::Minimize(vec![visible.target.clone()])
        );
    }

    #[test]
    fn partial_success_is_the_only_restore_set() {
        let one = observed(1, 10, false);
        let two = observed(2, 20, false);
        let mut session = ShowDesktopSession::default();
        session.complete_minimize(vec![two.target.clone()]);
        assert!(session.is_active());
        assert_eq!(
            session.plan(&[observed(1, 10, true), observed(2, 20, true)]),
            ShowDesktopPlan::Restore(vec![two.target])
        );
        assert_ne!(one.target, observed(1, 11, true).target);
    }

    #[test]
    fn restore_requires_complete_fresh_identity_and_eligibility() {
        let admitted = observed(7, 70, false);
        let mut session = ShowDesktopSession::default();
        session.complete_minimize(vec![admitted.target]);
        let pid_reused = observed(7, 71, true);
        let mut identity_changed = observed(7, 70, true);
        identity_changed
            .target
            .window_identity
            .push_str(":replacement");
        let new_window = observed(8, 80, true);
        assert_eq!(
            session.plan(&[pid_reused, identity_changed, new_window]),
            ShowDesktopPlan::Restore(Vec::new())
        );
    }

    #[test]
    fn completion_clears_session_and_allows_a_new_cycle() {
        let target = observed(9, 90, false);
        let mut session = ShowDesktopSession::default();
        session.complete_minimize(vec![target.target.clone(), target.target.clone()]);
        assert!(session.is_active());
        session.complete_restore();
        assert!(!session.is_active());
        assert_eq!(
            session.plan(std::slice::from_ref(&target)),
            ShowDesktopPlan::Minimize(vec![target.target])
        );
    }

    #[test]
    fn empty_or_failed_minimize_does_not_create_active_session() {
        let mut session = ShowDesktopSession::default();
        assert_eq!(session.plan(&[]), ShowDesktopPlan::Minimize(Vec::new()));
        session.complete_minimize(Vec::new());
        assert!(!session.is_active());
    }
}
