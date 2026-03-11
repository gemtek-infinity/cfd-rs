use std::path::Path;

use super::scoring::{
    build_function_entry, categorize_file_score, recommended_action_for_file_score, round2,
};
use super::types::{CodeSmellEntry, FileScore, FunctionEntry};

/// Maximum file size considered for Debtmap analysis.
pub const MAX_ANALYZABLE_SIZE: u64 = 512_000;

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

/// Returns `true` for debt types that represent rewrite-phase bookkeeping
/// markers (TODO, FIXME, TestTodo) rather than actual code complexity.
pub fn is_marker_debt(dt: &debtmap::DebtType) -> bool {
    matches!(
        dt,
        debtmap::DebtType::Todo { .. } | debtmap::DebtType::Fixme { .. } | debtmap::DebtType::TestTodo { .. }
    )
}

// ---------------------------------------------------------------------------
// Crate-based analysis (AST-level)
// ---------------------------------------------------------------------------

pub struct CrateAnalysis {
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    /// Complexity-only debt score (excludes Todo/Fixme/TestTodo markers).
    pub complexity_debt_score: u32,
    pub max_nesting: u32,
    pub total_cyclomatic: u32,
    pub max_cyclomatic: u32,
    pub total_cognitive: u32,
    pub functions: Vec<FunctionEntry>,
    pub code_smells: Vec<CodeSmellEntry>,
}

/// Analyze a file using the `debtmap` crate's AST analysis when the
/// language is supported, returning `None` for unsupported languages.
pub fn analyze_with_crate(content: &str, path: &Path) -> Option<CrateAnalysis> {
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
        .map(|f| {
            build_function_entry(
                f.name.clone(),
                f.line,
                f.length,
                f.cyclomatic,
                f.cognitive,
                f.nesting,
            )
        })
        .collect();

    let fn_count = functions.len();
    let total_cyclomatic = metrics.complexity.cyclomatic_complexity;
    let max_cyclomatic = functions.iter().map(|f| f.cyclomatic).max().unwrap_or(0);
    let total_cognitive = metrics.complexity.cognitive_complexity;
    let max_nesting = functions.iter().map(|f| f.nesting).max().unwrap_or(0);

    // Separate marker-debt (Todo/Fixme/TestTodo) from complexity-debt.
    let complexity_debt_score: u32 = metrics
        .debt_items
        .iter()
        .filter(|item| !is_marker_debt(&item.debt_type))
        .map(|item| debtmap::debt::calculate_debt_score(item))
        .sum();

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
        complexity_debt_score,
        max_nesting,
        total_cyclomatic,
        max_cyclomatic,
        total_cognitive,
        functions,
        code_smells,
    })
}

// ---------------------------------------------------------------------------
// Crate score
// ---------------------------------------------------------------------------

/// Normalized composite score for crate-analyzed files.
///
/// Components are scaled so that a 300-line file with moderate complexity
/// scores roughly 15–40, matching the documented file-level categories in
/// `docs/ai-context-routing.md`.
///
/// Only complexity-relevant debt contributes to the score.  Marker-debt
/// (Todo, Fixme, TestTodo) is expected during rewrite phases and excluded
/// so that rewrite bookkeeping does not inflate files into hotspot
/// territory.  `sqrt` compresses the remaining debt outliers while
/// preserving relative ordering.
pub fn compute_score_crate(a: &CrateAnalysis) -> f64 {
    let size = (a.line_count as f64 / 100.0).min(20.0);
    let complexity = a.total_cyclomatic as f64 * 0.5 + a.total_cognitive as f64 * 0.3;
    let nesting = a.max_nesting as f64 * 2.0;
    let debt = (a.complexity_debt_score as f64).sqrt();

    size + complexity + nesting + debt
}

pub fn file_score_from_crate(path: String, a: &CrateAnalysis) -> FileScore {
    let score = compute_score_crate(a);
    let rounded_score = round2(score);

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
        score: rounded_score,
        score_category: categorize_file_score(rounded_score),
        recommended_action: recommended_action_for_file_score(rounded_score),
        analysis_method: "ast",
        avg_cyclomatic: avg_cyc,
        max_cyclomatic: Some(a.max_cyclomatic),
        avg_cognitive: avg_cog,
    }
}

// ---------------------------------------------------------------------------
// Manual fallback analysis (Go, Python, generic text)
// ---------------------------------------------------------------------------

pub struct ManualMetrics {
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
}

pub fn analyze_manual(text: &str) -> ManualMetrics {
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

        if is_fn_definition(trimmed) {
            fn_count += 1;
        }

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
/// Marker-debt (TODOs) is excluded from the score — rewrite bookkeeping
/// should not inflate cognitive-load categories.
pub fn compute_score_manual(m: &ManualMetrics) -> f64 {
    let size = (m.line_count as f64 / 100.0).min(20.0);
    let fns = m.fn_count as f64;
    let depth = m.max_indent_depth as f64 * 2.0;

    size + fns + depth
}

pub fn file_score_from_manual(path: String, m: &ManualMetrics) -> FileScore {
    let rounded_score = round2(compute_score_manual(m));

    FileScore {
        path,
        line_count: m.line_count,
        fn_count: m.fn_count,
        todo_count: m.todo_count,
        max_indent_depth: m.max_indent_depth,
        score: rounded_score,
        score_category: categorize_file_score(rounded_score),
        recommended_action: recommended_action_for_file_score(rounded_score),
        analysis_method: "heuristic",
        avg_cyclomatic: None,
        max_cyclomatic: None,
        avg_cognitive: None,
    }
}

// ---------------------------------------------------------------------------
// Unified scoring — tries crate first, falls back to manual
// ---------------------------------------------------------------------------

/// Intermediate analysis result shared between summary and scoring paths.
pub struct AnalysisSummary {
    pub analysis_method: &'static str,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: f64,
    pub functions: Vec<FunctionEntry>,
    pub code_smells: Vec<CodeSmellEntry>,
}

pub fn summarize_analysis(text: &str, file_path: &Path) -> AnalysisSummary {
    if let Some(ca) = analyze_with_crate(text, file_path) {
        let score = round2(compute_score_crate(&ca));
        return AnalysisSummary {
            analysis_method: "ast",
            line_count: ca.line_count,
            fn_count: ca.fn_count,
            todo_count: ca.todo_count,
            max_indent_depth: ca.max_nesting as usize,
            score,
            functions: ca.functions,
            code_smells: ca.code_smells,
        };
    }

    let m = analyze_manual(text);
    let score = round2(compute_score_manual(&m));

    AnalysisSummary {
        analysis_method: "heuristic",
        line_count: m.line_count,
        fn_count: m.fn_count,
        todo_count: m.todo_count,
        max_indent_depth: m.max_indent_depth,
        score,
        functions: Vec::new(),
        code_smells: Vec::new(),
    }
}

/// Unified scoring — tries crate first, falls back to manual.
pub fn score_text(repo_root: &Path, path: &Path, text: &str) -> FileScore {
    let rel = crate::repo::make_relative(repo_root, path);

    if let Some(crate_analysis) = analyze_with_crate(text, path) {
        return file_score_from_crate(rel, &crate_analysis);
    }

    let manual = analyze_manual(text);
    file_score_from_manual(rel, &manual)
}

// ---------------------------------------------------------------------------
// Shared predicates
// ---------------------------------------------------------------------------

/// Detect function definitions across Rust (`fn`), Go (`func`), and Python
/// (`def`).  Operates on a left-trimmed line.
pub fn is_fn_definition(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("pub(crate) fn ")
        || trimmed.starts_with("pub(crate) async fn ")
        || trimmed.starts_with("func ")
        || trimmed.starts_with("def ")
}

pub fn contains_todo_marker(line: &str) -> bool {
    let upper = line.to_uppercase();
    upper.contains("TODO") || upper.contains("FIXME")
}

pub fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, ch| match ch {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

// ---------------------------------------------------------------------------
// Detail collectors
// ---------------------------------------------------------------------------

use super::types::TodoEntry;

pub fn collect_todos(text: &str, limit: usize) -> Vec<TodoEntry> {
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

pub fn collect_long_fns(text: &str, threshold: usize) -> Vec<usize> {
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

        let Some(start) = fn_start else { continue };

        if brace_depth > 0 || line_number <= start {
            continue;
        }

        let fn_len = line_number - start + 1;
        if fn_len >= threshold {
            long_starts.push(start);
        }

        fn_start = None;
        brace_depth = 0;
    }

    long_starts
}
