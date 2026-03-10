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

#[derive(Debug, Serialize)]
struct SearchHit {
    path: String,
    score: usize,
    snippet: String,
}

#[derive(Debug, Serialize)]
struct PathEntry {
    path: String,
    kind: &'static str,
    size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct FileMetadata {
    path: String,
    kind: &'static str,
    size_bytes: u64,
    line_count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct BundleEntry {
    path: String,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct ContextBundle {
    bundle: &'static str,
    summary: &'static str,
    entries: Vec<BundleEntry>,
}

#[derive(Debug, Serialize)]
struct ContextBrief {
    bundle: &'static str,
    summary: &'static str,
    first_path: String,
    next_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SnapshotFact {
    label: &'static str,
    value: &'static str,
}

#[derive(Debug, Serialize)]
struct ContextSnapshot {
    snapshot: &'static str,
    summary: &'static str,
    facts: Vec<SnapshotFact>,
    source_paths: Vec<String>,
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
        let base_path = base_path.unwrap_or_else(|| ".".to_string());
        let recursive = recursive.unwrap_or(false);
        let max_results = max_results.unwrap_or(100).clamp(1, 500) as usize;

        let base_path_canon =
            match resolve_repo_path(&self.repo_root, &self.repo_root_canon, base_path.as_str()) {
                Ok(path) => path,
                Err(error) => {
                    return to_json(serde_json::json!({
                        "error": error,
                        "path": base_path
                    }));
                }
            };

        let filter_extensions = normalize_extensions(extensions.as_deref());

        let entries = collect_paths(
            &self.repo_root,
            &base_path_canon,
            recursive,
            filter_extensions.as_ref(),
            max_results,
        )
        .await;

        to_json(entries)
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
        if paths.is_empty() {
            return to_json(serde_json::json!({
                "error": "paths must not be empty"
            }));
        }

        let mut roots = Vec::new();
        for path in paths {
            match resolve_repo_path(&self.repo_root, &self.repo_root_canon, path.as_str()) {
                Ok(root) => roots.push(root),
                Err(error) => {
                    return to_json(serde_json::json!({
                        "error": error,
                        "path": path
                    }));
                }
            }
        }

        let max_results = max_results.unwrap_or(5).clamp(1, 20) as usize;

        search_roots(&self.repo_root, &roots, &query, max_results).await
    }

    #[tool(description = "Return a curated narrow context bundle for a common repository question type.")]
    async fn get_context_bundle(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        match context_bundle(bundle.trim()) {
            Some(bundle) => to_json(bundle),
            None => to_json(serde_json::json!({
                "error": "unknown bundle",
                "supported_bundles": supported_bundle_names()
            })),
        }
    }

    #[tool(description = "Return a compact first-read brief for a curated repository context bundle.")]
    async fn get_context_brief(
        &self,
        Parameters(ContextBundleRequest { bundle }): Parameters<ContextBundleRequest>,
    ) -> String {
        match context_brief(bundle.trim()) {
            Some(brief) => to_json(brief),
            None => to_json(serde_json::json!({
                "error": "unknown bundle",
                "supported_bundles": supported_bundle_names()
            })),
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
        match context_snapshot(snapshot.trim()) {
            Some(snapshot) => to_json(snapshot),
            None => to_json(serde_json::json!({
                "error": "unknown snapshot",
                "supported_snapshots": supported_snapshot_names()
            })),
        }
    }

    #[tool(description = "Read a repo file with truncation and repo-boundary enforcement.")]
    async fn read_file(
        &self,
        Parameters(ReadFileRequest { path, max_chars }): Parameters<ReadFileRequest>,
    ) -> String {
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 12000) as usize;

        let candidate_canon = match resolve_repo_path(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(path) => path,
            Err(error) => {
                return to_json(serde_json::json!({
                    "error": error,
                    "path": path
                }));
            }
        };

        if !candidate_canon.is_file() {
            return to_json(serde_json::json!({
                "error": "path is not a regular file",
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
        let max_chars = max_chars.unwrap_or(4000).clamp(200, 16000) as usize;

        if start_line == 0 || end_line < start_line {
            return to_json(serde_json::json!({
                "error": "line range must be 1-based and end_line must be >= start_line",
                "path": path
            }));
        }

        let candidate_canon = match resolve_repo_path(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(path) => path,
            Err(error) => {
                return to_json(serde_json::json!({
                    "error": error,
                    "path": path
                }));
            }
        };

        if !candidate_canon.is_file() {
            return to_json(serde_json::json!({
                "error": "path is not a regular file",
                "path": path
            }));
        }

        match fs::read_to_string(&candidate_canon).await {
            Ok(text) => match slice_lines(text.as_str(), start_line as usize, end_line as usize, max_chars) {
                Ok((content, total_line_count, truncated)) => to_json(serde_json::json!({
                    "path": path,
                    "start_line": start_line,
                    "end_line": usize::min(end_line as usize, total_line_count),
                    "total_line_count": total_line_count,
                    "truncated": truncated,
                    "content": content
                })),
                Err(error) => to_json(serde_json::json!({
                    "error": error,
                    "path": path
                })),
            },
            Err(_) => to_json(serde_json::json!({
                "error": "file not readable as UTF-8 text",
                "path": path
            })),
        }
    }

    #[tool(description = "Return metadata for a repo path, including line count for readable text files.")]
    async fn file_metadata(
        &self,
        Parameters(FileMetadataRequest { path }): Parameters<FileMetadataRequest>,
    ) -> String {
        let candidate_canon = match resolve_repo_path(&self.repo_root, &self.repo_root_canon, &path) {
            Ok(path) => path,
            Err(error) => {
                return to_json(serde_json::json!({
                    "error": error,
                    "path": path
                }));
            }
        };

        let metadata = match fs::metadata(&candidate_canon).await {
            Ok(metadata) => metadata,
            Err(_) => {
                return to_json(serde_json::json!({
                    "error": "path not readable",
                    "path": path
                }));
            }
        };

        let kind = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        let line_count = if metadata.is_file() && is_text_file(&candidate_canon) {
            fs::read_to_string(&candidate_canon)
                .await
                .ok()
                .map(|text| text.lines().count())
        } else {
            None
        };

        let metadata = FileMetadata {
            path,
            kind,
            size_bytes: metadata.len(),
            line_count,
        };

        to_json(metadata)
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

async fn collect_paths(
    repo_root: &Path,
    base_path: &Path,
    recursive: bool,
    extensions: Option<&BTreeSet<String>>,
    max_results: usize,
) -> Vec<PathEntry> {
    let Ok(base_meta) = fs::symlink_metadata(base_path).await else {
        return Vec::new();
    };

    if !base_meta.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut stack = vec![base_path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        let mut children = Vec::new();
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let entry_path = entry.path();

            let Ok(entry_meta) = fs::symlink_metadata(&entry_path).await else {
                continue;
            };

            if entry_meta.file_type().is_symlink() {
                continue;
            }

            children.push((entry_path, entry_meta));
        }

        children.sort_by(|left, right| left.0.cmp(&right.0));

        for (entry_path, entry_meta) in children {
            if entry_meta.is_dir() {
                entries.push(PathEntry {
                    path: make_relative(repo_root, &entry_path),
                    kind: "directory",
                    size_bytes: None,
                });

                if recursive {
                    stack.push(entry_path);
                }
            } else if entry_meta.is_file() {
                if !path_matches_extensions(&entry_path, extensions) {
                    continue;
                }

                entries.push(PathEntry {
                    path: make_relative(repo_root, &entry_path),
                    kind: "file",
                    size_bytes: Some(entry_meta.len()),
                });
            }

            if entries.len() >= max_results {
                return entries;
            }
        }

        if !recursive {
            break;
        }
    }

    entries
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

fn normalize_extensions(extensions: Option<&[String]>) -> Option<BTreeSet<String>> {
    let mut normalized = BTreeSet::new();

    for extension in extensions.unwrap_or(&[]) {
        let extension = extension.trim().trim_start_matches('.').to_ascii_lowercase();
        if !extension.is_empty() {
            normalized.insert(extension);
        }
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn path_matches_extensions(path: &Path, extensions: Option<&BTreeSet<String>>) -> bool {
    let Some(extensions) = extensions else {
        return true;
    };

    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extensions.contains(&extension.to_ascii_lowercase()))
        .unwrap_or(false)
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

fn resolve_repo_path(repo_root: &Path, repo_root_canon: &Path, path: &str) -> Result<PathBuf, &'static str> {
    if invalid_repo_relative_path(path) {
        return Err("path must be repo-relative and must not escape the repo root");
    }

    let candidate = repo_root.join(path);
    let candidate_canon =
        std::fs::canonicalize(&candidate).map_err(|_| "file or directory not found or not readable")?;

    if !candidate_canon.starts_with(repo_root_canon) {
        return Err("path escapes repo root");
    }

    Ok(candidate_canon)
}

fn slice_lines(
    text: &str,
    start_line: usize,
    end_line: usize,
    max_chars: usize,
) -> Result<(String, usize, bool), &'static str> {
    let all_lines = text.lines().collect::<Vec<_>>();
    let total_line_count = all_lines.len();

    if total_line_count == 0 {
        return Ok((String::new(), 0, false));
    }

    if start_line > total_line_count {
        return Err("start_line is out of bounds for this file");
    }

    let end_line = usize::min(end_line, total_line_count);
    let content = all_lines[start_line - 1..end_line].join("\n");
    let truncated = content.chars().count() > max_chars;
    let content = content.chars().take(max_chars).collect();

    Ok((content, total_line_count, truncated))
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

fn supported_bundle_names() -> Vec<&'static str> {
    vec![
        "scope-lane",
        "repo-state",
        "active-surface",
        "first-slice-parity",
        "runtime-deps",
        "behavior-baseline",
    ]
}

fn context_bundle(bundle: &str) -> Option<ContextBundle> {
    match bundle {
        "scope-lane" => Some(ContextBundle {
            bundle: "scope-lane",
            summary: "Use this bundle for rewrite boundaries, current lane, and non-negotiable scope.",
            entries: vec![
                BundleEntry {
                    path: "REWRITE_CHARTER.md".to_string(),
                    reason: "Shortest source of non-negotiables, lane decisions, and scope boundaries.",
                },
                BundleEntry {
                    path: "docs/compatibility-scope.md".to_string(),
                    reason: "Defines what compatibility means and what it does not imply.",
                },
                BundleEntry {
                    path: "docs/promotion-gates.md".to_string(),
                    reason: "Gives the current phase model when phase boundaries matter to scope.",
                },
            ],
        }),
        "repo-state" => Some(ContextBundle {
            bundle: "repo-state",
            summary: "Use this bundle for what exists now in the workspace without loading the entire old \
                      status narrative.",
            entries: vec![
                BundleEntry {
                    path: "STATUS.md".to_string(),
                    reason: "Short status index and summary.",
                },
                BundleEntry {
                    path: "docs/status/rewrite-foundation.md".to_string(),
                    reason: "Current lane, workspace shape, and governance baseline.",
                },
                BundleEntry {
                    path: "docs/status/active-surface.md".to_string(),
                    reason: "Current executable surface and deferred scope.",
                },
            ],
        }),
        "active-surface" => Some(ContextBundle {
            bundle: "active-surface",
            summary: "Use this bundle for the currently admitted implementation surface and nearby deferred \
                      slices.",
            entries: vec![
                BundleEntry {
                    path: "STATUS.md".to_string(),
                    reason: "Short status index for current state entry.",
                },
                BundleEntry {
                    path: "docs/status/active-surface.md".to_string(),
                    reason: "Current Phase 3.3 surface and deferred Big Phase 3 slices.",
                },
                BundleEntry {
                    path: "docs/promotion-gates.md".to_string(),
                    reason: "Promotion boundaries when deciding whether a behavior belongs to the active \
                             slice.",
                },
            ],
        }),
        "first-slice-parity" => Some(ContextBundle {
            bundle: "first-slice-parity",
            summary: "Use this bundle for first-slice implementation history and parity-backed closure \
                      state.",
            entries: vec![
                BundleEntry {
                    path: "docs/first-slice-freeze.md".to_string(),
                    reason: "Frozen accepted first-slice scope.",
                },
                BundleEntry {
                    path: "docs/status/first-slice-parity.md".to_string(),
                    reason: "Current first-slice implementation and parity status by subphase.",
                },
                BundleEntry {
                    path: "tools/first_slice_parity.py".to_string(),
                    reason: "Parity harness entry point when implementation details matter.",
                },
            ],
        }),
        "runtime-deps" => Some(ContextBundle {
            bundle: "runtime-deps",
            summary: "Use this bundle for dependency admission, allocator rules, and runtime structure \
                      constraints.",
            entries: vec![
                BundleEntry {
                    path: "docs/dependency-policy.md".to_string(),
                    reason: "Dependency admission and workspace dependency truth.",
                },
                BundleEntry {
                    path: "docs/allocator-runtime-baseline.md".to_string(),
                    reason: "Allocator and runtime baseline.",
                },
                BundleEntry {
                    path: "docs/go-rust-semantic-mapping.md".to_string(),
                    reason: "Concurrency and lifecycle doctrine when runtime shape matters.",
                },
            ],
        }),
        "behavior-baseline" => Some(ContextBundle {
            bundle: "behavior-baseline",
            summary: "Use this bundle for behavior and parity questions against the frozen Go baseline.",
            entries: vec![
                BundleEntry {
                    path: "baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md".to_string(),
                    reason: "Topic-to-source map into the frozen Go tree.",
                },
                BundleEntry {
                    path: "baseline-2026.2.0/design-audit/REPO_REFERENCE.md".to_string(),
                    reason: "Broad repository reference for behavioral and contract questions.",
                },
                BundleEntry {
                    path: "baseline-2026.2.0/old-impl".to_string(),
                    reason: "Frozen behavioral source of truth; read the specific package or test after \
                             routing.",
                },
            ],
        }),
        _ => None,
    }
}

fn context_brief(bundle: &str) -> Option<ContextBrief> {
    let bundle = context_bundle(bundle)?;
    let mut paths = bundle.entries.into_iter().map(|entry| entry.path);
    let first_path = paths.next()?;
    let next_paths = paths.collect();

    Some(ContextBrief {
        bundle: bundle.bundle,
        summary: bundle.summary,
        first_path,
        next_paths,
    })
}

fn supported_snapshot_names() -> Vec<&'static str> {
    vec![
        "governing-files",
        "scope-lane",
        "repo-state",
        "active-phase",
        "runtime-deps",
        "behavior-baseline",
        "lane-decisions",
    ]
}

fn context_snapshot(snapshot: &str) -> Option<ContextSnapshot> {
    match snapshot {
        "governing-files" => Some(ContextSnapshot {
            snapshot: "governing-files",
            summary: "Compact answer for which repository file owns the main governance topic categories.",
            facts: vec![
                SnapshotFact {
                    label: "scope_and_lane",
                    value: "Use REWRITE_CHARTER.md for non-negotiables, active lane, and scope boundaries; \
                            use docs/compatibility-scope.md when the meaning of compatibility needs detail.",
                },
                SnapshotFact {
                    label: "current_state_and_phase",
                    value: "Use STATUS.md as the short current-state index, docs/status/* for focused \
                            current-state detail, and docs/promotion-gates.md for active phase and \
                            promotion boundaries.",
                },
                SnapshotFact {
                    label: "dependencies_and_runtime",
                    value: "Use docs/dependency-policy.md for dependency admission and workspace dependency \
                            truth, and docs/allocator-runtime-baseline.md plus \
                            docs/go-rust-semantic-mapping.md for runtime and lifecycle doctrine.",
                },
                SnapshotFact {
                    label: "behavior_and_parity",
                    value: "Use baseline-2026.2.0/old-impl first for behavior truth, then \
                            baseline-2026.2.0/design-audit for topic maps and broader parity reference.",
                },
            ],
            source_paths: vec![
                "REWRITE_CHARTER.md".to_string(),
                "STATUS.md".to_string(),
                "docs/promotion-gates.md".to_string(),
                "docs/dependency-policy.md".to_string(),
                "baseline-2026.2.0/old-impl".to_string(),
            ],
        }),
        "scope-lane" => Some(ContextSnapshot {
            snapshot: "scope-lane",
            summary: "Compact answer for rewrite boundaries, compatibility lane, and governing scope files.",
            facts: vec![
                SnapshotFact {
                    label: "scope_owner",
                    value: "REWRITE_CHARTER.md owns non-negotiables, lane decisions, and scope boundaries.",
                },
                SnapshotFact {
                    label: "compatibility_meaning",
                    value: "Compatibility is bounded by the frozen rewrite lane and must not be widened by \
                            implication from later slices or broader platform assumptions.",
                },
                SnapshotFact {
                    label: "phase_boundary_owner",
                    value: "Promotion and current phase truth belong to docs/promotion-gates.md rather than \
                            branch names, draft plans, or workflow notes.",
                },
            ],
            source_paths: vec![
                "REWRITE_CHARTER.md".to_string(),
                "docs/compatibility-scope.md".to_string(),
                "docs/promotion-gates.md".to_string(),
            ],
        }),
        "repo-state" => Some(ContextSnapshot {
            snapshot: "repo-state",
            summary: "Compact answer for what exists now versus what remains explicitly unimplemented.",
            facts: vec![
                SnapshotFact {
                    label: "workspace_state",
                    value: "The workspace is real but partial, not a blank scaffold and not a \
                            parity-complete rewrite.",
                },
                SnapshotFact {
                    label: "implemented_now",
                    value: "Accepted first-slice config, credentials, and ingress behavior exists in \
                            crates/cloudflared-config, and a narrow Phase 3.3 QUIC tunnel core exists in \
                            crates/cloudflared-cli.",
                },
                SnapshotFact {
                    label: "explicitly_missing",
                    value: "Pingora integration, later wire or protocol slices, security or compliance \
                            operational behavior, and broader platform scope are not implemented yet.",
                },
            ],
            source_paths: vec![
                "STATUS.md".to_string(),
                "docs/status/rewrite-foundation.md".to_string(),
                "docs/status/active-surface.md".to_string(),
            ],
        }),
        "active-phase" => Some(ContextSnapshot {
            snapshot: "active-phase",
            summary: "Compact answer for the current implementation phase and what it must not imply.",
            facts: vec![
                SnapshotFact {
                    label: "current_big_phase",
                    value: "Big Phase 3 is current.",
                },
                SnapshotFact {
                    label: "active_task",
                    value: "Phase 3.3 owns the QUIC tunnel core for the frozen Linux production-alpha lane.",
                },
                SnapshotFact {
                    label: "deferred_next_slices",
                    value: "Phase 3.4 Pingora integration, Phase 3.5 wire or protocol, Phase 3.6 security \
                            or compliance operations, and Phase 3.7 standard-format integration remain \
                            deferred.",
                },
                SnapshotFact {
                    label: "must_not_imply",
                    value: "Current Phase 3.3 work must not imply that Pingora, later wire behavior, \
                            compliance operations, or broader packaging and deployment tooling already \
                            exist.",
                },
            ],
            source_paths: vec![
                "STATUS.md".to_string(),
                "docs/status/active-surface.md".to_string(),
                "docs/promotion-gates.md".to_string(),
            ],
        }),
        "runtime-deps" => Some(ContextSnapshot {
            snapshot: "runtime-deps",
            summary: "Compact answer for dependency admission, workspace dependency truth, and runtime \
                      baseline constraints.",
            facts: vec![
                SnapshotFact {
                    label: "dependency_default",
                    value: "Normal workspace-managed third-party dependency truth should normally live in \
                            [workspace.dependencies], with crate-local declarations reserved for explicit \
                            isolation cases.",
                },
                SnapshotFact {
                    label: "admission_rule",
                    value: "Dependencies are admitted only for active owning slices and must not silently \
                            redesign externally visible behavior or preload later-slice scope.",
                },
                SnapshotFact {
                    label: "runtime_guardrail",
                    value: "Runtime and allocator shape remain governed by the accepted baseline; later \
                            async, transport, or observability dependencies do not become defaults before \
                            their owning slices start.",
                },
            ],
            source_paths: vec![
                "docs/dependency-policy.md".to_string(),
                "docs/allocator-runtime-baseline.md".to_string(),
                "docs/go-rust-semantic-mapping.md".to_string(),
            ],
        }),
        "behavior-baseline" => Some(ContextSnapshot {
            snapshot: "behavior-baseline",
            summary: "Compact answer for where behavior and parity truth must be sourced before making \
                      claims about rewrite correctness.",
            facts: vec![
                SnapshotFact {
                    label: "first_truth_source",
                    value: "Behavior and parity questions start with baseline-2026.2.0/old-impl code and \
                            tests, not the Rust rewrite shape.",
                },
                SnapshotFact {
                    label: "second_truth_source",
                    value: "Use baseline-2026.2.0/design-audit after the frozen source tree when a topic \
                            map, reference index, or broader audit summary is needed.",
                },
                SnapshotFact {
                    label: "claim_guardrail",
                    value: "Do not claim parity from Rust code alone; route to the frozen baseline first \
                            and widen only after the baseline path is known.",
                },
            ],
            source_paths: vec![
                "baseline-2026.2.0/old-impl".to_string(),
                "baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md".to_string(),
                "baseline-2026.2.0/design-audit/REPO_REFERENCE.md".to_string(),
            ],
        }),
        "lane-decisions" => Some(ContextSnapshot {
            snapshot: "lane-decisions",
            summary: "Compact answer for the frozen alpha-lane decisions across transport, Pingora, FIPS, \
                      and deployment.",
            facts: vec![
                SnapshotFact {
                    label: "transport_lane",
                    value: "The frozen alpha lane requires 0-RTT, uses quiche first, and chooses quiche \
                            plus BoringSSL rather than quiche plus OpenSSL.",
                },
                SnapshotFact {
                    label: "pingora_role",
                    value: "Pingora is in the production-alpha critical path above the quiche transport \
                            lane as the initial application-layer proxy path, not as the transport owner.",
                },
                SnapshotFact {
                    label: "fips_boundary",
                    value: "FIPS-in-alpha is a bounded governance commitment around the admitted crypto \
                            surface and explicit build or link boundary, not proof of working \
                            implementation or certification.",
                },
                SnapshotFact {
                    label: "deployment_contract",
                    value: "The deployment contract is Linux only on x86_64-unknown-linux-gnu with GNU or \
                            glibc artifacts, a supervised host service model, and a bare-metal-first stance.",
                },
            ],
            source_paths: vec![
                "docs/adr/0002-transport-tls-crypto-lane.md".to_string(),
                "docs/adr/0003-pingora-critical-path.md".to_string(),
                "docs/adr/0004-fips-in-alpha-definition.md".to_string(),
                "docs/adr/0005-deployment-contract.md".to_string(),
            ],
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        context_brief, context_bundle, context_snapshot, invalid_repo_relative_path, normalize_extensions,
        path_matches_extensions, slice_lines, supported_bundle_names, supported_snapshot_names,
    };
    use std::{collections::BTreeSet, path::Path};

    #[test]
    fn rejects_absolute_and_parent_paths() {
        assert!(invalid_repo_relative_path("/tmp/file"));
        assert!(invalid_repo_relative_path("../file"));
        assert!(invalid_repo_relative_path("docs/../../file"));
        assert!(!invalid_repo_relative_path("docs/file.md"));
    }

    #[test]
    fn normalizes_extension_filters() {
        let normalized = normalize_extensions(Some(&[".MD".to_string(), "rs".to_string(), "  ".to_string()]))
            .expect("extensions should be present");

        assert_eq!(normalized, BTreeSet::from(["md".to_string(), "rs".to_string()]));
    }

    #[test]
    fn matches_extensions_case_insensitively() {
        let extensions = BTreeSet::from(["md".to_string()]);

        assert!(path_matches_extensions(
            Path::new("docs/README.MD"),
            Some(&extensions)
        ));
        assert!(!path_matches_extensions(
            Path::new("src/main.rs"),
            Some(&extensions)
        ));
    }

    #[test]
    fn slices_requested_line_range() {
        let text = "one\ntwo\nthree\nfour\n";
        let (content, total_line_count, truncated) =
            slice_lines(text, 2, 3, 100).expect("line slice should succeed");

        assert_eq!(content, "two\nthree");
        assert_eq!(total_line_count, 4);
        assert!(!truncated);
    }

    #[test]
    fn rejects_out_of_bounds_line_start() {
        let error = slice_lines("one\ntwo\n", 4, 4, 100).expect_err("slice should fail");

        assert_eq!(error, "start_line is out of bounds for this file");
    }

    #[test]
    fn exposes_known_context_bundle() {
        let bundle = context_bundle("repo-state").expect("bundle should exist");

        assert_eq!(bundle.bundle, "repo-state");
        assert_eq!(bundle.entries.len(), 3);
    }

    #[test]
    fn exposes_compact_context_brief() {
        let brief = context_brief("repo-state").expect("brief should exist");

        assert_eq!(brief.bundle, "repo-state");
        assert_eq!(brief.first_path, "STATUS.md");
        assert_eq!(brief.next_paths.len(), 2);
    }

    #[test]
    fn exposes_repo_state_snapshot() {
        let snapshot = context_snapshot("repo-state").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "repo-state");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(snapshot.source_paths.contains(&"STATUS.md".to_string()));
    }

    #[test]
    fn exposes_runtime_dependency_snapshot() {
        let snapshot = context_snapshot("runtime-deps").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "runtime-deps");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(
            snapshot
                .source_paths
                .contains(&"docs/dependency-policy.md".to_string())
        );
    }

    #[test]
    fn exposes_behavior_baseline_snapshot() {
        let snapshot = context_snapshot("behavior-baseline").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "behavior-baseline");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(
            snapshot
                .source_paths
                .contains(&"baseline-2026.2.0/old-impl".to_string())
        );
    }

    #[test]
    fn exposes_lane_decision_snapshot() {
        let snapshot = context_snapshot("lane-decisions").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "lane-decisions");
        assert_eq!(snapshot.facts.len(), 4);
        assert!(
            snapshot
                .source_paths
                .contains(&"docs/adr/0002-transport-tls-crypto-lane.md".to_string())
        );
    }

    #[test]
    fn exposes_governing_file_snapshot() {
        let snapshot = context_snapshot("governing-files").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "governing-files");
        assert_eq!(snapshot.facts.len(), 4);
        assert!(snapshot.source_paths.contains(&"REWRITE_CHARTER.md".to_string()));
    }

    #[test]
    fn advertises_supported_snapshot_names() {
        let supported = supported_snapshot_names();

        assert!(supported.contains(&"scope-lane"));
        assert!(supported.contains(&"repo-state"));
        assert!(supported.contains(&"active-phase"));
        assert!(supported.contains(&"runtime-deps"));
        assert!(supported.contains(&"behavior-baseline"));
        assert!(supported.contains(&"lane-decisions"));
        assert!(supported.contains(&"governing-files"));
    }

    #[test]
    fn advertises_supported_bundle_names() {
        let supported = supported_bundle_names();

        assert!(supported.contains(&"scope-lane"));
        assert!(supported.contains(&"behavior-baseline"));
    }
}
