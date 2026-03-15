use super::{
    CfdRsMemory, GrepPathsRequest, ListPathsRequest, Parameters, SearchPathsRequest, SearchRequest, fs, log,
    path_error, profile, repo, search, to_json,
};
use rmcp::{tool, tool_router};

#[tool_router(router = search_tools_router, vis = "pub")]
impl CfdRsMemory {
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

        let roots = match super::resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
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

        let roots = match super::resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
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
}
