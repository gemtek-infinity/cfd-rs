use crate::fs::is_text_file;
use crate::repo;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Maximum file size (bytes) considered for text search indexing.
const MAX_SEARCHABLE_FILE_SIZE: u64 = 512_000;

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub path: String,
    pub score: usize,
    pub snippet: String,
}

/// Search text files under the given roots for query terms, returning scored
/// hits.
pub async fn search_roots(
    repo_root: &Path,
    roots: &[PathBuf],
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchHit>, &'static str> {
    let terms = normalize_terms(query);
    if terms.is_empty() {
        return Err("query must not be empty");
    }

    let mut files = BTreeSet::new();
    for root in roots {
        collect_text_files(root, &mut files).await;
    }

    let mut hits = Vec::new();

    for path in files {
        if let Ok(text) = fs::read_to_string(&path).await {
            let score = score_text(&text, &terms);
            if score == 0 {
                continue;
            }

            hits.push(SearchHit {
                path: repo::make_relative(repo_root, &path),
                score,
                snippet: make_snippet(&text, query, 320),
            });
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(max_results);
    Ok(hits)
}

fn is_searchable_file(path: &Path, size: u64) -> bool {
    size <= MAX_SEARCHABLE_FILE_SIZE && is_text_file(path)
}

async fn collect_text_files(path: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(path).await else {
        return;
    };

    if meta.file_type().is_symlink() {
        return;
    }

    if meta.is_file() {
        if is_searchable_file(path, meta.len()) {
            out.insert(path.to_path_buf());
        }
        return;
    }

    if meta.is_dir() {
        walk_dir(path, out).await;
    }
}

async fn walk_dir(start: &Path, out: &mut BTreeSet<PathBuf>) {
    let mut stack = vec![start.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let entry_path = entry.path();

            let Ok(entry_meta) = fs::symlink_metadata(&entry_path).await else {
                continue;
            };

            if entry_meta.file_type().is_symlink() {
                continue;
            }

            if entry_meta.is_dir() {
                stack.push(entry_path);
            } else if entry_meta.is_file() && is_searchable_file(&entry_path, entry_meta.len()) {
                out.insert(entry_path);
            }
        }
    }
}

fn normalize_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn score_text(text: &str, terms: &[String]) -> usize {
    let hay = text.to_lowercase();
    terms.iter().map(|t| hay.matches(t).count()).sum()
}

fn make_snippet(text: &str, query: &str, limit: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let needle = query.trim().to_lowercase();

    let Some(byte_idx) = flat.to_lowercase().find(&needle) else {
        return flat.chars().take(limit).collect();
    };

    let match_start = flat[..byte_idx].chars().count();
    let match_len = needle.chars().count();
    let total_chars = flat.chars().count();

    let window_start = match_start.saturating_sub(limit / 2);
    let window_end = usize::min(total_chars, match_start + match_len + (limit / 2));

    flat.chars()
        .skip(window_start)
        .take(window_end - window_start)
        .collect()
}
