use crate::{fs as mcp_fs, repo};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Maximum file size considered for Debtmap analysis.
const MAX_ANALYZABLE_SIZE: u64 = 512_000;

/// Round to two decimal places for stable JSON output.
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

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
    pub score: f64,
    /// `"ast"` when the `debtmap` crate parsed the file, `"heuristic"`
    /// otherwise.
    pub analysis_method: &'static str,
    /// Average cyclomatic complexity across functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_cyclomatic: Option<f64>,
    /// Maximum cyclomatic complexity among functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cyclomatic: Option<u32>,
    /// Average cognitive complexity across functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_cognitive: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: f64,
    pub analysis_method: &'static str,
    pub top_todos: Vec<TodoEntry>,
    pub long_fn_lines: Vec<usize>,
    /// Per-function complexity breakdown (AST only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FunctionEntry>,
    /// Code smells detected by the `debtmap` crate (AST only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub code_smells: Vec<CodeSmellEntry>,
}

#[derive(Debug, Serialize)]
pub struct TodoEntry {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TouchedFilesReview {
    pub files: Vec<FileScore>,
    pub total_score: f64,
    pub skipped: Vec<SkippedFile>,
}

#[derive(Debug, Serialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeSmellEntry {
    pub line: usize,
    pub debt_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionEntry {
    pub name: String,
    pub line: usize,
    pub length: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
}

#[derive(Debug, Serialize)]
pub struct FunctionComplexityReport {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub functions: Vec<FunctionEntry>,
    pub analysis_method: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CodeSmellReport {
    pub path: String,
    pub smells: Vec<CodeSmellEntry>,
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

fn detect_crate_language(path: &Path) -> Option<debtmap::Language> {
    let lang = debtmap::Language::from_path(path);
    match lang {
        debtmap::Language::Rust | debtmap::Language::JavaScript | debtmap::Language::TypeScript => Some(lang),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Crate-based analysis (AST-level)
// ---------------------------------------------------------------------------

struct CrateAnalysis {
    line_count: usize,
    fn_count: usize,
    todo_count: usize,
    /// Aggregate debt score computed by `debtmap::total_debt_score`.
    crate_debt_score: u32,
    max_nesting: u32,
    total_cyclomatic: u32,
    max_cyclomatic: u32,
    total_cognitive: u32,
    functions: Vec<FunctionEntry>,
    code_smells: Vec<CodeSmellEntry>,
}

/// Analyze a file using the `debtmap` crate's AST analysis when the
/// language is supported, returning `None` for unsupported languages.
fn analyze_with_crate(content: &str, path: &Path) -> Option<CrateAnalysis> {
    let lang = detect_crate_language(path)?;
    let analyzer = debtmap::get_analyzer(lang);

    if analyzer.language() == debtmap::Language::Unknown {
        return None;
    }

    let metrics = debtmap::analyze_file(content.to_string(), path.to_path_buf(), analyzer.as_ref()).ok()?;

    let functions: Vec<FunctionEntry> = metrics
        .complexity
        .functions
        .iter()
        .map(|f| FunctionEntry {
            name: f.name.clone(),
            line: f.line,
            length: f.length,
            cyclomatic: f.cyclomatic,
            cognitive: f.cognitive,
            nesting: f.nesting,
        })
        .collect();

    let fn_count = functions.len();
    let total_cyclomatic = metrics.complexity.cyclomatic_complexity;
    let max_cyclomatic = functions.iter().map(|f| f.cyclomatic).max().unwrap_or(0);
    let total_cognitive = metrics.complexity.cognitive_complexity;
    let max_nesting = functions.iter().map(|f| f.nesting).max().unwrap_or(0);
    let crate_debt_score = debtmap::debt::total_debt_score(&metrics.debt_items);

    let todo_items = debtmap::find_todos_and_fixmes(content, path);
    let todo_count = todo_items.len();

    let raw_smells = debtmap::find_code_smells(content, path);
    let code_smells: Vec<CodeSmellEntry> = raw_smells
        .iter()
        .map(|s| CodeSmellEntry {
            line: s.line,
            debt_type: format!("{:?}", s.debt_type),
            description: s.message.clone(),
            severity: format!("{}", s.priority),
        })
        .collect();

    let line_count = if metrics.total_lines > 0 {
        metrics.total_lines
    } else {
        content.lines().count()
    };

    Some(CrateAnalysis {
        line_count,
        fn_count,
        todo_count,
        crate_debt_score,
        max_nesting,
        total_cyclomatic,
        max_cyclomatic,
        total_cognitive,
        functions,
        code_smells,
    })
}

/// Normalized composite score for crate-analyzed files.
///
/// Components are scaled so that a 300-line file with moderate complexity
/// scores roughly 15–40, matching the debtmap crate's scoring magnitude.
/// The debt component uses `debtmap::total_debt_score` directly.
fn compute_score_crate(a: &CrateAnalysis) -> f64 {
    let size = (a.line_count as f64 / 100.0).min(20.0);
    let complexity = a.total_cyclomatic as f64 * 0.5 + a.total_cognitive as f64 * 0.3;
    let nesting = a.max_nesting as f64 * 2.0;
    let todos = a.todo_count as f64 * 1.5;
    let debt = a.crate_debt_score as f64;

    size + complexity + nesting + todos + debt
}

fn file_score_from_crate(path: String, a: &CrateAnalysis) -> FileScore {
    let score = compute_score_crate(a);
    let avg_cyc = if a.fn_count > 0 {
        Some(a.total_cyclomatic as f64 / a.fn_count as f64)
    } else {
        None
    };
    let avg_cog = if a.fn_count > 0 {
        Some(a.total_cognitive as f64 / a.fn_count as f64)
    } else {
        None
    };

    FileScore {
        path,
        line_count: a.line_count,
        fn_count: a.fn_count,
        todo_count: a.todo_count,
        max_indent_depth: a.max_nesting as usize,
        score: round2(score),
        analysis_method: "ast",
        avg_cyclomatic: avg_cyc,
        max_cyclomatic: Some(a.max_cyclomatic),
        avg_cognitive: avg_cog,
    }
}

fn file_score_from_manual(path: String, m: &ManualMetrics) -> FileScore {
    FileScore {
        path,
        line_count: m.line_count,
        fn_count: m.fn_count,
        todo_count: m.todo_count,
        max_indent_depth: m.max_indent_depth,
        score: round2(compute_score_manual(m)),
        analysis_method: "heuristic",
        avg_cyclomatic: None,
        max_cyclomatic: None,
        avg_cognitive: None,
    }
}

/// Unified scoring — tries crate first, falls back to manual.
fn score_text(repo_root: &Path, path: &Path, text: &str) -> FileScore {
    let rel = repo::make_relative(repo_root, path);

    if let Some(crate_analysis) = analyze_with_crate(text, path) {
        return file_score_from_crate(rel, &crate_analysis);
    }

    let manual = analyze_manual(text);
    file_score_from_manual(rel, &manual)
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

    let repo_root = repo_root.to_path_buf();
    let file_list: Vec<PathBuf> = files.into_iter().collect();

    let scores = tokio::task::spawn_blocking(move || {
        let mut scores = Vec::new();
        for path in &file_list {
            if let Ok(text) = std::fs::read_to_string(path)
                && text.len() as u64 <= MAX_ANALYZABLE_SIZE
            {
                scores.push(score_text(&repo_root, path, &text));
            }
        }
        scores
    })
    .await
    .unwrap_or_default();

    let mut scores = scores;
    scores.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
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
    let rel = repo::make_relative(repo_root, file_path);

    let (analysis_method, line_count, fn_count, todo_count, max_indent_depth, score, functions, code_smells) =
        if let Some(ca) = analyze_with_crate(&text, file_path) {
            let s = round2(compute_score_crate(&ca));
            (
                "ast",
                ca.line_count,
                ca.fn_count,
                ca.todo_count,
                ca.max_nesting as usize,
                s,
                ca.functions,
                ca.code_smells,
            )
        } else {
            let m = analyze_manual(&text);
            let s = round2(compute_score_manual(&m));
            (
                "heuristic",
                m.line_count,
                m.fn_count,
                m.todo_count,
                m.max_indent_depth,
                s,
                Vec::new(),
                Vec::new(),
            )
        };

    let top_todos = collect_todos(&text, 10);

    // For AST-analyzed files, derive long functions from function entries.
    // For manual analysis, use brace-counting heuristic.
    let long_fn_lines = if analysis_method == "ast" {
        functions
            .iter()
            .filter(|f| f.length >= 60)
            .map(|f| f.line)
            .collect()
    } else {
        collect_long_fns(&text, 60)
    };

    Ok(FileSummary {
        path: rel,
        line_count,
        fn_count,
        todo_count,
        max_indent_depth,
        score,
        analysis_method,
        top_todos,
        long_fn_lines,
        functions,
        code_smells,
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

        match read_analyzable(path).await {
            Ok(text) => {
                let score = score_text(repo_root, path, &text);
                files.push(score);
            }
            Err(reason) => {
                skipped.push(SkippedFile { path: rel, reason });
            }
        }
    }

    files.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    let total_score = round2(files.iter().map(|f| f.score).sum());

    TouchedFilesReview {
        files,
        total_score,
        skipped,
    }
}

// ---------------------------------------------------------------------------
// Code smells (crate-powered)
// ---------------------------------------------------------------------------

/// Detect code smells in a single file using the `debtmap` crate.
pub async fn code_smells(repo_root: &Path, file_path: &Path) -> Result<CodeSmellReport, &'static str> {
    let text = read_analyzable(file_path).await?;
    let rel = repo::make_relative(repo_root, file_path);

    let raw_smells = debtmap::find_code_smells(&text, file_path);
    let smells: Vec<CodeSmellEntry> = raw_smells
        .iter()
        .map(|s| CodeSmellEntry {
            line: s.line,
            debt_type: format!("{:?}", s.debt_type),
            description: s.message.clone(),
            severity: format!("{}", s.priority),
        })
        .collect();

    let total = smells.len();
    Ok(CodeSmellReport {
        path: rel,
        smells,
        total,
    })
}

// ---------------------------------------------------------------------------
// Function complexity (crate-powered + fallback)
// ---------------------------------------------------------------------------

/// Return per-function complexity breakdown for one file. Uses AST analysis
/// for supported languages, manual heuristic otherwise (with an empty
/// `functions` list in the latter case).
pub async fn function_complexity(
    repo_root: &Path,
    file_path: &Path,
) -> Result<FunctionComplexityReport, &'static str> {
    let text = read_analyzable(file_path).await?;
    let rel = repo::make_relative(repo_root, file_path);

    if let Some(ca) = analyze_with_crate(&text, file_path) {
        return Ok(FunctionComplexityReport {
            path: rel,
            line_count: ca.line_count,
            fn_count: ca.fn_count,
            functions: ca.functions,
            analysis_method: "ast",
        });
    }

    let m = analyze_manual(&text);
    Ok(FunctionComplexityReport {
        path: rel,
        line_count: m.line_count,
        fn_count: m.fn_count,
        functions: Vec::new(),
        analysis_method: "heuristic",
    })
}

// ---------------------------------------------------------------------------
// Manual fallback analysis (Go, Python, generic text)
// ---------------------------------------------------------------------------

struct ManualMetrics {
    line_count: usize,
    fn_count: usize,
    todo_count: usize,
    max_indent_depth: usize,
}

fn analyze_manual(text: &str) -> ManualMetrics {
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
        if contains_todo_marker(trimmed) {
            todo_count += 1;
        }
    }

    ManualMetrics {
        line_count,
        fn_count,
        todo_count,
        max_indent_depth,
    }
}

/// Normalized heuristic score for files without AST analysis.
///
/// Uses the same magnitude as `compute_score_crate` so that files
/// analyzed via different paths produce comparable scores.
fn compute_score_manual(m: &ManualMetrics) -> f64 {
    let size = (m.line_count as f64 / 100.0).min(20.0);
    let fns = m.fn_count as f64;
    let todos = m.todo_count as f64 * 1.5;
    let depth = m.max_indent_depth as f64 * 2.0;

    size + fns + todos + depth
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
        if contains_todo_marker(line) {
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
        let line_number = idx + 1;
        let trimmed = line.trim_start();

        if is_fn_definition(trimmed) && fn_start.is_none() {
            fn_start = Some(line_number);
            brace_depth = 0;
        }

        brace_depth += brace_delta(line);

        if let Some(start) = fn_start
            && brace_depth <= 0
            && line_number > start
        {
            let fn_len = line_number - start + 1;
            if fn_len >= threshold {
                long_starts.push(start);
            }
            fn_start = None;
            brace_depth = 0;
        }
    }

    long_starts
}

fn contains_todo_marker(line: &str) -> bool {
    let upper = line.to_uppercase();
    upper.contains("TODO") || upper.contains("FIXME")
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, ch| match ch {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

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
        if is_analyzable_file(base, &meta) {
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
            } else if is_analyzable_file(&path, &meta) {
                out.insert(path);
            }
        }
    }
}

fn is_analyzable_file(path: &Path, meta: &std::fs::Metadata) -> bool {
    meta.is_file() && meta.len() <= MAX_ANALYZABLE_SIZE && mcp_fs::is_text_file(path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_analyzes_rust_code() {
        let text = "fn main() {\n    if true {\n        println!(\"hello\");\n    }\n}\n";
        let path = Path::new("test.rs");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some(), "Rust should be analyzed by the crate");
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 1);
        assert!(a.line_count >= 5);
        assert!(a.total_cyclomatic > 0);
    }

    #[test]
    fn manual_fallback_for_go() {
        let text = "func main() {\n    fmt.Println(\"hello\")\n}\n";
        let path = Path::new("main.go");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_none(), "Go should fall back to manual");

        let m = analyze_manual(text);
        assert_eq!(m.fn_count, 1);
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn scores_simple_text() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let metrics = analyze_manual(text);

        assert_eq!(metrics.line_count, 3);
        assert_eq!(metrics.fn_count, 1);
        assert_eq!(metrics.todo_count, 0);
        assert!(metrics.max_indent_depth >= 1);
        assert!(compute_score_manual(&metrics) > 0.0);
    }

    #[test]
    fn counts_todos_and_fixmes() {
        let text = "// TODO: fix this\nlet x = 1;\n// FIXME: broken\n";
        let metrics = analyze_manual(text);

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
        let metrics = analyze_manual(text);

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
            lines.push(format!("    let x{i} = {i};"));
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

        let simple_score = compute_score_manual(&analyze_manual(simple));
        let complex_score = compute_score_manual(&analyze_manual(complex));

        assert!(complex_score > simple_score);
    }

    #[test]
    fn empty_text_scores_zero() {
        let metrics = analyze_manual("");

        assert_eq!(metrics.line_count, 0);
        assert_eq!(compute_score_manual(&metrics), 0.0);
    }

    #[test]
    fn crate_detects_code_smells_in_rust() {
        let text = "fn main() {\n    // TODO: fix this later\n    let x = 42;\n}\n";
        let path = Path::new("test.rs");
        let smells = debtmap::find_code_smells(text, path);

        // The crate may or may not treat a single TODO as a code smell;
        // the important thing is that the call succeeds without panicking.
        let _ = smells;
    }

    #[test]
    fn crate_finds_todos_and_fixmes() {
        let text = "fn main() {\n    // TODO: first\n    // FIXME: second\n}\n";
        let path = Path::new("test.rs");
        let todos = debtmap::find_todos_and_fixmes(text, path);

        assert!(todos.len() >= 2);
    }

    #[test]
    fn crate_reports_function_complexity() {
        let text = r#"
fn simple() {
    let x = 1;
}

fn complex(a: i32, b: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            match a + b {
                0 => 0,
                1 => 1,
                _ => a + b,
            }
        } else {
            -1
        }
    } else {
        0
    }
}
"#;
        let path = Path::new("test.rs");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some());
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 2);

        let complex_fn = a.functions.iter().find(|f| f.name == "complex");
        assert!(complex_fn.is_some());
        let cf = complex_fn.expect("complex function should be found");
        assert!(cf.cyclomatic > 1, "complex fn should have cyclomatic > 1");
    }

    #[test]
    fn crate_analyzes_javascript_code() {
        let text = "function sum(a, b) {\n  if (a > 0) {\n    return a + b;\n  }\n  return b;\n}\n";
        let path = Path::new("test.js");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some(), "JavaScript should be analyzed by the crate");
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 1);
        assert!(a.total_cyclomatic > 0);
    }

    #[test]
    fn python_uses_manual_fallback_until_supported() {
        let text = "def add(a, b):\n    return a + b\n";
        let path = Path::new("test.py");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_none(), "Python should use manual fallback for now");

        let score = score_text(
            Path::new("/tmp/test_repo"),
            Path::new("/tmp/test_repo/test.py"),
            text,
        );
        assert_eq!(score.analysis_method, "heuristic");
    }

    #[test]
    fn unified_scoring_uses_ast_for_rust() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let root = Path::new("/tmp/test_repo");
        let path = Path::new("/tmp/test_repo/src/main.rs");
        let score = score_text(root, path, text);

        assert_eq!(score.analysis_method, "ast");
        assert!(score.avg_cyclomatic.is_some());
    }

    #[test]
    fn unified_scoring_uses_heuristic_for_go() {
        let text = "func main() {\n    fmt.Println(\"hello\")\n}\n";
        let root = Path::new("/tmp/test_repo");
        let path = Path::new("/tmp/test_repo/main.go");
        let score = score_text(root, path, text);

        assert_eq!(score.analysis_method, "heuristic");
        assert!(score.avg_cyclomatic.is_none());
    }
}
