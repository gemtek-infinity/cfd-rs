use super::text::parse_sections;
use super::{MilestoneRecord, ROADMAP_INDEX_PATH, RoadmapIndexEntry};
use crate::phase5::helpers::read_repo_text;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn parse_milestones(text: &str) -> Vec<MilestoneRecord> {
    parse_sections(text, "### ")
        .into_iter()
        .map(|(title, content)| MilestoneRecord {
            name: normalize_milestone_heading(&title),
            content,
        })
        .collect()
}

pub(super) fn parse_roadmap_index(repo_root: &Path) -> Result<HashMap<String, RoadmapIndexEntry>, String> {
    let text = read_repo_text(repo_root, ROADMAP_INDEX_PATH)?;
    let mut lines = text.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| "roadmap-index.csv is empty".to_string())?;
    let header = split_csv_line(header_line)?;

    if header
        != [
            "row_id",
            "domain",
            "milestone",
            "owner_crate",
            "status_bucket",
            "blocked_by",
            "evidence_ref",
        ]
    {
        return Err("roadmap-index.csv header does not match expected schema".to_string());
    }

    let mut entries = HashMap::new();

    for line in lines.filter(|line| !line.trim().is_empty()) {
        let columns = split_csv_line(line)?;

        if columns.len() != 7 {
            return Err(format!("roadmap-index.csv row does not have 7 columns: {line}"));
        }

        let entry = RoadmapIndexEntry {
            row_id: columns[0].clone(),
            milestone: columns[2].clone(),
            owner_crate: columns[3].clone(),
            status_bucket: columns[4].clone(),
            blocked_by: columns[5].clone(),
            evidence_ref: columns[6].clone(),
        };

        entries.insert(entry.row_id.clone(), entry);
    }

    Ok(entries)
}

pub(super) fn rows_for_milestone(index: &HashMap<String, RoadmapIndexEntry>, milestone: &str) -> Vec<String> {
    let mut row_ids: Vec<String> = index
        .values()
        .filter(|entry| entry.milestone == milestone)
        .map(|entry| entry.row_id.clone())
        .collect();
    row_ids.sort();
    row_ids
}

fn normalize_milestone_heading(raw: &str) -> String {
    raw.split_once(". ")
        .map(|(_, remainder)| remainder.trim().to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

fn split_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                cells.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(format!("unterminated quoted csv field: {line}"));
    }

    cells.push(current.trim().to_string());
    Ok(cells)
}
