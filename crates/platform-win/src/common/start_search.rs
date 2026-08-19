//! Bounded local providers for the owned SuperDesktop Start surface.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use shell_provider_protocol::{
    CommandDescriptor, CommandId, CommandRisk, SearchCategory, SearchResult,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchLimits {
    pub max_results: usize,
    pub max_depth: usize,
    pub max_visited: usize,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_results: 100,
            max_depth: 8,
            max_visited: 10_000,
        }
    }
}

pub fn discover_applications(roots: &[PathBuf], limit: usize) -> Vec<SearchResult> {
    search_applications("", roots, limit)
}

pub fn search_applications(query: &str, roots: &[PathBuf], limit: usize) -> Vec<SearchResult> {
    let extensions = ["lnk", "url", "appref-ms", "exe"];
    bounded_paths(
        query,
        roots,
        SearchLimits {
            max_results: limit,
            ..SearchLimits::default()
        },
        || true,
    )
    .into_iter()
    .filter(|path| {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| extensions.contains(&value.to_ascii_lowercase().as_str()))
    })
    .map(|path| result_for_path(path, SearchCategory::Application))
    .collect()
}

pub fn search_files(
    query: &str,
    roots: &[PathBuf],
    limits: SearchLimits,
    continue_search: impl FnMut() -> bool,
) -> Vec<SearchResult> {
    bounded_paths(query, roots, limits, continue_search)
        .into_iter()
        .map(|path| result_for_path(path, SearchCategory::File))
        .collect()
}

pub fn settings_catalog() -> Vec<SearchResult> {
    [
        (
            "settings:display",
            "Display settings",
            "ms-settings:display",
        ),
        (
            "settings:network",
            "Network & Internet",
            "ms-settings:network",
        ),
        (
            "settings:personalization",
            "Personalization",
            "ms-settings:personalization",
        ),
        ("settings:apps", "Apps", "ms-settings:appsfeatures"),
        ("settings:accounts", "Accounts", "ms-settings:yourinfo"),
        (
            "settings:time",
            "Time & language",
            "ms-settings:dateandtime",
        ),
        (
            "settings:accessibility",
            "Ease of Access",
            "ms-settings:easeofaccess",
        ),
        (
            "settings:update",
            "Update & Security",
            "ms-settings:windowsupdate",
        ),
    ]
    .into_iter()
    .map(|(id, title, uri)| SearchResult {
        id: id.into(),
        title: title.into(),
        subtitle: Some("Settings".into()),
        category: SearchCategory::Setting,
        score_milli: 0,
        activation: activation(format!("settings:{uri}")),
    })
    .collect()
}

pub fn default_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(app_data).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    roots
}

pub fn default_file_roots() -> Vec<PathBuf> {
    let Some(profile) = std::env::var_os("USERPROFILE") else {
        return Vec::new();
    };
    let profile = PathBuf::from(profile);
    ["Desktop", "Documents", "Downloads"]
        .into_iter()
        .map(|name| profile.join(name))
        .collect()
}

/// Rehydrates a persisted file result only while it still resolves beneath an
/// admitted local-search root. Missing, moved, or escaped items are dropped.
pub fn restore_persisted_result(id: &str) -> Option<SearchResult> {
    let path = PathBuf::from(id.strip_prefix("path:")?);
    let canonical = path.canonicalize().ok()?;
    if !canonical.is_file() || !is_path_within(&canonical, &default_file_roots()) {
        return None;
    }
    Some(result_for_path(canonical, SearchCategory::File))
}

fn bounded_paths(
    query: &str,
    roots: &[PathBuf],
    limits: SearchLimits,
    mut continue_search: impl FnMut() -> bool,
) -> Vec<PathBuf> {
    let query = query.to_lowercase();
    let mut queue = VecDeque::new();
    for root in roots {
        if let Ok(root) = root.canonicalize() {
            queue.push_back((root, 0usize));
        }
    }
    let mut output = Vec::new();
    let mut visited = 0usize;
    while let Some((directory, depth)) = queue.pop_front() {
        if !continue_search() || visited >= limits.max_visited || output.len() >= limits.max_results
        {
            break;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            visited = visited.saturating_add(1);
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if (query.is_empty() || name.contains(&query)) && path.is_file() {
                output.push(path.clone());
                if output.len() >= limits.max_results {
                    break;
                }
            }
            if depth < limits.max_depth && path.is_dir() {
                queue.push_back((path, depth + 1));
            }
            if visited >= limits.max_visited || !continue_search() {
                break;
            }
        }
    }
    output.sort_by(|left, right| {
        left.to_string_lossy()
            .to_lowercase()
            .cmp(&right.to_string_lossy().to_lowercase())
    });
    output
}

fn result_for_path(path: PathBuf, category: SearchCategory) -> SearchResult {
    let title = path
        .file_stem()
        .or_else(|| path.file_name())
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let stable = path.to_string_lossy().to_lowercase();
    SearchResult {
        id: format!("path:{stable}"),
        title,
        subtitle: path
            .parent()
            .map(|value| value.to_string_lossy().into_owned()),
        category,
        score_milli: 0,
        activation: activation(format!("open:{stable}")),
    }
}

fn activation(id: String) -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId(id),
        label: "Open".into(),
        enabled: true,
        risk: CommandRisk::Normal,
        children: Vec::new(),
    }
}

pub fn is_path_within(path: &Path, roots: &[PathBuf]) -> bool {
    path.canonicalize().is_ok_and(|path| {
        roots
            .iter()
            .any(|root| root.canonicalize().is_ok_and(|root| path.starts_with(root)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn providers_are_bounded_cancellable_and_stable() {
        let root = std::env::temp_dir().join(format!(
            "superdesktop-search-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("Alpha.lnk"), b"fixture").unwrap();
        fs::write(root.join("nested").join("Alpha.txt"), b"fixture").unwrap();
        assert_eq!(
            discover_applications(std::slice::from_ref(&root), 10).len(),
            1
        );
        let found = search_files(
            "alpha",
            std::slice::from_ref(&root),
            SearchLimits {
                max_results: 1,
                max_depth: 4,
                max_visited: 10,
            },
            || true,
        );
        assert_eq!(found.len(), 1);
        assert!(
            search_files(
                "alpha",
                std::slice::from_ref(&root),
                SearchLimits::default(),
                || false
            )
            .is_empty()
        );
        for index in 0..20 {
            fs::write(root.join(format!("A{index:02}.lnk")), b"fixture").unwrap();
        }
        fs::write(root.join("Snow Shot.lnk"), b"fixture").unwrap();
        let applications = search_applications("snow shot", std::slice::from_ref(&root), 1);
        assert_eq!(applications.len(), 1);
        assert_eq!(applications[0].title, "Snow Shot");
        assert!(is_path_within(
            &root.join("Alpha.lnk"),
            std::slice::from_ref(&root)
        ));
        assert!(!settings_catalog().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_ditto_and_snow_shot_shortcuts_are_found_when_present() {
        let roots = default_application_roots();
        for title in ["Ditto", "Snow Shot"] {
            let installed = roots.iter().any(|root| {
                bounded_paths(
                    &title.to_lowercase(),
                    std::slice::from_ref(root),
                    SearchLimits::default(),
                    || true,
                )
                .iter()
                .any(|path| path.extension().is_some_and(|extension| extension == "lnk"))
            });
            if installed {
                let results = search_applications(title, &roots, 10);
                assert!(
                    results
                        .iter()
                        .any(|result| result.title.eq_ignore_ascii_case(title)),
                    "installed {title} shortcut was not searchable"
                );
            }
        }
    }
}
