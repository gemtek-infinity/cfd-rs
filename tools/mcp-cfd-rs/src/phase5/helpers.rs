use super::ParityRowRecord;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) fn read_repo_text(repo_root: &Path, relative_path: &str) -> Result<String, String> {
    let path = repo_root.join(relative_path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {relative_path}: {error}"))
}

pub(super) fn map_row(header: &[String], cells: &[String]) -> HashMap<String, String> {
    header
        .iter()
        .cloned()
        .zip(cells.iter().cloned())
        .collect::<HashMap<_, _>>()
}

pub(super) fn value_or_empty(row: &HashMap<String, String>, key: &str) -> String {
    row.get(key).cloned().unwrap_or_default()
}

pub(super) fn normalize_row_id(row_id: &str) -> String {
    row_id.trim().to_uppercase()
}

pub(super) fn domain_for_row(row_id: &str) -> Result<String, String> {
    if row_id.starts_with("CLI-") {
        return Ok("CLI".to_string());
    }

    if row_id.starts_with("CDC-") {
        return Ok("CDC".to_string());
    }

    if row_id.starts_with("HIS-") {
        return Ok("HIS".to_string());
    }

    Err(format!("unknown row id domain: {row_id}"))
}

pub(super) fn normalize_domain(domain: &str) -> Result<String, String> {
    let normalized = domain.trim().to_uppercase();

    if matches!(normalized.as_str(), "CLI" | "CDC" | "HIS") {
        return Ok(normalized);
    }

    Err(format!("unsupported domain: {domain}"))
}

pub(super) fn ledger_path_for_domain(domain: &str) -> &'static str {
    match domain {
        "CLI" => "docs/parity/cli/implementation-checklist.md",
        "CDC" => "docs/parity/cdc/implementation-checklist.md",
        "HIS" => "docs/parity/his/implementation-checklist.md",
        _ => "docs/parity/README.md",
    }
}

pub(super) fn collect_source_paths(paths: &[String]) -> Vec<String> {
    let mut unique = Vec::new();

    for path in paths {
        if !unique.contains(path) {
            unique.push(path.clone());
        }
    }

    unique
}

pub(super) fn is_closed_row(row: &ParityRowRecord) -> bool {
    matches!(
        row.rust_status_now.as_str(),
        "audited, parity-backed" | "audited, intentional divergence"
    )
}

pub(super) fn is_partial_status(status: &str) -> bool {
    status.contains("partial") || status.contains("minimal")
}
