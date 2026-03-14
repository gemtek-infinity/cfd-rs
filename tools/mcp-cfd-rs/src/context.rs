use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BundleEntry {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextBundle {
    pub bundle: &'static str,
    pub summary: &'static str,
    pub entries: Vec<BundleEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextBrief {
    pub bundle: &'static str,
    pub summary: &'static str,
    pub first_path: String,
    pub next_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotFact {
    pub label: &'static str,
    pub value: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSnapshot {
    pub snapshot: &'static str,
    pub summary: &'static str,
    pub facts: Vec<SnapshotFact>,
    pub source_paths: Vec<String>,
}
