#[cfg(feature = "debtmap")]
use crate::cogload;
use crate::{fs, log, phase5, profile, repo, search, workspace};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs as tokio_fs;

#[allow(dead_code)]
pub const CORE_TOOL_NAMES: &[&str] = &[
    "find_governance",
    "find_behavior_truth",
    "search_paths",
    "grep_paths",
    "list_paths",
    "get_context_bundle",
    "get_context_brief",
    "get_context_snapshot",
    "read_file",
    "read_file_lines",
    "file_metadata",
    "status_summary",
    "phase5_priority",
    "parity_row_details",
    "domain_gaps_ranked",
    "baseline_source_mapping",
    "crate_surface_summary",
    "crate_dependency_graph",
];

#[allow(dead_code)]
pub const DEBTMAP_TOOL_NAMES: &[&str] = &[
    "debtmap_top_hotspots",
    "debtmap_file_summary",
    "debtmap_touched_files_review",
    "debtmap_code_smells",
    "debtmap_function_complexity",
    "debtmap_unified_analysis",
    "debtmap_ci_gate",
];

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
struct GrepPathsRequest {
    pattern: String,
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
struct EmptyRequest {}

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
struct ParityRowDetailsRequest {
    row_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DomainGapsRankedRequest {
    domain: String,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct BaselineSourceMappingRequest {
    row_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CrateSurfaceSummaryRequest {
    crate_name: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapHotspotsRequest {
    limit: Option<u32>,
    path_prefix: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapFileSummaryRequest {
    path: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapTouchedFilesRequest {
    paths: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapCodeSmellsRequest {
    path: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapFunctionComplexityRequest {
    path: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapUnifiedAnalysisRequest {
    limit: Option<u32>,
    path_prefix: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct DebtmapCiGateRequest {
    path_prefix: Option<String>,
    paths: Option<Vec<String>>,
}

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

    #[tool(description = "Search frozen behavior and parity sources, returning small grounded hits.")]
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
            Ok(resolved) => resolved,
            Err((error, path)) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let max = max_results.unwrap_or(5).clamp(1, 20) as usize;
        self.search_and_respond(&span, &roots, &query, max).await
    }

    #[tool(
        description = "Regex search across repo-relative files or directories, returning matched lines with \
                       file paths and line numbers."
    )]
    async fn grep_paths(
        &self,
        Parameters(GrepPathsRequest {
            pattern,
            paths,
            max_results,
        }): Parameters<GrepPathsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("grep_paths");

        if paths.is_empty() {
            span.error("paths must not be empty");
            return path_error("paths must not be empty", "");
        }

        if pattern.is_empty() {
            span.error("pattern must not be empty");
            return path_error("pattern must not be empty", "");
        }

        let roots = match resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
            Ok(resolved) => resolved,
            Err((error, path)) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let max = max_results.unwrap_or(50).clamp(1, 200) as usize;
        span.detail(&format!("pattern={} roots={} max={}", pattern, roots.len(), max));

        match search::grep_roots(&self.repo_root, &roots, &pattern, max).await {
            Ok(hits) => {
                span.done(&format!("hits={}", hits.len()));
                to_json(hits)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(
        description = "List repo paths under a repo-relative directory with optional recursion and \
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

        let base_path_canon = match repo::resolve(&self.repo_root, &self.repo_root_canon, &base_path) {
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

    #[tool(description = "Return a curated narrow context bundle for a common repository question type.")]
    async fn get_context_bundle(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_bundle");
        match profile::bundle(bundle.trim()) {
            Some(bundle) => {
                span.done(&format!(
                    "bundle={} entries={}",
                    bundle.bundle,
                    bundle.entries.len()
                ));
                to_json(bundle)
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
            Some(brief) => {
                span.done(&format!("bundle={}", brief.bundle));
                to_json(brief)
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

    #[tool(description = "Return a compact source-backed snapshot for a core rewrite routing question.")]
    async fn get_context_snapshot(
        &self,
        Parameters(ContextSnapshotRequest { snapshot }): Parameters<ContextSnapshotRequest>,
    ) -> String {
        let span = log::ToolSpan::start("get_context_snapshot");
        match profile::snapshot(snapshot.trim()) {
            Some(snapshot) => {
                span.done(&format!(
                    "snapshot={} facts={}",
                    snapshot.snapshot,
                    snapshot.facts.len()
                ));
                to_json(snapshot)
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

    #[tool(description = "Return the current tracked status summary from STATUS.md.")]
    async fn status_summary(&self, Parameters(EmptyRequest {}): Parameters<EmptyRequest>) -> String {
        let span = log::ToolSpan::start("status_summary");

        match phase5::status_summary(&self.repo_root) {
            Ok(summary) => {
                span.done(&format!("milestone={}", summary.active_milestone));
                to_json(summary)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(description = "Return the current Phase 5 priority queue and active milestone detail.")]
    async fn phase5_priority(&self, Parameters(EmptyRequest {}): Parameters<EmptyRequest>) -> String {
        let span = log::ToolSpan::start("phase5_priority");

        match phase5::phase5_priority(&self.repo_root) {
            Ok(priority) => {
                span.done(&format!("milestone={}", priority.active_milestone.name));
                to_json(priority)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(description = "Return combined ledger and roadmap detail for one exact parity row ID.")]
    async fn parity_row_details(
        &self,
        Parameters(ParityRowDetailsRequest { row_id }): Parameters<ParityRowDetailsRequest>,
    ) -> String {
        let span = log::ToolSpan::start("parity_row_details");

        match phase5::parity_row_details(&self.repo_root, &row_id) {
            Ok(details) => {
                span.done(&format!(
                    "row_id={} milestone={}",
                    details.row_id, details.roadmap.milestone
                ));
                to_json(details)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "row_id": row_id }))
            }
        }
    }

    #[tool(
        description = "Return ranked open gaps for one parity domain without loading all ledgers together."
    )]
    async fn domain_gaps_ranked(
        &self,
        Parameters(DomainGapsRankedRequest { domain, limit }): Parameters<DomainGapsRankedRequest>,
    ) -> String {
        let span = log::ToolSpan::start("domain_gaps_ranked");
        let limit = limit.unwrap_or(10).clamp(1, 50) as usize;

        match phase5::domain_gaps_ranked(&self.repo_root, &domain, limit) {
            Ok(ranked) => {
                span.done(&format!("domain={} rows={}", ranked.domain, ranked.rows.len()));
                to_json(ranked)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "domain": domain }))
            }
        }
    }

    #[tool(
        description = "Map one parity row ID back to frozen baseline source files, symbol hints, and the \
                       exact parity feature doc."
    )]
    async fn baseline_source_mapping(
        &self,
        Parameters(BaselineSourceMappingRequest { row_id }): Parameters<BaselineSourceMappingRequest>,
    ) -> String {
        let span = log::ToolSpan::start("baseline_source_mapping");

        match phase5::baseline_source_mapping(&self.repo_root, &row_id) {
            Ok(mapping) => {
                span.done(&format!(
                    "row_id={} paths={}",
                    mapping.row_id,
                    mapping.baseline_paths.len()
                ));
                to_json(mapping)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "row_id": row_id }))
            }
        }
    }

    #[tool(
        description = "Summarize one workspace crate surface, ownership, and allowed direct dependencies."
    )]
    async fn crate_surface_summary(
        &self,
        Parameters(CrateSurfaceSummaryRequest { crate_name }): Parameters<CrateSurfaceSummaryRequest>,
    ) -> String {
        let span = log::ToolSpan::start("crate_surface_summary");

        match workspace::crate_surface_summary(&self.repo_root, &crate_name) {
            Ok(summary) => {
                span.done(&format!(
                    "crate={} deps={}",
                    summary.crate_name,
                    summary.direct_dependencies.len()
                ));
                to_json(summary)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error, "crate_name": crate_name }))
            }
        }
    }

    #[tool(description = "Return the workspace crate dependency graph and architecture-policy verdict.")]
    async fn crate_dependency_graph(&self, Parameters(EmptyRequest {}): Parameters<EmptyRequest>) -> String {
        let span = log::ToolSpan::start("crate_dependency_graph");

        match workspace::crate_dependency_graph(&self.repo_root) {
            Ok(graph) => {
                span.done(&format!(
                    "nodes={} edges={} violations={}",
                    graph.nodes.len(),
                    graph.edges.len(),
                    graph.violations.len()
                ));
                to_json(graph)
            }
            Err(error) => {
                span.error(&error);
                to_json(serde_json::json!({ "error": error }))
            }
        }
    }

    #[tool(description = "Read a repo file with truncation and repo-boundary enforcement.")]
    async fn read_file(
        &self,
        Parameters(ReadFileRequest { path, max_chars }): Parameters<ReadFileRequest>,
    ) -> String {
        let span = log::ToolSpan::start("read_file");
        let max_chars = max_chars.unwrap_or(8000).clamp(200, 32000) as usize;

        let resolved = match self.resolve_repo_file(&path) {
            Ok(path) => path,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let text = match tokio_fs::read_to_string(&resolved).await {
            Ok(text) => text,
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
            "content": content,
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
        let max_chars = max_chars.unwrap_or(8000).clamp(200, 32000) as usize;

        if start_line == 0 || end_line < start_line {
            span.error("invalid line range");
            return path_error(
                "line range must be 1-based and end_line must be >= start_line",
                &path,
            );
        }

        let resolved = match self.resolve_repo_file(&path) {
            Ok(path) => path,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let text = match tokio_fs::read_to_string(&resolved).await {
            Ok(text) => text,
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
            path, start_line, actual_end, truncated
        ));
        to_json(serde_json::json!({
            "path": path,
            "start_line": start_line,
            "end_line": actual_end,
            "total_line_count": total_line_count,
            "truncated": truncated,
            "content": content,
        }))
    }

    #[tool(description = "Return metadata for a repo path, including line count for readable text files.")]
    async fn file_metadata(
        &self,
        Parameters(FileMetadataRequest { path }): Parameters<FileMetadataRequest>,
    ) -> String {
        let span = log::ToolSpan::start("file_metadata");

        let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(path) => path,
            Err(error) => {
                span.error(error);
                return path_error(error, &path);
            }
        };

        let metadata = match tokio_fs::metadata(&resolved).await {
            Ok(metadata) => metadata,
            Err(_) => {
                span.error("path not readable");
                return path_error("path not readable", &path);
            }
        };

        let result = build_file_metadata(&resolved, &path, &metadata).await;
        span.done(&format!("path={} kind={}", result.path, result.kind));
        to_json(result)
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(
            description = "Return top cognitive-load hotspot files for the repo or a bounded path prefix. \
                           Use for refactor triage, not as always-on context."
        )
    )]
    #[allow(dead_code)]
    async fn debtmap_top_hotspots(
        &self,
        Parameters(DebtmapHotspotsRequest { limit, path_prefix }): Parameters<DebtmapHotspotsRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&limit, &path_prefix);
            debtmap_unavailable("debtmap_top_hotspots")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_top_hotspots");
            let limit = limit.unwrap_or(10).clamp(1, 50) as usize;

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
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
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Return a focused debtmap summary for one file.")
    )]
    #[allow(dead_code)]
    async fn debtmap_file_summary(
        &self,
        Parameters(DebtmapFileSummaryRequest { path }): Parameters<DebtmapFileSummaryRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            debtmap_unavailable("debtmap_file_summary")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_file_summary");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
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
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Score a provided list of touched files for bounded cognitive-load review.")
    )]
    #[allow(dead_code)]
    async fn debtmap_touched_files_review(
        &self,
        Parameters(DebtmapTouchedFilesRequest { paths }): Parameters<DebtmapTouchedFilesRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &paths;
            debtmap_unavailable("debtmap_touched_files_review")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_touched_files_review");

            if paths.is_empty() {
                span.error("paths must not be empty");
                return path_error("paths must not be empty", "");
            }

            let resolved = match resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
                Ok(resolved) => resolved,
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
                review.skipped.len()
            ));
            to_json(review)
        }
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Detect code smells in a single file using debtmap AST analysis.")
    )]
    #[allow(dead_code)]
    async fn debtmap_code_smells(
        &self,
        Parameters(DebtmapCodeSmellsRequest { path }): Parameters<DebtmapCodeSmellsRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            debtmap_unavailable("debtmap_code_smells")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_code_smells");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
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
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Return per-function complexity breakdown for a single file.")
    )]
    #[allow(dead_code)]
    async fn debtmap_function_complexity(
        &self,
        Parameters(DebtmapFunctionComplexityRequest { path }): Parameters<DebtmapFunctionComplexityRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            debtmap_unavailable("debtmap_function_complexity")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_function_complexity");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
                Err(error) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

            match cogload::function_complexity(&self.repo_root, &resolved).await {
                Ok(report) => {
                    span.done(&format!(
                        "path={} fn_count={} method={}",
                        report.path, report.fn_count, report.analysis_method
                    ));
                    to_json(report)
                }
                Err(error) => {
                    span.error(error);
                    path_error(error, &path)
                }
            }
        }
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Run full unified debtmap analysis for deep structural review.")
    )]
    #[allow(dead_code)]
    async fn debtmap_unified_analysis(
        &self,
        Parameters(DebtmapUnifiedAnalysisRequest { limit, path_prefix }): Parameters<
            DebtmapUnifiedAnalysisRequest,
        >,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&limit, &path_prefix);
            debtmap_unavailable("debtmap_unified_analysis")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_unified_analysis");
            let limit = limit.unwrap_or(20).clamp(1, 100) as usize;

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
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
                        report.total_items, report.debt_density, report.total_loc
                    ));
                    to_json(report)
                }
                Err(error) => {
                    span.error(&error);
                    to_json(serde_json::json!({ "error": error }))
                }
            }
        }
    }

    #[cfg_attr(
        feature = "debtmap",
        tool(description = "Evaluate debtmap CI gate rules against the repo or a bounded file set.")
    )]
    #[allow(dead_code)]
    async fn debtmap_ci_gate(
        &self,
        Parameters(DebtmapCiGateRequest { path_prefix, paths }): Parameters<DebtmapCiGateRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&path_prefix, &paths);
            debtmap_unavailable("debtmap_ci_gate")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_ci_gate");

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
                    Err(error) => {
                        span.error(error);
                        return path_error(error, prefix);
                    }
                },
                None => None,
            };

            let touched_filter: Option<std::collections::HashSet<String>> =
                paths.map(|items| items.into_iter().collect());

            span.detail(&format!(
                "prefix={:?} touched_filter={}",
                path_prefix,
                touched_filter
                    .as_ref()
                    .map_or("none".to_string(), |items| format!("{} files", items.len()))
            ));

            match cogload::run_unified_analysis(&self.repo_root, scope.as_deref(), 500).await {
                Ok(report) => {
                    let gate = cogload::evaluate_ci_gate_filtered(&report, touched_filter.as_ref());
                    span.done(&format!(
                        "pass={} blocking={} warnings={}",
                        gate.pass,
                        gate.blocking.len(),
                        gate.warnings.len()
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
}

impl CfdRsMemory {
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
            "Read-only repository memory server. Start with `status_summary` for repo truth and \
             `phase5_priority` for the active queue. Use `parity_row_details` or `domain_gaps_ranked` for \
             parity work, `baseline_source_mapping` for frozen-source routing, and `crate_surface_summary` \
             or `crate_dependency_graph` before broad code scans. Use `get_context_snapshot`, \
             `get_context_bundle`, or `get_context_brief` for compact routing, and widen to direct file \
             reads only when the first MCP answer is insufficient. The required operational server surface \
             includes debtmap; use `debtmap_*` once the task narrows to hotspot, review, or refactor work."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

fn resolve_paths(
    repo_root: &Path,
    repo_root_canon: &Path,
    paths: &[String],
) -> Result<Vec<PathBuf>, (&'static str, String)> {
    let mut resolved = Vec::with_capacity(paths.len());
    for path in paths {
        match repo::resolve(repo_root, repo_root_canon, path.as_str()) {
            Ok(resolved_path) => resolved.push(resolved_path),
            Err(error) => return Err((error, path.clone())),
        }
    }
    Ok(resolved)
}

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

#[allow(dead_code)]
fn debtmap_unavailable(tool_name: &str) -> String {
    to_json(serde_json::json!({
        "error": "debtmap feature not enabled",
        "tool": tool_name,
        "hint": "this is the maintenance-only MCP surface; restart the server with debtmap enabled because the operational agent surface requires it"
    }))
}

#[cfg(test)]
mod tests {
    use super::{CORE_TOOL_NAMES, DEBTMAP_TOOL_NAMES};
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf())
            .expect("repo root")
    }

    fn bullet_list_from_doc(section_heading: &str) -> Vec<String> {
        let path = repo_root().join("docs/ai-context-routing.md");
        let text = fs::read_to_string(path).expect("routing doc");
        let mut capture = false;
        let mut items = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if !capture {
                if trimmed == section_heading {
                    capture = true;
                }
                continue;
            }

            if trimmed.starts_with("#") && trimmed != section_heading {
                break;
            }

            if let Some(item) = trimmed.strip_prefix("- `")
                && let Some(value) = item.strip_suffix('`')
            {
                items.push(value.to_string());
            }
        }

        items
    }

    #[test]
    fn core_tool_names_match_routing_doc() {
        let from_doc = bullet_list_from_doc("### Core tools");
        let from_code = CORE_TOOL_NAMES
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();

        assert_eq!(from_code, from_doc);
    }

    #[test]
    fn debtmap_tool_names_match_routing_doc() {
        let from_doc = bullet_list_from_doc("### Debtmap extension tools");
        let from_code = DEBTMAP_TOOL_NAMES
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();

        assert_eq!(from_code, from_doc);
    }
}
