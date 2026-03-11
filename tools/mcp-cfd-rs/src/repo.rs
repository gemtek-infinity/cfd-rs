use std::path::{Component, Path, PathBuf};

/// Resolve a repo-relative path string to a canonical filesystem path,
/// enforcing that the result stays within the repo root.
pub fn resolve(repo_root: &Path, repo_root_canon: &Path, path: &str) -> Result<PathBuf, &'static str> {
    if !is_repo_relative(path) {
        return Err("path must be repo-relative and must not escape the repo root");
    }

    let candidate = repo_root.join(path);
    let candidate_canon =
        std::fs::canonicalize(&candidate).map_err(|_| "file or directory not found or not readable")?;

    if !candidate_canon.starts_with(repo_root_canon) {
        return Err("path escapes repo root");
    }

    Ok(candidate_canon)
}

/// Produce the shortest repo-relative display string for a path under the repo
/// root.
pub fn make_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn is_repo_relative(path: &str) -> bool {
    let p = Path::new(path);

    !p.is_absolute()
        && !p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::is_repo_relative;

    #[test]
    fn rejects_absolute_and_parent_paths() {
        assert!(!is_repo_relative("/tmp/file"));
        assert!(!is_repo_relative("../file"));
        assert!(!is_repo_relative("docs/../../file"));
        assert!(is_repo_relative("docs/file.md"));
    }
}
