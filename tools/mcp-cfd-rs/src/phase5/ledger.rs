use super::ParityRowRecord;
use super::helpers::{ledger_path_for_domain, map_row, read_repo_text, value_or_empty};
use std::path::Path;

pub(super) fn parse_ledger_rows(repo_root: &Path, domain: &str) -> Result<Vec<ParityRowRecord>, String> {
    let path = ledger_path_for_domain(domain);
    let text = read_repo_text(repo_root, path)?;
    let mut rows = Vec::new();
    let mut current_section = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim();

        if let Some(section) = line.strip_prefix("### ") {
            current_section = section.trim().to_string();
            index += 1;
            continue;
        }

        if line.starts_with('|') {
            let header = split_markdown_row(line);
            let is_id_table = header.first().is_some_and(|value| value == "ID");
            let has_divider = lines
                .get(index + 1)
                .map(|divider| is_markdown_divider(&split_markdown_row(divider.trim())))
                .unwrap_or(false);

            if is_id_table && has_divider {
                index += 2;
                parse_table_body(&lines, &header, domain, &current_section, &mut rows, &mut index);
                continue;
            }
        }

        index += 1;
    }

    Ok(rows)
}

fn parse_table_body(
    lines: &[&str],
    header: &[String],
    domain: &str,
    section: &str,
    rows: &mut Vec<ParityRowRecord>,
    index: &mut usize,
) {
    while *index < lines.len() {
        let row_line = lines[*index].trim();

        if !row_line.starts_with('|') {
            break;
        }

        let cells = split_markdown_row(row_line);
        if cells.len() != header.len() {
            break;
        }

        let row = map_row(header, &cells);
        let Some(row_id) = row.get("ID") else {
            *index += 1;
            continue;
        };

        if !row_id.starts_with(domain) {
            *index += 1;
            continue;
        }

        rows.push(ParityRowRecord {
            row_id: row_id.to_string(),
            domain: domain.to_string(),
            section: section.to_string(),
            feature_group: value_or_empty(&row, "Feature group"),
            baseline_source: value_or_empty(&row, "Baseline source"),
            baseline_behavior_or_contract: value_or_empty(&row, "Baseline behavior or contract"),
            rust_owner_now: value_or_empty(&row, "Rust owner now"),
            rust_status_now: value_or_empty(&row, "Rust status now"),
            parity_evidence_status: value_or_empty(&row, "Parity evidence status"),
            divergence_status: value_or_empty(&row, "Divergence status"),
            required_tests: value_or_empty(&row, "Required tests"),
            priority: value_or_empty(&row, "Priority"),
            notes: value_or_empty(&row, "Notes"),
        });

        *index += 1;
    }
}

fn split_markdown_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_markdown_divider(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}
