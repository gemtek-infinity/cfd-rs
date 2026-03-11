use crate::repo;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize)]
pub struct PathEntry {
    pub path: String,
    pub kind: &'static str,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub kind: &'static str,
    pub size_bytes: u64,
    pub line_count: Option<usize>,
}

pub async fn collect_paths(
    repo_root: &Path,
    base_path: &Path,
    recursive: bool,
    extensions: Option<&BTreeSet<String>>,
    max_results: usize,
) -> Vec<PathEntry> {
    let Ok(base_meta) = fs::symlink_metadata(base_path).await else {
        return Vec::new();
    };

    if !base_meta.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut stack = vec![base_path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        let mut children = Vec::new();
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let entry_path = entry.path();

            let Ok(entry_meta) = fs::symlink_metadata(&entry_path).await else {
                continue;
            };

            if entry_meta.file_type().is_symlink() {
                continue;
            }

            children.push((entry_path, entry_meta));
        }

        children.sort_by(|left, right| left.0.cmp(&right.0));

        for (entry_path, entry_meta) in children {
            if entry_meta.is_dir() {
                entries.push(PathEntry {
                    path: repo::make_relative(repo_root, &entry_path),
                    kind: "directory",
                    size_bytes: None,
                });

                if recursive {
                    stack.push(entry_path);
                }
            } else if entry_meta.is_file() {
                if !path_matches_extensions(&entry_path, extensions) {
                    continue;
                }

                entries.push(PathEntry {
                    path: repo::make_relative(repo_root, &entry_path),
                    kind: "file",
                    size_bytes: Some(entry_meta.len()),
                });
            }

            if entries.len() >= max_results {
                return entries;
            }
        }

        if !recursive {
            break;
        }
    }

    entries
}

pub fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("md" | "txt" | "rs" | "toml" | "yaml" | "yml" | "json" | "go" | "sh" | "py" | "sql")
    )
}

pub fn normalize_extensions(extensions: Option<&[String]>) -> Option<BTreeSet<String>> {
    let mut normalized = BTreeSet::new();

    for extension in extensions.unwrap_or(&[]) {
        let extension = extension.trim().trim_start_matches('.').to_ascii_lowercase();
        if !extension.is_empty() {
            normalized.insert(extension);
        }
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn slice_lines(
    text: &str,
    start_line: usize,
    end_line: usize,
    max_chars: usize,
) -> Result<(String, usize, bool), &'static str> {
    let all_lines = text.lines().collect::<Vec<_>>();
    let total_line_count = all_lines.len();

    if total_line_count == 0 {
        return Ok((String::new(), 0, false));
    }

    if start_line > total_line_count {
        return Err("start_line is out of bounds for this file");
    }

    let end_line = usize::min(end_line, total_line_count);
    let content = all_lines[start_line - 1..end_line].join("\n");
    let truncated = content.chars().count() > max_chars;
    let content = content.chars().take(max_chars).collect();

    Ok((content, total_line_count, truncated))
}

fn path_matches_extensions(path: &Path, extensions: Option<&BTreeSet<String>>) -> bool {
    let Some(extensions) = extensions else {
        return true;
    };

    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extensions.contains(&extension.to_ascii_lowercase()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{normalize_extensions, path_matches_extensions, slice_lines};
    use std::{collections::BTreeSet, path::Path};

    #[test]
    fn normalizes_extension_filters() {
        let normalized = normalize_extensions(Some(&[".MD".to_string(), "rs".to_string(), "  ".to_string()]))
            .expect("extensions should be present");

        assert_eq!(normalized, BTreeSet::from(["md".to_string(), "rs".to_string()]));
    }

    #[test]
    fn matches_extensions_case_insensitively() {
        let extensions = BTreeSet::from(["md".to_string()]);

        assert!(path_matches_extensions(
            Path::new("docs/README.MD"),
            Some(&extensions)
        ));
        assert!(!path_matches_extensions(
            Path::new("src/main.rs"),
            Some(&extensions)
        ));
    }

    #[test]
    fn slices_requested_line_range() {
        let text = "one\ntwo\nthree\nfour\n";
        let (content, total_line_count, truncated) =
            slice_lines(text, 2, 3, 100).expect("line slice should succeed");

        assert_eq!(content, "two\nthree");
        assert_eq!(total_line_count, 4);
        assert!(!truncated);
    }

    #[test]
    fn rejects_out_of_bounds_line_start() {
        let error = slice_lines("one\ntwo\n", 4, 4, 100).expect_err("slice should fail");

        assert_eq!(error, "start_line is out of bounds for this file");
    }
}
