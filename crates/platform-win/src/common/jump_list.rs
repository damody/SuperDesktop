//! Bounded, owned Jump List data collected outside the GPUI process.

use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use shell_provider_protocol::{CommandDescriptor, CommandId, CommandRisk, JumpListResponse};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Recent, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

pub fn enumerate(application_id: &str, limit: usize) -> JumpListResponse {
    let application = PathBuf::from(application_id)
        .canonicalize()
        .ok()
        .filter(|path| path.is_file());
    let mut recent = recent_root()
        .and_then(|root| fs::read_dir(root).ok())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path().canonicalize().ok()?;
            path.is_file().then(|| {
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                (modified, path)
            })
        })
        .collect::<Vec<_>>();
    recent.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let recent = recent
        .into_iter()
        .take(limit.min(20))
        .map(|(_, path)| command_for_path("open", "Open", &path))
        .collect();
    let tasks = application
        .as_deref()
        .map(|path| vec![command_for_path("launch", "Open new window", path)])
        .unwrap_or_default();
    JumpListResponse {
        recent,
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

fn recent_root() -> Option<PathBuf> {
    // SAFETY: The static known-folder GUID is valid. The returned CoTaskMem
    // allocation is copied into PathBuf and freed exactly once.
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Recent, KF_FLAG_DEFAULT, None).ok()?;
        let result = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0.cast::<c_void>()));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_application_never_creates_launch_task_and_output_is_bounded() {
        let output = enumerate(r"C:\definitely-missing\app.exe", 3);
        assert!(output.tasks.is_empty());
        assert!(output.recent.len() <= 3);
    }
}
