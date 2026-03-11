use crate::{cogload, context, fs, log, profile, repo, search};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs as tokio_fs;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchRequest {
    query: String,
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListPathsRequest {
    base_path: Option<String>,
    extensions: Option<Vec<String>>,
    recursive: Option<bool>,
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchPathsRequest {
    query: String,
    paths: Vec<String>,
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ContextBundleRequest {
    bundle: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ContextSnapshotRequest {
    snapshot: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ActiveContextRequest {
    max_chars: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileRequest {
    path: String,
    max_chars: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileLinesRequest {
    path: String,
    start_line: u32,
    end_line: u32,
    max_chars: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FileMetadataRequest {
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapHotspotsRequest {
    limit: Option<u32>,
    path_prefix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapFileSummaryRequest {
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapTouchedFilesRequest {
    paths: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapCodeSmellsRequest {
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapFunctionComplexityRequest {
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapUnifiedAnalysisRequest {
    /// Maximum number of items to return (default: 20).
    limit: Option<u32>,
    /// Optional sub-path to scope the analysis.
    path_prefix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapCiGateRequest {
    /// Optional sub-path to scope the analysis.
    path_prefix: Option<String>,
    /// Optional list of repo-relative file paths to restrict violations to.
    /// When provided, only violations in these files are reported; the
    /// debt_density gate is skipped (it is a whole-scope metric).
    paths: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Server handler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CfdRsMemory {
    repo_root: PathBuf,
    repo_root_canon: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CfdRsMemory {
    pub fn new(repo_root: PathBuf, repo_root_canon: PathBuf) -> Self {
        Self {
            repo_root,
            repo_root_canon,
            tool_router: Self::tool_router(),
        }
    }

    // -- search tools -------------------------------------------------------

    #[tool(description = "Search governance and policy files, returning small grounded hits.")]
    async fn find_governance(
        &self,
        Parameters(SearchRequest { query, max_results }): Parameters<SearchRequest>,
    ) -> String {
        let span = log::ToolSpan::start("find_governance");
        let roots = profile::governance_roots(&self.repo_root);
        let max = max_results.unwrap_or(5).clamp(1, 10) as usize;
        self.search_and_respond(&span, &roots, &query, max).await
    }

    #[tool(description = "Search frozen behavior/parity sources, returning small grounded hits.")]
    async fn find_behavior_truth(
        &self,
        Parameters(SearchRequest { query, max_results }): Parameters<SearchRequest>,
    ) -> String {
        let span = log::ToolSpan::start("find_behavior_truth");
        let roots = profile::behavior_truth_roots(&self.repo_root);
        let max = max_results.unwrap_or(5).clamp(1, 10) as usize;
        self.search_and_respond(&span, &roots, &query, max).await
    }

    #[tool(
        description = "Search only the provided repo-relative files or directories, returning small \
                       grounded hits."
    )]
    async fn search_paths(
        &self,
        Parameters(SearchPathsRequest {
            query,
            paths,
            max_results,
        }): Parameters<SearchPathsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("search_paths");

        if paths.is_empty() {
            span.error("paths must not be empty");
            return path_error("paths must not be empty", "");
        }

        let roots = match resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
            Ok(r) => r,
            Err((error, path)) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let max = max_results.unwrap_or(5).clamp(1, 20) as usize;
        self.search_and_respond(&span, &roots, &query, max).await
    }

    // -- listing tools ------------------------------------------------------

    #[tool(
        description = "List repo paths under a repo-relative directory, with optional recursion and \
                       extension filtering."
    )]
    async fn list_paths(
        &self,
        Parameters(ListPathsRequest {
            base_path,
            extensions,
            recursive,
            max_results,
        }): Parameters<ListPathsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("list_paths");
        let base_path = base_path.unwrap_or_else(|| ".".to_string());
        let recursive = recursive.unwrap_or(false);
        let max_results = max_results.unwrap_or(100).clamp(1, 500) as usize;

        span.detail(&format!("base_path={} recursive={}", base_path, recursive));

        let base_path_canon = match repo::resolve(&self.repo_root, &self.repo_root_canon, base_path.as_str())
        {
            Ok(path) => path,
            Err(error) => {
                span.error(error);
                return path_error(error, &base_path);
            }
        };

        let filter_extensions = fs::normalize_extensions(extensions.as_deref());

        let entries = fs::collect_paths(
            &self.repo_root,
            &base_path_canon,
            recursive,
            filter_extensions.as_ref(),
            max_results,
        )
        .await;

        span.done(&format!("count={}", entries.len()));
        to_json(entries)
    }

    // -- context tools ------------------------------------------------------

    #[tool(description = "Return a curated narrow context bundle for a common repository question type.")]
    async fn get_context_bundle(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_bundle");
        match profile::bundle(bundle.trim()) {
            Some(b) => {
                span.done(&format!("bundle={} entries={}", b.bundle, b.entries.len()));
                to_json(b)
            }
            None => {
                span.error("unknown bundle");
                to_json(serde_json::json!({
                    "error": "unknown bundle",
                    "supported_bundles": profile::supported_bundle_names()
                }))
            }
        }
    }

    #[tool(description = "Return a compact first-read brief for a curated repository context bundle.")]
    async fn get_context_brief(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_brief");
        match profile::brief(bundle.trim()) {
            Some(b) => {
                span.done(&format!("bundle={}", b.bundle));
                to_json(b)
            }
            None => {
                span.error("unknown bundle");
                to_json(serde_json::json!({
                    "error": "unknown bundle",
                    "supported_bundles": profile::supported_bundle_names()
                }))
            }
        }
    }

    #[tool(
        description = "Return a compact source-backed snapshot for common repo-state or phase-state \
                       questions."
    )]
    async fn get_context_snapshot(
        &self,
        Parameters(ContextSnapshotRequest { snapshot }): Parameters<ContextSnapshotRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_snapshot");
        match profile::snapshot(snapshot.trim()) {
            Some(s) => {
                span.done(&format!("snapshot={} facts={}", s.snapshot, s.facts.len()));
                to_json(s)
            }
            None => {
                span.error("unknown snapshot");
                to_json(serde_json::json!({
                    "error": "unknown snapshot",
                    "supported_snapshots": profile::supported_snapshot_names()
                }))
            }
        }
    }

    #[tool(
        description = "Return active context from docs/ACTIVE_CONTEXT.md when present, with explicit \
                       missing-file fallback."
    )]
    async fn get_active_context(
        &self,
        Parameters(ActiveContextRequest { max_chars }): Parameters<ActiveContextRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_active_context");
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 12000) as usize;

        let active_context = context::load_active_context(&self.repo_root, max_chars).await;

        span.done(&format!(
            "found={} source={} truncated={}",
            active_context.found, active_context.source, active_context.truncated,
        ));

        to_json(active_context)
    }

    // -- read tools ---------------------------------------------------------

    #[tool(description = "Read a repo file with truncation and repo-boundary enforcement.")]
    async fn read_file(
        &self,
        Parameters(ReadFileRequest { path, max_chars }): Parameters<ReadFileRequest>,
    ) -> String {
        let span = log::ToolSpan::start("read_file");
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 12000) as usize;

        let resolved = match self.resolve_repo_file(&path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let text = match tokio_fs::read_to_string(&resolved).await {
            Ok(t) => t,
            Err(_) => {
                span.error("file not readable as UTF-8 text");
                return path_error("file not readable as UTF-8 text", &path);
            }
        };

        let truncated = text.chars().count() > max_chars;
        let content: String = text.chars().take(max_chars).collect();

        span.done(&format!("path={} truncated={}", path, truncated));
        to_json(serde_json::json!({
            "path": path,
            "truncated": truncated,
            "content": content
        }))
    }

    #[tool(
        description = "Read a specific line range from a repo file with truncation and repo-boundary \
                       enforcement."
    )]
    async fn read_file_lines(
        &self,
        Parameters(ReadFileLinesRequest {
            path,
            start_line,
            end_line,
            max_chars,
        }): Parameters<ReadFileLinesRequest>,
    ) -> String {
        let span = log::ToolSpan::start("read_file_lines");
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 16000) as usize;

        if start_line == 0 || end_line < start_line {
            span.error("invalid line range");
            return path_error(
                "line range must be 1-based and end_line must be >= start_line",
                &path,
            );
        }

        let resolved = match self.resolve_repo_file(&path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let text = match tokio_fs::read_to_string(&resolved).await {
            Ok(t) => t,
            Err(_) => {
                span.error("file not readable as UTF-8 text");
                return path_error("file not readable as UTF-8 text", &path);
            }
        };

        let (content, total_line_count, truncated) =
            match fs::slice_lines(text.as_str(), start_line as usize, end_line as usize, max_chars) {
                Ok(result) => result,
                Err(error) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

        let actual_end = usize::min(end_line as usize, total_line_count);
        span.done(&format!(
            "path={} lines={}..{} truncated={}",
            path, start_line, actual_end, truncated,
        ));
        to_json(serde_json::json!({
            "path": path,
            "start_line": start_line,
            "end_line": actual_end,
            "total_line_count": total_line_count,
            "truncated": truncated,
            "content": content
        }))
    }

    // -- metadata tools -----------------------------------------------------

    #[tool(description = "Return metadata for a repo path, including line count for readable text files.")]
    async fn file_metadata(
        &self,
        Parameters(FileMetadataRequest { path }): Parameters<FileMetadataRequest>,
    ) -> String {
        let span = log::ToolSpan::start("file_metadata");

        let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let metadata = match tokio_fs::metadata(&resolved).await {
            Ok(m) => m,
            Err(_) => {
                span.error("path not readable");
                return path_error("path not readable", &path);
            }
        };

        let result = build_file_metadata(&resolved, &path, &metadata).await;

        span.done(&format!("path={} kind={}", result.path, result.kind));
        to_json(result)
    }

    // -- debtmap tools ------------------------------------------------------

    #[tool(
        description = "Return top cognitive-load hotspot files for the repo or a bounded path prefix. \
                       Includes additive file-level score categories and recommended-action labels; use for \
                       refactor triage, not as always-on context."
    )]
    async fn debtmap_top_hotspots(
        &self,
        Parameters(DebtmapHotspotsRequest { limit, path_prefix }): Parameters<DebtmapHotspotsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_top_hotspots");
        let limit = limit.unwrap_or(10).clamp(1, 50) as usize;

        let scope = match &path_prefix {
            Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                Ok(p) => Some(p),
                Err(error) => {
                    span.error(error);
                    return path_error(error, prefix);
                }
            },
            None => None,
        };

        span.detail(&format!("limit={} prefix={:?}", limit, path_prefix));

        let hotspots = cogload::top_hotspots(&self.repo_root, scope.as_deref(), limit).await;

        span.done(&format!("hotspots={}", hotspots.len()));
        to_json(hotspots)
    }

    #[tool(
        description = "Return a focused Debtmap summary for one file, including TODO locations and \
                       long-function line numbers. Includes file-level score category and \
                       recommended-action labels."
    )]
    async fn debtmap_file_summary(
        &self,
        Parameters(DebtmapFileSummaryRequest { path }): Parameters<DebtmapFileSummaryRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_file_summary");

        let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        match cogload::file_summary(&self.repo_root, &resolved).await {
            Ok(summary) => {
                span.done(&format!("path={} score={}", summary.path, summary.score));
                to_json(summary)
            }
            Err(error) => {
                span.error(error);
                path_error(error, &path)
            }
        }
    }

    #[tool(
        description = "Score a provided list of touched files for bounded cognitive-load review. Use after \
                       edits, not as always-on analysis. Includes file-level score categories and \
                       recommended-action labels."
    )]
    async fn debtmap_touched_files_review(
        &self,
        Parameters(DebtmapTouchedFilesRequest { paths }): Parameters<DebtmapTouchedFilesRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_touched_files_review");

        if paths.is_empty() {
            span.error("paths must not be empty");
            return path_error("paths must not be empty", "");
        }

        let resolved = match resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
            Ok(r) => r,
            Err((error, path)) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let review = cogload::touched_files_review(&self.repo_root, &resolved).await;

        span.done(&format!(
            "files={} total_score={} skipped={}",
            review.files.len(),
            review.total_score,
            review.skipped.len(),
        ));
        to_json(review)
    }

    #[tool(
        description = "Detect code smells in a single file using the debtmap crate's AST analysis. Works \
                       best for Rust, TypeScript, and JavaScript files."
    )]
    async fn debtmap_code_smells(
        &self,
        Parameters(DebtmapCodeSmellsRequest { path }): Parameters<DebtmapCodeSmellsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_code_smells");

        let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        match cogload::code_smells(&self.repo_root, &resolved).await {
            Ok(report) => {
                span.done(&format!("path={} smells={}", report.path, report.total));
                to_json(report)
            }
            Err(error) => {
                span.error(error);
                path_error(error, &path)
            }
        }
    }

    #[tool(
        description = "Return per-function complexity breakdown for a single file. Uses AST analysis for \
                       Rust, TypeScript, and JavaScript; heuristic fallback for other languages. Includes \
                       separate cyclomatic, cognitive, and total-complexity categories plus \
                       recommended-action labels."
    )]
    async fn debtmap_function_complexity(
        &self,
        Parameters(DebtmapFunctionComplexityRequest { path }): Parameters<DebtmapFunctionComplexityRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_function_complexity");

        let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(p) => p,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        match cogload::function_complexity(&self.repo_root, &resolved).await {
            Ok(report) => {
                span.done(&format!(
                    "path={} fn_count={} method={}",
                    report.path, report.fn_count, report.analysis_method,
                ));
                to_json(report)
            }
            Err(error) => {
                span.error(error);
                path_error(error, &path)
            }
        }
    }

    #[tool(
        description = "Run full unified analysis (identical to debtmap CLI `analyze`) detecting God \
                       Objects, coupling, cohesion, and call-graph issues. Returns structured items sorted \
                       by score. Use for deep structural analysis, not routine edits."
    )]
    async fn debtmap_unified_analysis(
        &self,
        Parameters(DebtmapUnifiedAnalysisRequest { limit, path_prefix }): Parameters<
            DebtmapUnifiedAnalysisRequest,
        >,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_unified_analysis");
        let limit = limit.unwrap_or(20).clamp(1, 100) as usize;

        let scope = match &path_prefix {
            Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                Ok(p) => Some(p),
                Err(error) => {
                    span.error(error);
                    return path_error(error, prefix);
                }
            },
            None => None,
        };

        span.detail(&format!("limit={} prefix={:?}", limit, path_prefix));

        match cogload::run_unified_analysis(&self.repo_root, scope.as_deref(), limit).await {
            Ok(report) => {
                span.done(&format!(
                    "items={} density={:.1} loc={}",
                    report.total_items, report.debt_density, report.total_loc,
                ));
                to_json(report)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(
        description = "Evaluate CI gate rules against the repo. Returns pass/fail with blocking violations \
                       and warnings. When `paths` is provided, only violations in those files are reported \
                       and the debt_density gate is skipped. Blocking rules: priority critical/high, \
                       god_object_score >= 45, cyclomatic >= 31, cognitive >= 25. Warning rules: priority \
                       medium, god_object_score < 45, coupling Hub/highly_coupled, cyclomatic 21-30, \
                       cognitive 15-24."
    )]
    async fn debtmap_ci_gate(
        &self,
        Parameters(DebtmapCiGateRequest { path_prefix, paths }): Parameters<DebtmapCiGateRequest>,
    ) -> String {
        let span = log::ToolSpan::start("debtmap_ci_gate");

        let scope = match &path_prefix {
            Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                Ok(p) => Some(p),
                Err(error) => {
                    span.error(error);
                    return path_error(error, prefix);
                }
            },
            None => None,
        };

        let touched_filter: Option<std::collections::HashSet<String>> =
            paths.map(|p| p.into_iter().collect());

        span.detail(&format!(
            "prefix={:?} touched_filter={}",
            path_prefix,
            touched_filter
                .as_ref()
                .map_or("none".to_string(), |s| format!("{} files", s.len())),
        ));

        // Run full analysis with a high limit so CI gate sees everything.
        match cogload::run_unified_analysis(&self.repo_root, scope.as_deref(), 500).await {
            Ok(report) => {
                let gate = cogload::evaluate_ci_gate_filtered(&report, touched_filter.as_ref());
                span.done(&format!(
                    "pass={} blocking={} warnings={}",
                    gate.pass,
                    gate.blocking.len(),
                    gate.warnings.len(),
                ));
                to_json(gate)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl CfdRsMemory {
    /// Shared search-and-format for scoped search tools.
    async fn search_and_respond(
        &self,
        span: &log::ToolSpan,
        roots: &[PathBuf],
        query: &str,
        max_results: usize,
    ) -> String {
        span.detail(&format!("roots={}", roots.len()));

        match search::search_roots(&self.repo_root, roots, query, max_results).await {
            Ok(hits) => {
                span.done(&format!("hits={}", hits.len()));
                to_json(hits)
            }
            Err(error) => {
                span.error(error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    /// Resolve a repo-relative path to a canonical file path, rejecting
    /// non-files and paths outside the repo boundary.
    fn resolve_repo_file(&self, path: &str) -> Result<PathBuf, &'static str> {
        let resolved = repo::resolve(&self.repo_root, &self.repo_root_canon, path)?;
        if !resolved.is_file() {
            return Err("path is not a regular file");
        }
        Ok(resolved)
    }
}

#[tool_handler]
impl ServerHandler for CfdRsMemory {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Read-only repository memory server. Prefer staged retrieval: route first, list or search the \
             smallest path set, inspect metadata or snippets, then read only the needed lines or chunk \
             before making claims."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

/// Resolve a list of repo-relative paths, returning all or the first error.
fn resolve_paths(
    repo_root: &PathBuf,
    repo_root_canon: &PathBuf,
    paths: &[String],
) -> Result<Vec<PathBuf>, (&'static str, String)> {
    let mut resolved = Vec::with_capacity(paths.len());
    for path in paths {
        match repo::resolve(repo_root, repo_root_canon, path.as_str()) {
            Ok(p) => resolved.push(p),
            Err(error) => return Err((error, path.clone())),
        }
    }
    Ok(resolved)
}

/// Build file metadata, determining kind and optional line count.
async fn build_file_metadata(
    resolved: &PathBuf,
    path: &str,
    metadata: &std::fs::Metadata,
) -> fs::FileMetadata {
    let kind = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };

    let line_count = if metadata.is_file() && fs::is_text_file(resolved) {
        tokio_fs::read_to_string(resolved)
            .await
            .ok()
            .map(|text| text.lines().count())
    } else {
        None
    };

    fs::FileMetadata {
        path: path.to_string(),
        kind,
        size_bytes: metadata.len(),
        line_count,
    }
}

fn to_json<T: Serialize>(value: T) -> String {
    serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
}

fn path_error(error: &str, path: &str) -> String {
    to_json(serde_json::json!({ "error": error, "path": path }))
}
