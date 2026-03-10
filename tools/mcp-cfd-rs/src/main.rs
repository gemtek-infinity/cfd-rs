use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone)]
struct CfdRsMemory {
    repo_root: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FindTruthRequest {
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
    fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Return a compact snapshot of rewrite governance and the active lane.")]
    async fn governance_snapshot(&self) -> String {
        to_json(serde_json::json!({
            "objective": "Build a production-grade, parity-backed, Linux-first Rust rewrite.",
            "compatibility_baseline": {
                "behavioral_baseline": "baseline-2026.2.0/old-impl/",
                "derived_reference_layer": "baseline-2026.2.0/design-audit/",
                "target_release_baseline": "2026.2.0",
                "workspace_version": "2026.2.0-alpha.202603"
            },
            "active_lane": {
                "platform": "x86_64-unknown-linux-gnu",
                "shipped_gnu_artifacts": ["x86-64-v2", "x86-64-v4"],
                "zero_rtt_required": true,
                "transport_priority": "quiche first",
                "tls_lane": "quiche + BoringSSL",
                "pingora_in_critical_path": true,
                "fips_in_alpha_lane": true
            },
            "first_accepted_slice": [
                "config discovery/loading/normalization",
                "credentials surface",
                "ingress normalization/ordering/defaulting"
            ],
            "routing": {
                "behavior_and_parity": [
                    "baseline-2026.2.0/old-impl/ code and tests",
                    "baseline-2026.2.0/design-audit/"
                ],
                "non_negotiables": "REWRITE_CHARTER.md",
                "current_state": "STATUS.md",
                "policy": "docs/*.md",
                "workflow": ["AGENTS.md", "SKILLS.md"]
            }
        }))
    }

    #[tool(description = "Search repo truth sources and return small grounded hits.")]
    async fn find_truth(
        &self,
        Parameters(FindTruthRequest { query, max_results }): Parameters<FindTruthRequest>,
    ) -> String {
        let max_results = max_results.unwrap_or(5).clamp(1, 10) as usize;
        let terms = normalize_terms(&query);

        let search_roots = vec![
            self.repo_root.join("REWRITE_CHARTER.md"),
            self.repo_root.join("STATUS.md"),
            self.repo_root.join("AGENTS.md"),
            self.repo_root.join("SKILLS.md"),
            self.repo_root.join("docs"),
            self.repo_root.join(".github"),
            self.repo_root.join("crates"),
            self.repo_root.join("baseline-2026.2.0/design-audit"),
            self.repo_root.join("baseline-2026.2.0/old-impl"),
        ];

        let mut files = Vec::new();
        for root in search_roots {
            collect_text_files(&root, &mut files).await;
        }

        let mut hits = Vec::new();

        for path in files {
            if let Ok(text) = fs::read_to_string(&path).await {
                let score = score_text(&text, &terms);
                if score == 0 {
                    continue;
                }

                hits.push(SearchHit {
                    path: make_relative(&self.repo_root, &path),
                    score,
                    snippet: make_snippet(&text, &query, 320),
                });
            }
        }

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        hits.truncate(max_results);

        to_json(hits)
    }

    #[tool(description = "Read a repo file with truncation to avoid context bloat.")]
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

        match fs::read_to_string(&candidate).await {
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
                "error": "file not found or not readable",
                "path": path
            })),
        }
    }
}

#[tool_handler]
impl ServerHandler for CfdRsMemory {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::current_dir()?;
    let server = CfdRsMemory::new(repo_root).serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}

fn to_json<T: Serialize>(value: T) -> String {
    serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
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
    let flat = text.replace('\n', " ");
    let lower = flat.to_lowercase();
    let needle = query.to_lowercase();

    if let Some(idx) = lower.find(&needle) {
        let start = idx.saturating_sub(limit / 2);
        let end = usize::min(flat.len(), idx + needle.len() + (limit / 2));

        if let Some(slice) = flat.get(start..end) {
            return slice.to_string();
        }
    }

    flat.chars().take(limit).collect()
}

async fn collect_text_files(path: &Path, out: &mut Vec<PathBuf>) {
    let Ok(meta) = fs::metadata(path).await else {
        return;
    };

    if meta.is_file() {
        if is_text_file(path) && meta.len() <= 512_000 {
            out.push(path.to_path_buf());
        }
        return;
    }

    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let Ok(meta) = entry.metadata().await else {
                continue;
            };

            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() && meta.len() <= 512_000 && is_text_file(&path) {
                out.push(path);
            }
        }
    }
}

fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("md" | "txt" | "rs" | "toml" | "yaml" | "yml" | "json" | "go" | "sh" | "py" | "sql")
    )
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
