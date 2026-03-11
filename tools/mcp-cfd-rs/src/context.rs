use serde::Serialize;
use std::path::Path;
use tokio::fs;

pub const ACTIVE_CONTEXT_PATH: &str = "docs/ACTIVE_CONTEXT.md";

#[derive(Debug, Serialize)]
pub struct BundleEntry {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ContextBundle {
    pub bundle: &'static str,
    pub summary: &'static str,
    pub entries: Vec<BundleEntry>,
}

#[derive(Debug, Serialize)]
pub struct ContextBrief {
    pub bundle: &'static str,
    pub summary: &'static str,
    pub first_path: String,
    pub next_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotFact {
    pub label: &'static str,
    pub value: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ContextSnapshot {
    pub snapshot: &'static str,
    pub summary: &'static str,
    pub facts: Vec<SnapshotFact>,
    pub source_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ActiveContextView {
    pub found: bool,
    pub source_path: &'static str,
    pub source: &'static str,
    pub max_chars: usize,
    pub truncated: bool,
    pub content: Option<String>,
    pub message: &'static str,
    pub next_steps: Vec<&'static str>,
}

pub async fn load_active_context(repo_root: &Path, max_chars: usize) -> ActiveContextView {
    let max_chars = max_chars.clamp(200, 12000);
    let path = repo_root.join(ACTIVE_CONTEXT_PATH);

    match fs::read_to_string(path).await {
        Ok(text) => {
            let content: String = text.chars().take(max_chars).collect();
            let truncated = text.chars().count() > max_chars;

            ActiveContextView {
                found: true,
                source_path: ACTIVE_CONTEXT_PATH,
                source: "file",
                max_chars,
                truncated,
                content: Some(content),
                message: "active context loaded from docs/ACTIVE_CONTEXT.md",
                next_steps: vec![
                    "Read docs/promotion-gates.md for broader phase governance if needed.",
                    "Update docs/ACTIVE_CONTEXT.md when active scope changes.",
                ],
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ActiveContextView {
            found: false,
            source_path: ACTIVE_CONTEXT_PATH,
            source: "missing",
            max_chars,
            truncated: false,
            content: None,
            message: "no active context file found",
            next_steps: vec![
                "Create docs/ACTIVE_CONTEXT.md to define current active context.",
                "Read docs/promotion-gates.md and STATUS.md directly.",
            ],
        },
        Err(_) => ActiveContextView {
            found: false,
            source_path: ACTIVE_CONTEXT_PATH,
            source: "error",
            max_chars,
            truncated: false,
            content: None,
            message: "active context file is not readable",
            next_steps: vec![
                "Fix file permissions or encoding for docs/ACTIVE_CONTEXT.md.",
                "Read docs/promotion-gates.md and STATUS.md directly.",
            ],
        },
    }
}
