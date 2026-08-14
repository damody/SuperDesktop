use std::process::{Command, Stdio};

use platform_win::common::ffi_boundary::{CallbackFence, CallbackResult};

use crate::{
    LaunchSpec,
    admission::AdmissionTerminal,
    resolver::{executable_identity, redact},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LaunchOutcome {
    Launched { process_id: u32 },
    ValidationFailed(&'static str),
    SpawnFailed(String),
}
#[derive(Clone, Debug, Default)]
pub struct ProcessLauncher;
impl ProcessLauncher {
    pub fn launch(&self, spec: &LaunchSpec) -> LaunchOutcome {
        match executable_identity(&spec.application) {
            Ok(identity) if identity == spec.executable_identity => {}
            Ok(_) => return LaunchOutcome::ValidationFailed("executable-identity-changed"),
            Err(reason) => return LaunchOutcome::ValidationFailed(reason),
        }
        let mut command = Command::new(&spec.application);
        command
            .envs(spec.child_environment.iter())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match command.spawn() {
            Ok(child) => {
                let process_id = child.id();
                drop(child);
                LaunchOutcome::Launched { process_id }
            }
            Err(error) => LaunchOutcome::SpawnFailed(format!(
                "spawn:{}:{}",
                error.kind(),
                redact(&spec.application)
            )),
        }
    }
}
pub fn invoke_completion_callback(
    fence: &CallbackFence,
    terminal: AdmissionTerminal,
) -> CallbackResult<AdmissionTerminal> {
    fence.invoke(|| terminal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutableResolver, build_default_launch};
    use platform_win::common::native_window::resource_snapshot;
    use std::path::PathBuf;
    fn cmd_spec() -> LaunchSpec {
        let system = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap()
            .join("System32/cmd.exe");
        let resolver = ExecutableResolver {
            setting: Some(system.clone()),
            developer_release: system.clone(),
            adjacent: system,
        };
        let (executable, _) = resolver.resolve().unwrap();
        build_default_launch(&executable)
    }
    #[test]
    fn explicit_application_spawn_and_raii_handle_cleanup() {
        let before = resource_snapshot().unwrap();
        let outcome = ProcessLauncher.launch(&cmd_spec());
        assert!(matches!(outcome, LaunchOutcome::Launched { .. }));
        std::thread::sleep(std::time::Duration::from_millis(100));
        let after = resource_snapshot().unwrap();
        assert!(i64::from(after.process_handles) - i64::from(before.process_handles) <= 2)
    }
    #[test]
    fn removed_or_changed_executable_fails_before_spawn() {
        let mut spec = cmd_spec();
        spec.executable_identity = "changed".into();
        assert_eq!(
            ProcessLauncher.launch(&spec),
            LaunchOutcome::ValidationFailed("executable-identity-changed")
        );
        spec.application = PathBuf::from(r"C:\missing\SuperExplorer.exe");
        assert!(matches!(
            ProcessLauncher.launch(&spec),
            LaunchOutcome::ValidationFailed(_)
        ))
    }
    #[test]
    fn completion_callback_uses_frozen_no_unwind_fence() {
        let fence = CallbackFence::default();
        assert!(matches!(
            invoke_completion_callback(&fence, AdmissionTerminal::Launched),
            CallbackResult::Returned(AdmissionTerminal::Launched)
        ))
    }
}
