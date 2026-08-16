use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use platform_win::common::desktop_operations::{
    CollisionPolicy, TransferTerminal, copy_file_cancellable, move_item, permanent_delete,
    recycle_item, rename_item, validate_filename,
};
use shell_core::{CorrelationId, ShellItemId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeletePolicy {
    Recycle,
    PermanentExplicit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferIntent {
    Copy,
    Move,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopOperation {
    Refresh,
    Rename {
        source: PathBuf,
        new_name: String,
    },
    Delete {
        source: PathBuf,
        policy: DeletePolicy,
    },
    Transfer {
        source: PathBuf,
        destination: PathBuf,
        intent: TransferIntent,
        collision: CollisionPolicy,
    },
    Reposition {
        item: ShellItemId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopOperationRequest {
    pub correlation_id: CorrelationId,
    pub operation: DesktopOperation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    pub completed_items: u32,
    pub total_items: u32,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopOperationTerminal {
    Succeeded,
    Cancelled,
    PartiallyFailed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopOperationError {
    InvalidName,
    MissingSource,
    MissingDestination,
    SameSourceAndDestination,
    DuplicateTerminal,
    UnknownOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingOperation {
    cancelled: bool,
    progress: OperationProgress,
    terminal: Option<DesktopOperationTerminal>,
}

#[derive(Clone, Debug, Default)]
pub struct DesktopOperationController {
    next_correlation: u128,
    pending: BTreeMap<CorrelationId, PendingOperation>,
    refresh_required: bool,
}

pub fn execute_desktop_operation(
    request: &DesktopOperationRequest,
    allowed_roots: &[PathBuf],
    mut should_continue: impl FnMut(u64, u64) -> bool,
) -> DesktopOperationTerminal {
    let result = match &request.operation {
        DesktopOperation::Refresh | DesktopOperation::Reposition { .. } => {
            return DesktopOperationTerminal::Succeeded;
        }
        DesktopOperation::Rename { source, new_name } => {
            rename_item(source, new_name, allowed_roots).map(|_| ())
        }
        DesktopOperation::Delete {
            source,
            policy: DeletePolicy::Recycle,
        } => recycle_item(source, allowed_roots),
        DesktopOperation::Delete {
            source,
            policy: DeletePolicy::PermanentExplicit,
        } => permanent_delete(source, allowed_roots, true),
        DesktopOperation::Transfer {
            source,
            destination,
            intent: TransferIntent::Copy,
            collision,
        } => {
            return match copy_file_cancellable(
                source,
                destination,
                allowed_roots,
                *collision,
                &mut should_continue,
            ) {
                Ok((_, TransferTerminal::Completed)) => DesktopOperationTerminal::Succeeded,
                Ok((_, TransferTerminal::Cancelled)) => DesktopOperationTerminal::Cancelled,
                Err(_) => DesktopOperationTerminal::Failed,
            };
        }
        DesktopOperation::Transfer {
            source,
            destination,
            intent: TransferIntent::Move,
            collision,
        } => move_item(source, destination, allowed_roots, *collision).map(|_| ()),
    };
    if result.is_ok() {
        DesktopOperationTerminal::Succeeded
    } else {
        DesktopOperationTerminal::Failed
    }
}

impl DesktopOperationController {
    pub fn plan(
        &mut self,
        operation: DesktopOperation,
    ) -> Result<DesktopOperationRequest, DesktopOperationError> {
        validate_operation(&operation)?;
        self.next_correlation = self.next_correlation.saturating_add(1);
        let correlation_id = CorrelationId(self.next_correlation);
        self.pending.insert(
            correlation_id,
            PendingOperation {
                cancelled: false,
                progress: OperationProgress {
                    completed_items: 0,
                    total_items: 1,
                    completed_bytes: 0,
                    total_bytes: 0,
                },
                terminal: None,
            },
        );
        Ok(DesktopOperationRequest {
            correlation_id,
            operation,
        })
    }

    pub fn cancel(&mut self, correlation_id: CorrelationId) -> bool {
        let Some(operation) = self.pending.get_mut(&correlation_id) else {
            return false;
        };
        if operation.terminal.is_some() {
            return false;
        }
        operation.cancelled = true;
        true
    }

    pub fn is_cancelled(&self, correlation_id: CorrelationId) -> bool {
        self.pending
            .get(&correlation_id)
            .is_some_and(|value| value.cancelled)
    }

    pub fn progress(&mut self, correlation_id: CorrelationId, progress: OperationProgress) -> bool {
        let Some(operation) = self.pending.get_mut(&correlation_id) else {
            return false;
        };
        if operation.terminal.is_some()
            || progress.completed_items > progress.total_items
            || progress.completed_bytes > progress.total_bytes
        {
            return false;
        }
        operation.progress = progress;
        true
    }

    pub fn terminal(
        &mut self,
        correlation_id: CorrelationId,
        terminal: DesktopOperationTerminal,
    ) -> Result<(), DesktopOperationError> {
        let operation = self
            .pending
            .get_mut(&correlation_id)
            .ok_or(DesktopOperationError::UnknownOperation)?;
        if operation.terminal.replace(terminal).is_some() {
            return Err(DesktopOperationError::DuplicateTerminal);
        }
        self.refresh_required = true;
        Ok(())
    }

    pub fn take_refresh_required(&mut self) -> bool {
        std::mem::take(&mut self.refresh_required)
    }

    pub fn reconcile_selection(
        &self,
        selected: &mut BTreeSet<ShellItemId>,
        available: &BTreeSet<ShellItemId>,
    ) {
        selected.retain(|item| available.contains(item));
    }
}

fn validate_operation(operation: &DesktopOperation) -> Result<(), DesktopOperationError> {
    match operation {
        DesktopOperation::Rename { source, new_name } => {
            if source.as_os_str().is_empty() {
                return Err(DesktopOperationError::MissingSource);
            }
            validate_filename(new_name).map_err(|_| DesktopOperationError::InvalidName)
        }
        DesktopOperation::Delete { source, .. } if source.as_os_str().is_empty() => {
            Err(DesktopOperationError::MissingSource)
        }
        DesktopOperation::Transfer {
            source,
            destination,
            ..
        } => {
            if source.as_os_str().is_empty() {
                Err(DesktopOperationError::MissingSource)
            } else if destination.as_os_str().is_empty() {
                Err(DesktopOperationError::MissingDestination)
            } else if source == destination {
                Err(DesktopOperationError::SameSourceAndDestination)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_lifecycle_cancel_and_first_terminal_are_deterministic() {
        let mut controller = DesktopOperationController::default();
        assert_eq!(
            controller.plan(DesktopOperation::Rename {
                source: "item.txt".into(),
                new_name: "bad?.txt".into(),
            }),
            Err(DesktopOperationError::InvalidName)
        );
        let request = controller.plan(DesktopOperation::Refresh).unwrap();
        assert!(controller.cancel(request.correlation_id));
        assert!(controller.is_cancelled(request.correlation_id));
        controller
            .terminal(request.correlation_id, DesktopOperationTerminal::Cancelled)
            .unwrap();
        assert_eq!(
            controller.terminal(request.correlation_id, DesktopOperationTerminal::Succeeded),
            Err(DesktopOperationError::DuplicateTerminal)
        );
        assert!(controller.take_refresh_required());
        assert!(!controller.take_refresh_required());
    }

    #[test]
    fn reconcile_keeps_only_surviving_stable_identities() {
        let one = ShellItemId::new("one").unwrap();
        let two = ShellItemId::new("two").unwrap();
        let mut selected = [one.clone(), two].into_iter().collect();
        DesktopOperationController::default()
            .reconcile_selection(&mut selected, &[one.clone()].into_iter().collect());
        assert_eq!(selected, [one].into_iter().collect());
    }
}
