use super::ParityRowRecord;
pub const CLOSED_RUST_STATUSES: &[&str] = &["audited, parity-backed", "audited, intentional divergence"];
pub const PARTIAL_RUST_STATUSES: &[&str] = &["audited, partial"];
pub const NOT_AUDITED_RUST_STATUSES: &[&str] = &["not audited"];
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
    is_closed_status(&row.rust_status_now)
}

pub(super) fn is_closed_status(status: &str) -> bool {
    CLOSED_RUST_STATUSES.contains(&status.trim())
}

pub(super) fn is_partial_status(status: &str) -> bool {
    PARTIAL_RUST_STATUSES.contains(&status.trim())
}

pub(super) fn is_not_audited_status(status: &str) -> bool {
    NOT_AUDITED_RUST_STATUSES.contains(&status.trim())
}

pub(super) fn is_row_id(value: &str) -> bool {
    let normalized = value.trim();
    normalized.len() == 7
        && matches!(&normalized[0..3], "CLI" | "CDC" | "HIS")
        && normalized.as_bytes().get(3) == Some(&b'-')
        && normalized[4..].chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{is_closed_status, is_not_audited_status, is_partial_status};

    #[test]
    fn closed_statuses_match_canonical_vocabulary() {
        assert!(is_closed_status("audited, parity-backed"));
        assert!(is_closed_status("audited, intentional divergence"));
        assert!(!is_closed_status("audited, partial"));
        assert!(!is_closed_status("not audited"));
    }

    #[test]
    fn partial_and_not_audited_statuses_are_exact() {
        assert!(is_partial_status("audited, partial"));
        assert!(!is_partial_status("blocked"));
        assert!(is_not_audited_status("not audited"));
        assert!(!is_not_audited_status("audited, absent"));
    }
}
