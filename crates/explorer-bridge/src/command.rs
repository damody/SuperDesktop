use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::ResolvedExecutable;

pub const INITIAL_PATH_ENV: &str = "EXPLORER_INITIAL_PATH";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchSpec {
    pub application: PathBuf,
    pub executable_identity: String,
    pub child_environment: BTreeMap<OsString, OsString>,
    pub display_name: &'static str,
    pub claims_this_pc: bool,
}

pub fn build_default_launch(executable: &ResolvedExecutable) -> LaunchSpec {
    LaunchSpec {
        application: executable.path.clone(),
        executable_identity: executable.identity.clone(),
        child_environment: BTreeMap::new(),
        display_name: "SuperExplorer",
        claims_this_pc: false,
    }
}

pub fn build_folder_launch(
    executable: &ResolvedExecutable,
    directory: &Path,
) -> Result<LaunchSpec, &'static str> {
    if !directory.is_absolute() {
        return Err("invalid-initial-directory");
    }
    let metadata = fs::symlink_metadata(directory).map_err(|_| "invalid-initial-directory")?;
    if !metadata.is_dir()
        || std::os::windows::fs::MetadataExt::file_attributes(&metadata) & 0x400 != 0
    {
        return Err("invalid-initial-directory");
    }
    let canonical = directory
        .canonicalize()
        .map_err(|_| "invalid-initial-directory")?;
    let mut child_environment = BTreeMap::new();
    child_environment.insert(OsString::from(INITIAL_PATH_ENV), canonical.into_os_string());
    Ok(LaunchSpec {
        application: executable.path.clone(),
        executable_identity: executable.identity.clone(),
        child_environment,
        display_name: "SuperExplorer",
        claims_this_pc: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutableCandidate;
    fn executable() -> ResolvedExecutable {
        ResolvedExecutable {
            path: PathBuf::from(r"C:\Program Files\SuperExplorer.exe"),
            candidate: ExecutableCandidate::Setting,
            identity: "id".into(),
        }
    }
    #[test]
    fn default_is_truthful_and_has_no_initial_path() {
        let spec = build_default_launch(&executable());
        assert!(spec.child_environment.is_empty());
        assert_eq!(spec.display_name, "SuperExplorer");
        assert!(!spec.claims_this_pc)
    }
    #[test]
    fn unicode_space_and_special_folder_round_trip_without_parent_mutation() {
        let root = std::env::temp_dir().join(format!("橋 接 folder & [] {}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let before = std::env::var_os(INITIAL_PATH_ENV);
        let spec = build_folder_launch(&executable(), &root).unwrap();
        assert_eq!(
            spec.child_environment
                .get(&OsString::from(INITIAL_PATH_ENV)),
            Some(&root.canonicalize().unwrap().into_os_string())
        );
        assert_eq!(std::env::var_os(INITIAL_PATH_ENV), before);
        fs::remove_dir_all(root).unwrap()
    }
    #[test]
    fn relative_missing_and_regular_file_are_rejected_before_spawn() {
        let root = std::env::temp_dir().join(format!("bridge-invalid-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = root.join("file.txt");
        fs::write(&file, b"x").unwrap();
        for path in [PathBuf::from("relative"), root.join("missing"), file] {
            assert_eq!(
                build_folder_launch(&executable(), &path),
                Err("invalid-initial-directory")
            )
        }
        fs::remove_dir_all(root).unwrap()
    }
}
