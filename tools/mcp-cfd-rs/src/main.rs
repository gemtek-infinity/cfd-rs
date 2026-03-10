use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
use tokio::fs;

#[derive(Debug, Clone)]
struct CfdRsMemory {
    repo_root: PathBuf,
    repo_root_canon: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchRequest {
    query: String,
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileRequest {
    path: String,
    max_chars: Option<u32>,
}

#[derive(Debug, Serialize)]
struct SearchHit {
    path: String,
    score: usize,
    snippet: String,
}

#[tool_router]
impl CfdRsMemory {
    fn new(repo_root: PathBuf, repo_root_canon: PathBuf) -> Self {
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
        let max_results = max_results.unwrap_or(5).clamp(1, 10) as usize;

        let roots = vec![
            self.repo_root.join("REWRITE_CHARTER.md"),
            self.repo_root.join("STATUS.md"),
            self.repo_root.join("AGENTS.md"),
            self.repo_root.join("SKILLS.md"),
            self.repo_root.join("docs"),
            self.repo_root.join(".github"),
        ];

        search_roots(&self.repo_root, &roots, &query, max_results).await
    }

    #[tool(description = "Search frozen behavior/parity sources, returning small grounded hits.")]
    async fn find_behavior_truth(
        &self,
        Parameters(SearchRequest { query, max_results }): Parameters<SearchRequest>,
    ) -> String {
        let max_results = max_results.unwrap_or(5).clamp(1, 10) as usize;

        let roots = vec![
            self.repo_root.join("baseline-2026.2.0/design-audit"),
            self.repo_root.join("baseline-2026.2.0/old-impl"),
        ];

        search_roots(&self.repo_root, &roots, &query, max_results).await
    }

    #[tool(description = "Read a repo file with truncation and repo-boundary enforcement.")]
    async fn read_file(
        &self,
        Parameters(ReadFileRequest { path, max_chars }): Parameters<ReadFileRequest>,
    ) -> String {
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 12000) as usize;

        if invalid_repo_relative_path(&path) {
            return to_json(serde_json::json!({
                "error": "path must be repo-relative and must not escape the repo root"
            }));
        }

        let candidate = self.repo_root.join(&path);

        let candidate_canon = match std::fs::canonicalize(&candidate) {
            Ok(p) => p,
            Err(_) => {
                return to_json(serde_json::json!({
                    "error": "file not found or not readable",
                    "path": path
                }));
            }
        };

        if !candidate_canon.starts_with(&self.repo_root_canon) || !candidate_canon.is_file() {
            return to_json(serde_json::json!({
                "error": "path escapes repo root or is not a regular file",
                "path": path
            }));
        }

        match fs::read_to_string(&candidate_canon).await {
            Ok(text) => {
                let content: String = text.chars().take(max_chars).collect();
                let truncated = text.chars().count() > max_chars;

                to_json(serde_json::json!({
                    "path": path,
                    "truncated": truncated,
                    "content": content
                }))
            }
            Err(_) => to_json(serde_json::json!({
                "error": "file not readable as UTF-8 text",
                "path": path
            })),
        }
    }
}

#[tool_handler]
impl ServerHandler for CfdRsMemory {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Read-only repository memory server. Use its tools to retrieve small grounded slices before \
             making claims."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::current_dir()?;
    let repo_root_canon = std::fs::canonicalize(&repo_root)?;

    let server = CfdRsMemory::new(repo_root, repo_root_canon)
        .serve(stdio())
        .await?;

    server.waiting().await?;
    Ok(())
}

async fn search_roots(repo_root: &Path, roots: &[PathBuf], query: &str, max_results: usize) -> String {
    let terms = normalize_terms(query);
    if terms.is_empty() {
        return to_json(serde_json::json!({
            "error": "query must not be empty"
        }));
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
                path: make_relative(repo_root, &path),
                score,
                snippet: make_snippet(&text, query, 320),
            });
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(max_results);

    to_json(hits)
}

async fn collect_text_files(path: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(path).await else {
        return;
    };

    let file_type = meta.file_type();
    if file_type.is_symlink() {
        return;
    }

    if meta.is_file() {
        if is_text_file(path) && meta.len() <= 512_000 {
            out.insert(path.to_path_buf());
        }
        return;
    }

    if !meta.is_dir() {
        return;
    }

    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let entry_path = entry.path();

            let Ok(entry_meta) = fs::symlink_metadata(&entry_path).await else {
                continue;
            };

            let entry_type = entry_meta.file_type();
            if entry_type.is_symlink() {
                continue;
            }

            if entry_meta.is_dir() {
                stack.push(entry_path);
            } else if entry_meta.is_file() && entry_meta.len() <= 512_000 && is_text_file(&entry_path) {
                out.insert(entry_path);
            }
        }
    }
}

fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("md" | "txt" | "rs" | "toml" | "yaml" | "yml" | "json" | "go" | "sh" | "py" | "sql")
    )
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
    let flat_lower = flat.to_lowercase();
    let needle = query.trim().to_lowercase();

    if needle.is_empty() {
        return flat.chars().take(limit).collect();
    }

    if let Some(byte_idx) = flat_lower.find(&needle) {
        let match_char_idx = flat_lower[..byte_idx].chars().count();
        let needle_chars = needle.chars().count();
        let total_chars = flat.chars().count();

        let start = match_char_idx.saturating_sub(limit / 2);
        let end = usize::min(total_chars, match_char_idx + needle_chars + (limit / 2));

        return flat.chars().skip(start).take(end - start).collect();
    }

    flat.chars().take(limit).collect()
}

fn make_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn invalid_repo_relative_path(path: &str) -> bool {
    let p = Path::new(path);

    p.is_absolute()
        || p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn to_json<T: Serialize>(value: T) -> String {
    serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
}
