use crate::{fs as mcp_fs, repo};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Maximum file size considered for Debtmap analysis.
const MAX_ANALYZABLE_SIZE: u64 = 512_000;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct FileScore {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: u32,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: u32,
    pub top_todos: Vec<TodoEntry>,
    pub long_fn_lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct TodoEntry {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TouchedFilesReview {
    pub files: Vec<FileScore>,
    pub total_score: u32,
    pub skipped: Vec<SkippedFile>,
}

#[derive(Debug, Serialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: &'static str,
}

// ---------------------------------------------------------------------------
// Top hotspots
// ---------------------------------------------------------------------------

/// Collect and rank the top hotspot files under `scope`, or under the repo
/// root when `scope` is `None`.
pub async fn top_hotspots(repo_root: &Path, scope: Option<&Path>, limit: usize) -> Vec<FileScore> {
    let base = scope.unwrap_or(repo_root);
    let mut files = BTreeSet::new();
    collect_analyzable_files(base, &mut files).await;

    let mut scores = Vec::new();
    for path in &files {
        if let Some(score) = score_file(repo_root, path).await {
            scores.push(score);
        }
    }

    scores.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    scores.truncate(limit);
    scores
}

// ---------------------------------------------------------------------------
// Single-file summary
// ---------------------------------------------------------------------------

/// Return a detailed summary for one file, including individual TODO
/// locations and lines where long functions start.
pub async fn file_summary(repo_root: &Path, file_path: &Path) -> Result<FileSummary, &'static str> {
    let text = read_analyzable(file_path).await?;

    let metrics = analyze_text(&text);
    let rel = repo::make_relative(repo_root, file_path);

    let top_todos = collect_todos(&text, 10);
    let long_fn_lines = collect_long_fns(&text, 60);

    Ok(FileSummary {
        path: rel,
        line_count: metrics.line_count,
        fn_count: metrics.fn_count,
        todo_count: metrics.todo_count,
        max_indent_depth: metrics.max_indent_depth,
        score: compute_score(&metrics),
        top_todos,
        long_fn_lines,
    })
}

// ---------------------------------------------------------------------------
// Touched-files review
// ---------------------------------------------------------------------------

/// Score a provided set of files for a bounded cognitive-load review.
pub async fn touched_files_review(repo_root: &Path, paths: &[PathBuf]) -> TouchedFilesReview {
    let mut files = Vec::new();
    let mut skipped = Vec::new();

    for path in paths {
        let rel = repo::make_relative(repo_root, path);

        if !path.is_file() {
            skipped.push(SkippedFile {
                path: rel,
                reason: "not a regular file",
            });
            continue;
        }

        if !mcp_fs::is_text_file(path) {
            skipped.push(SkippedFile {
                path: rel,
                reason: "not a recognized text file",
            });
            continue;
        }

        match score_file(repo_root, path).await {
            Some(score) => files.push(score),
            None => skipped.push(SkippedFile {
                path: rel,
                reason: "file not readable or too large",
            }),
        }
    }

    files.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));

    let total_score = files.iter().map(|f| f.score).sum();

    TouchedFilesReview {
        files,
        total_score,
        skipped,
    }
}

// ---------------------------------------------------------------------------
// Metrics engine
// ---------------------------------------------------------------------------

struct TextMetrics {
    line_count: usize,
    fn_count: usize,
    todo_count: usize,
    max_indent_depth: usize,
}

fn analyze_text(text: &str) -> TextMetrics {
    let mut line_count = 0;
    let mut fn_count = 0;
    let mut todo_count = 0;
    let mut max_indent_depth: usize = 0;

    for line in text.lines() {
        line_count += 1;

        let trimmed = line.trim_start();

        // Indent depth: count leading whitespace units (4-space or 1-tab).
        let leading_spaces = line.len() - trimmed.len();
        let depth = leading_spaces / 4;
        max_indent_depth = max_indent_depth.max(depth);

        // Function definitions — works for Rust `fn`, Go `func`, Python `def`.
        if is_fn_definition(trimmed) {
            fn_count += 1;
        }

        // TODO/FIXME markers.
        let upper = trimmed.to_uppercase();
        if upper.contains("TODO") || upper.contains("FIXME") {
            todo_count += 1;
        }
    }

    TextMetrics {
        line_count,
        fn_count,
        todo_count,
        max_indent_depth,
    }
}

fn compute_score(m: &TextMetrics) -> u32 {
    // Weighted heuristic — higher means more cognitive load.
    let line_weight = (m.line_count as u32).min(2000);
    let fn_weight = (m.fn_count as u32).saturating_mul(10);
    let todo_weight = (m.todo_count as u32).saturating_mul(15);
    let depth_weight = (m.max_indent_depth as u32).saturating_mul(8);

    line_weight
        .saturating_add(fn_weight)
        .saturating_add(todo_weight)
        .saturating_add(depth_weight)
}

// ---------------------------------------------------------------------------
// Shared predicates
// ---------------------------------------------------------------------------

/// Detect function definitions across Rust (`fn`), Go (`func`), and Python
/// (`def`).  Operates on a left-trimmed line.
fn is_fn_definition(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("pub(crate) fn ")
        || trimmed.starts_with("pub(crate) async fn ")
        || trimmed.starts_with("func ")
        || trimmed.starts_with("def ")
}

// ---------------------------------------------------------------------------
// Detail collectors
// ---------------------------------------------------------------------------

fn collect_todos(text: &str, limit: usize) -> Vec<TodoEntry> {
    let mut todos = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        let upper = line.to_uppercase();
        if upper.contains("TODO") || upper.contains("FIXME") {
            todos.push(TodoEntry {
                line: idx + 1,
                text: line.trim().to_string(),
            });
            if todos.len() >= limit {
                break;
            }
        }
    }

    todos
}

fn collect_long_fns(text: &str, threshold: usize) -> Vec<usize> {
    let mut long_starts = Vec::new();
    let mut fn_start: Option<usize> = None;
    let mut brace_depth: i32 = 0;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();

        if is_fn_definition(trimmed) && fn_start.is_none() {
            fn_start = Some(idx + 1);
            brace_depth = 0;
        }

        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        if let Some(start) = fn_start
            && brace_depth <= 0
            && idx + 1 > start
        {
            let fn_len = idx + 1 - start + 1;
            if fn_len >= threshold {
                long_starts.push(start);
            }
            fn_start = None;
            brace_depth = 0;
        }
    }

    long_starts
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

async fn score_file(repo_root: &Path, path: &Path) -> Option<FileScore> {
    let text = read_analyzable(path).await.ok()?;
    let metrics = analyze_text(&text);
    let score = compute_score(&metrics);

    Some(FileScore {
        path: repo::make_relative(repo_root, path),
        line_count: metrics.line_count,
        fn_count: metrics.fn_count,
        todo_count: metrics.todo_count,
        max_indent_depth: metrics.max_indent_depth,
        score,
    })
}

async fn read_analyzable(path: &Path) -> Result<String, &'static str> {
    let meta = fs::symlink_metadata(path)
        .await
        .map_err(|_| "file not found or not readable")?;

    if !meta.is_file() {
        return Err("path is not a regular file");
    }

    if meta.len() > MAX_ANALYZABLE_SIZE {
        return Err("file too large for Debtmap analysis");
    }

    if !mcp_fs::is_text_file(path) {
        return Err("not a recognized text file type");
    }

    fs::read_to_string(path)
        .await
        .map_err(|_| "file not readable as UTF-8 text")
}

async fn collect_analyzable_files(base: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(base).await else {
        return;
    };

    if meta.file_type().is_symlink() {
        return;
    }

    if meta.is_file() {
        if meta.len() <= MAX_ANALYZABLE_SIZE && mcp_fs::is_text_file(base) {
            out.insert(base.to_path_buf());
        }
        return;
    }

    if meta.is_dir() {
        walk_dir_for_analysis(base, out).await;
    }
}

async fn walk_dir_for_analysis(start: &Path, out: &mut BTreeSet<PathBuf>) {
    let mut stack = vec![start.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();

            let Ok(meta) = fs::symlink_metadata(&path).await else {
                continue;
            };

            if meta.file_type().is_symlink() {
                continue;
            }

            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() && meta.len() <= MAX_ANALYZABLE_SIZE && mcp_fs::is_text_file(&path) {
                out.insert(path);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{analyze_text, collect_long_fns, collect_todos, compute_score};

    #[test]
    fn scores_simple_text() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let metrics = analyze_text(text);

        assert_eq!(metrics.line_count, 3);
        assert_eq!(metrics.fn_count, 1);
        assert_eq!(metrics.todo_count, 0);
        assert!(metrics.max_indent_depth >= 1);
        assert!(compute_score(&metrics) > 0);
    }

    #[test]
    fn counts_todos_and_fixmes() {
        let text = "// TODO: fix this\nlet x = 1;\n// FIXME: broken\n";
        let metrics = analyze_text(text);

        assert_eq!(metrics.todo_count, 2);
    }

    #[test]
    fn counts_various_fn_forms() {
        let text = "\
fn foo() {}
pub fn bar() {}
async fn baz() {}
pub async fn qux() {}
pub(crate) fn internal() {}
pub(crate) async fn internal_async() {}
func goFunc() {}
def pyFunc():
";
        let metrics = analyze_text(text);

        assert_eq!(metrics.fn_count, 8);
    }

    #[test]
    fn collects_todo_entries_with_line_numbers() {
        let text = "line one\n// TODO: first\nline three\n// FIXME: second\n";
        let todos = collect_todos(text, 10);

        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].line, 2);
        assert_eq!(todos[1].line, 4);
    }

    #[test]
    fn detects_long_functions() {
        // 65-line function body should exceed the 60-line threshold.
        let mut lines = vec!["fn long_fn() {".to_string()];
        for i in 0..63 {
            lines.push(format!("    let x{} = {};", i, i));
        }
        lines.push("}".to_string());
        let text = lines.join("\n");

        let long = collect_long_fns(&text, 60);

        assert_eq!(long.len(), 1);
        assert_eq!(long[0], 1);
    }

    #[test]
    fn score_increases_with_complexity() {
        let simple = "fn a() {}\n";
        let complex = "fn a() {\n    // TODO: fix\n    if true {\n        if true {\n            let x = \
                       1;\n        }\n    }\n}\nfn b() {}\nfn c() {}\n";

        let simple_score = compute_score(&analyze_text(simple));
        let complex_score = compute_score(&analyze_text(complex));

        assert!(complex_score > simple_score);
    }

    #[test]
    fn empty_text_scores_zero() {
        let metrics = analyze_text("");

        assert_eq!(metrics.line_count, 0);
        assert_eq!(compute_score(&metrics), 0);
    }
}
