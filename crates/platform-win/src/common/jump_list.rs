//! Bounded, application-owned Jump List data collected outside the GPUI process.

use std::path::{Path, PathBuf};

use shell_provider_protocol::{CommandDescriptor, CommandId, CommandRisk, JumpListResponse};

pub fn enumerate(application_id: &str, limit: usize) -> JumpListResponse {
    let application = PathBuf::from(application_id)
        .canonicalize()
        .ok()
        .filter(|path| path.is_file());
    let _ = limit;
    let tasks = application
        .as_deref()
        .map(|path| vec![command_for_path("launch", "Open new window", path)])
        .unwrap_or_default();
    JumpListResponse {
        recent: Vec::new(),
        frequent: Vec::new(),
        tasks,
    }
}

fn command_for_path(prefix: &str, fallback: &str, path: &Path) -> CommandDescriptor {
    let label = path
        .file_stem()
        .or_else(|| path.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fallback)
        .to_owned();
    CommandDescriptor {
        id: CommandId(format!("jump:{prefix}:{}", path.display())),
        label,
        enabled: true,
        risk: CommandRisk::Normal,
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_application_never_creates_launch_task_or_unowned_destinations() {
        let output = enumerate(r"C:\definitely-missing\app.exe", 3);
        assert!(output.tasks.is_empty());
        assert!(output.recent.is_empty());
        assert!(output.frequent.is_empty());
    }

    #[test]
    fn current_executable_keeps_launch_task_without_global_recent_items() {
        let application = std::env::current_exe().unwrap();
        let output = enumerate(application.to_string_lossy().as_ref(), 20);
        assert!(output.recent.is_empty());
        assert!(output.frequent.is_empty());
        assert_eq!(output.tasks.len(), 1);
        assert!(output.tasks[0].id.0.starts_with("jump:launch:"));
    }
}
