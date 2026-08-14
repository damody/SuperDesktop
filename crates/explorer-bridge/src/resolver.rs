use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableCandidate {
    Setting,
    DeveloperRelease,
    Adjacent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedExecutable {
    pub path: PathBuf,
    pub candidate: ExecutableCandidate,
    pub identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolverTrace {
    pub selected: Option<ExecutableCandidate>,
    pub decisions: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ExecutableResolver {
    pub setting: Option<PathBuf>,
    pub developer_release: PathBuf,
    pub adjacent: PathBuf,
}

impl ExecutableResolver {
    pub fn resolve(&self) -> Result<(ResolvedExecutable, ResolverTrace), ResolverTrace> {
        let candidates = [
            self.setting
                .as_ref()
                .map(|path| (ExecutableCandidate::Setting, path)),
            Some((
                ExecutableCandidate::DeveloperRelease,
                &self.developer_release,
            )),
            Some((ExecutableCandidate::Adjacent, &self.adjacent)),
        ];
        let mut trace = ResolverTrace {
            selected: None,
            decisions: Vec::new(),
        };
        for (candidate, path) in candidates.into_iter().flatten() {
            match validate(path) {
                Ok((path, identity)) => {
                    trace.selected = Some(candidate);
                    trace
                        .decisions
                        .push(format!("{candidate:?}:accepted:{}", redact(&path)));
                    return Ok((
                        ResolvedExecutable {
                            path,
                            candidate,
                            identity,
                        },
                        trace,
                    ));
                }
                Err(reason) => trace
                    .decisions
                    .push(format!("{candidate:?}:rejected:{reason}:{}", redact(path))),
            }
        }
        Err(trace)
    }
}

fn validate(path: &Path) -> Result<(PathBuf, String), &'static str> {
    if !path.is_absolute() {
        return Err("not-absolute");
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "missing")?;
    if !metadata.is_file() {
        return Err("not-regular-file");
    }
    if std::os::windows::fs::MetadataExt::file_attributes(&metadata) & 0x400 != 0 {
        return Err("reparse-point");
    }
    let canonical = path.canonicalize().map_err(|_| "canonicalize")?;
    let identity = identity_from(&canonical, &metadata);
    Ok((canonical, identity))
}

pub(crate) fn executable_identity(path: &Path) -> Result<String, &'static str> {
    validate(path).map(|(_, identity)| identity)
}

fn identity_from(canonical: &Path, metadata: &fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "exe:{}:{}:{modified}",
        metadata.len(),
        canonical.to_string_lossy().to_ascii_lowercase()
    )
}

pub fn redact(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("<redacted>\\{name}"))
        .unwrap_or_else(|| "<redacted>".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn temp() -> PathBuf {
        std::env::temp_dir().join(format!(
            "superdesktop-resolver-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn priority_is_setting_then_developer_then_adjacent() {
        let root = temp();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let setting = root.join("setting.exe");
        let developer = root.join("developer.exe");
        fs::File::create(&setting).unwrap().write_all(b"x").unwrap();
        fs::File::create(&developer)
            .unwrap()
            .write_all(b"x")
            .unwrap();
        let resolver = ExecutableResolver {
            setting: Some(setting.clone()),
            developer_release: developer,
            adjacent: root.join("adjacent.exe"),
        };
        let (resolved, trace) = resolver.resolve().unwrap();
        assert_eq!(resolved.path, setting.canonicalize().unwrap());
        assert_eq!(trace.selected, Some(ExecutableCandidate::Setting));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_directory_relative_and_path_substitution_fail_closed() {
        let root = temp();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let resolver = ExecutableResolver {
            setting: Some(PathBuf::from("SuperExplorer.exe")),
            developer_release: root.clone(),
            adjacent: root.join("missing.exe"),
        };
        let trace = resolver.resolve().unwrap_err();
        assert!(
            trace
                .decisions
                .iter()
                .any(|decision| decision.contains("not-absolute"))
        );
        assert!(
            trace
                .decisions
                .iter()
                .any(|decision| decision.contains("not-regular-file"))
        );
        assert!(
            trace
                .decisions
                .iter()
                .all(|decision| !decision.contains(&root.to_string_lossy().to_string()))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
