mod context_tools;
mod debtmap_tools;
mod phase5_tools;
mod search_tools;

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
    limit: Option<u32>,
    path_prefix: Option<String>,
}

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

#[tool_router(router = local_tools_router)]
impl CfdRsMemory {
    pub fn new(repo_root: PathBuf, repo_root_canon: PathBuf) -> Self {
        Self {
            repo_root,
            repo_root_canon,
            tool_router: Self::local_tools_router()
                + Self::search_tools_router()
                + Self::context_tools_router()
                + Self::phase5_tools_router()
                + Self::debtmap_tools_router(),
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
            "Read-only repository memory server. Start with `status_summary` for repo truth, per-domain \
             parity progress, and the priority queue. Use `domain_gaps_ranked` for bounded ranked work \
             inside one domain with partial vs absent breakdown. Use `parity_row_details` for one exact \
             row, `baseline_source_mapping` for frozen-source routing, and `crate_surface_summary` or \
             `crate_dependency_graph` before broad code scans. Use `get_context_snapshot`, \
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
