use crate::{fs as mcp_fs, repo};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::analysis::{
    analyze_manual, analyze_with_crate, collect_long_fns, collect_todos, score_text, summarize_analysis,
};
use super::collection::{collect_analyzable_files, read_analyzable};
use super::scoring::round2;
use super::scoring::{categorize_file_score, recommended_action_for_file_score};
use super::types::{
    CodeSmellEntry, CodeSmellReport, FileScore, FileSummary, FunctionComplexityReport, SkippedFile,
    TouchedFilesReview,
};

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
                && text.len() as u64 <= super::analysis::MAX_ANALYZABLE_SIZE
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
    let analysis = summarize_analysis(&text, file_path);
    let top_todos = collect_todos(&text, 10);

    let long_fn_lines = if analysis.analysis_method == "ast" {
        analysis
            .functions
            .iter()
            .filter(|f| f.length >= 60)
            .map(|f| f.line)
            .collect()
    } else {
        collect_long_fns(&text, 60)
    };

    Ok(FileSummary {
        path: rel,
        line_count: analysis.line_count,
        fn_count: analysis.fn_count,
        todo_count: analysis.todo_count,
        max_indent_depth: analysis.max_indent_depth,
        score: analysis.score,
        score_category: categorize_file_score(analysis.score),
        recommended_action: recommended_action_for_file_score(analysis.score),
        analysis_method: analysis.analysis_method,
        top_todos,
        long_fn_lines,
        functions: analysis.functions,
        code_smells: analysis.code_smells,
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
        match score_touched_file(repo_root, path).await {
            Ok(score) => files.push(score),
            Err(skip) => skipped.push(skip),
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

async fn score_touched_file(repo_root: &Path, path: &Path) -> Result<FileScore, SkippedFile> {
    let rel = repo::make_relative(repo_root, path);

    if !path.is_file() {
        return Err(SkippedFile {
            path: rel,
            reason: "not a regular file",
        });
    }

    if !mcp_fs::is_text_file(path) {
        return Err(SkippedFile {
            path: rel,
            reason: "not a recognized text file",
        });
    }

    let text = read_analyzable(path)
        .await
        .map_err(|reason| SkippedFile { path: rel, reason })?;
    Ok(score_text(repo_root, path, &text))
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

/// Return per-function complexity breakdown for one file.
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
