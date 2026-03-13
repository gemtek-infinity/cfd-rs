use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const STATUS_PATH: &str = "STATUS.md";
pub const ROADMAP_PATH: &str = "docs/phase-5/roadmap.md";
pub const ROADMAP_INDEX_PATH: &str = "docs/phase-5/roadmap-index.csv";
pub const SOURCE_MAP_PATH: &str = "docs/parity/source-map.csv";

const MILESTONE_ORDER: &[&str] = &[
    "Program Reset",
    "CDC Contract Foundation",
    "Host and Runtime Foundation",
    "CLI Foundation",
    "Command Family Closure",
    "Proof Closure",
    "Performance Architecture Overhaul",
];

#[derive(Debug, Clone, Serialize)]
pub struct StatusField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriorityQueueEntry {
    pub rank: u32,
    pub row_ids: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanonicalLink {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusSummaryResponse {
    pub source_path: &'static str,
    pub active_snapshot: Vec<StatusField>,
    pub current_reality_summary: String,
    pub exists_now: Vec<String>,
    pub missing_now: Vec<String>,
    pub active_milestone: String,
    pub next_milestone: Option<String>,
    pub priority_rows: Vec<PriorityQueueEntry>,
    pub architecture_contract: Vec<String>,
    pub canonical_links: Vec<CanonicalLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MilestoneDetail {
    pub name: String,
    pub goal: Vec<String>,
    pub owner_crates: Vec<String>,
    pub prerequisites: Vec<String>,
    pub required_tests: Vec<String>,
    pub exit_evidence: Vec<String>,
    pub row_count: usize,
    pub row_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase5PriorityResponse {
    pub source_paths: Vec<&'static str>,
    pub active_milestone: MilestoneDetail,
    pub next_milestone: Option<String>,
    pub priority_queue: Vec<PriorityQueueEntry>,
    pub final_milestone: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoadmapStatus {
    pub milestone: String,
    pub owner_crate: String,
    pub status_bucket: String,
    pub blocked_by: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LedgerStatus {
    pub rust_owner_now: String,
    pub rust_status_now: String,
    pub parity_evidence_status: String,
    pub divergence_status: String,
    pub required_tests: String,
    pub priority: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParityRowDetailsResponse {
    pub source_paths: Vec<String>,
    pub row_id: String,
    pub domain: String,
    pub section: String,
    pub feature_group: String,
    pub feature_doc: String,
    pub baseline_source: String,
    pub baseline_paths: Vec<String>,
    pub symbol_hints: Vec<String>,
    pub baseline_behavior_or_contract: String,
    pub ledger_status: LedgerStatus,
    pub roadmap: RoadmapStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainGapEntry {
    pub row_id: String,
    pub feature_group: String,
    pub priority: String,
    pub milestone: String,
    pub owner_crate: String,
    pub status_bucket: String,
    pub rust_status_now: String,
    pub divergence_status: String,
    pub baseline_source: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainGapsRankedResponse {
    pub source_paths: Vec<String>,
    pub domain: String,
    pub active_milestone: String,
    pub total_open_rows: usize,
    pub rows: Vec<DomainGapEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaselineSourceMappingResponse {
    pub source_paths: Vec<String>,
    pub row_id: String,
    pub domain: String,
    pub feature_doc: String,
    pub baseline_source: String,
    pub baseline_paths: Vec<String>,
    pub symbol_hints: Vec<String>,
}

#[derive(Debug, Clone)]
struct RoadmapIndexEntry {
    row_id: String,
    milestone: String,
    owner_crate: String,
    status_bucket: String,
    blocked_by: String,
    evidence_ref: String,
}

#[derive(Debug, Clone)]
struct SourceMapEntry {
    feature_doc: String,
    baseline_paths: Vec<String>,
    symbol_hints: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParityRowRecord {
    row_id: String,
    domain: String,
    section: String,
    feature_group: String,
    baseline_source: String,
    baseline_behavior_or_contract: String,
    rust_owner_now: String,
    rust_status_now: String,
    parity_evidence_status: String,
    divergence_status: String,
    required_tests: String,
    priority: String,
    notes: String,
}

#[derive(Debug, Clone)]
struct MilestoneRecord {
    name: String,
    content: String,
}

pub fn status_summary(repo_root: &Path) -> Result<StatusSummaryResponse, String> {
    let text = read_repo_text(repo_root, STATUS_PATH)?;
    let sections = parse_sections(&text, "## ");

    let active_snapshot = section_content(&sections, "Active Snapshot")?;
    let current_reality = section_content(&sections, "Current Reality")?;
    let active_milestone = section_content(&sections, "Active Milestone")?;
    let priority_rows = section_content(&sections, "Priority Rows")?;
    let architecture_contract = section_content(&sections, "Architecture Contract")?;
    let canonical_links = section_content(&sections, "Canonical Links")?;

    Ok(StatusSummaryResponse {
        source_path: STATUS_PATH,
        active_snapshot: parse_status_fields(active_snapshot),
        current_reality_summary: first_paragraph(current_reality),
        exists_now: extract_list_block(current_reality, "What exists now"),
        missing_now: extract_list_block(current_reality, "What does not exist yet"),
        active_milestone: first_h3_heading(active_milestone)?,
        next_milestone: extract_inline_backtick_item(active_milestone, "Next milestone after"),
        priority_rows: parse_priority_queue(priority_rows)?,
        architecture_contract: extract_list_block(
            architecture_contract,
            "Allowed crate dependency direction",
        ),
        canonical_links: parse_canonical_links(canonical_links),
    })
}

pub fn phase5_priority(repo_root: &Path) -> Result<Phase5PriorityResponse, String> {
    let status = status_summary(repo_root)?;
    let roadmap = read_repo_text(repo_root, ROADMAP_PATH)?;
    let milestones = parse_milestones(&roadmap);
    let active_name = status.active_milestone.clone();
    let active = milestones
        .into_iter()
        .find(|milestone| milestone.name == active_name)
        .ok_or_else(|| format!("active milestone not found in roadmap: {active_name}"))?;
    let roadmap_index = parse_roadmap_index(repo_root)?;
    let row_ids = rows_for_milestone(&roadmap_index, &active_name);

    Ok(Phase5PriorityResponse {
        source_paths: vec![STATUS_PATH, ROADMAP_PATH, ROADMAP_INDEX_PATH],
        active_milestone: MilestoneDetail {
            name: active_name.clone(),
            goal: extract_list_block(&active.content, "Goal"),
            owner_crates: extract_list_block(&active.content, "Owner crates"),
            prerequisites: extract_list_block(&active.content, "Prerequisites"),
            required_tests: extract_list_block(&active.content, "Required tests"),
            exit_evidence: extract_list_block(&active.content, "Exit evidence"),
            row_count: row_ids.len(),
            row_ids,
        },
        next_milestone: status.next_milestone,
        priority_queue: status.priority_rows,
        final_milestone: MILESTONE_ORDER
            .last()
            .map(|value| (*value).to_string())
            .ok_or_else(|| "no final milestone configured".to_string())?,
    })
}

pub fn parity_row_details(repo_root: &Path, row_id: &str) -> Result<ParityRowDetailsResponse, String> {
    let normalized = normalize_row_id(row_id);
    let domain = domain_for_row(&normalized)?;
    let row = parse_ledger_rows(repo_root, &domain)?
        .into_iter()
        .find(|entry| entry.row_id == normalized)
        .ok_or_else(|| format!("row not found in {domain} ledger: {normalized}"))?;
    let index = parse_roadmap_index(repo_root)?;
    let source_map = parse_source_map(repo_root)?;
    let roadmap = index
        .get(&normalized)
        .ok_or_else(|| format!("row not found in roadmap index: {normalized}"))?;
    let source_map_entry = source_map
        .get(&normalized)
        .ok_or_else(|| format!("row not found in source map: {normalized}"))?;

    Ok(ParityRowDetailsResponse {
        source_paths: collect_source_paths(&[
            ledger_path_for_domain(&domain).to_string(),
            ROADMAP_INDEX_PATH.to_string(),
            SOURCE_MAP_PATH.to_string(),
            source_map_entry.feature_doc.clone(),
        ]),
        row_id: row.row_id,
        domain: row.domain,
        section: row.section,
        feature_group: row.feature_group,
        feature_doc: source_map_entry.feature_doc.clone(),
        baseline_source: row.baseline_source.clone(),
        baseline_paths: source_map_entry.baseline_paths.clone(),
        symbol_hints: source_map_entry.symbol_hints.clone(),
        baseline_behavior_or_contract: row.baseline_behavior_or_contract,
        ledger_status: LedgerStatus {
            rust_owner_now: row.rust_owner_now,
            rust_status_now: row.rust_status_now,
            parity_evidence_status: row.parity_evidence_status,
            divergence_status: row.divergence_status,
            required_tests: row.required_tests,
            priority: row.priority,
            notes: row.notes,
        },
        roadmap: RoadmapStatus {
            milestone: roadmap.milestone.clone(),
            owner_crate: roadmap.owner_crate.clone(),
            status_bucket: roadmap.status_bucket.clone(),
            blocked_by: roadmap.blocked_by.clone(),
            evidence_ref: roadmap.evidence_ref.clone(),
        },
    })
}

pub fn domain_gaps_ranked(
    repo_root: &Path,
    domain: &str,
    limit: usize,
) -> Result<DomainGapsRankedResponse, String> {
    let normalized_domain = normalize_domain(domain)?;
    let active_milestone = status_summary(repo_root)?.active_milestone;
    let index = parse_roadmap_index(repo_root)?;
    let rows = parse_ledger_rows(repo_root, &normalized_domain)?;

    let mut open_rows = Vec::new();

    for row in rows {
        if is_closed_row(&row) {
            continue;
        }

        let Some(index_row) = index.get(&row.row_id) else {
            continue;
        };

        open_rows.push(DomainGapEntry {
            row_id: row.row_id,
            feature_group: row.feature_group,
            priority: row.priority,
            milestone: index_row.milestone.clone(),
            owner_crate: index_row.owner_crate.clone(),
            status_bucket: index_row.status_bucket.clone(),
            rust_status_now: row.rust_status_now,
            divergence_status: row.divergence_status,
            baseline_source: row.baseline_source,
            notes: row.notes,
        });
    }

    open_rows.sort_by(|left, right| compare_gap_entries(left, right, &active_milestone));
    let total_open_rows = open_rows.len();
    open_rows.truncate(limit.max(1));

    Ok(DomainGapsRankedResponse {
        source_paths: vec![
            ledger_path_for_domain(&normalized_domain).to_string(),
            ROADMAP_INDEX_PATH.to_string(),
            STATUS_PATH.to_string(),
        ],
        domain: normalized_domain,
        active_milestone,
        total_open_rows,
        rows: open_rows,
    })
}

pub fn baseline_source_mapping(
    repo_root: &Path,
    row_id: &str,
) -> Result<BaselineSourceMappingResponse, String> {
    let details = parity_row_details(repo_root, row_id)?;

    Ok(BaselineSourceMappingResponse {
        source_paths: collect_source_paths(&[
            details.source_paths[0].clone(),
            SOURCE_MAP_PATH.to_string(),
            details.feature_doc.clone(),
        ]),
        row_id: details.row_id,
        domain: details.domain,
        feature_doc: details.feature_doc,
        baseline_source: details.baseline_source,
        baseline_paths: details.baseline_paths,
        symbol_hints: details.symbol_hints,
    })
}

fn read_repo_text(repo_root: &Path, relative_path: &str) -> Result<String, String> {
    let path = repo_root.join(relative_path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn parse_sections(text: &str, prefix: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in text.lines() {
        if let Some(title) = line.strip_prefix(prefix) {
            if let Some(previous_title) = current_title.replace(title.trim().to_string()) {
                sections.push((previous_title, current_lines.join("\n").trim().to_string()));
                current_lines.clear();
            }
            continue;
        }

        if current_title.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(title) = current_title {
        sections.push((title, current_lines.join("\n").trim().to_string()));
    }

    sections
}

fn section_content<'a>(sections: &'a [(String, String)], title: &str) -> Result<&'a str, String> {
    sections
        .iter()
        .find(|(section_title, _)| section_title == title)
        .map(|(_, content)| content.as_str())
        .ok_or_else(|| format!("missing required section in STATUS.md: {title}"))
}

fn parse_status_fields(section: &str) -> Vec<StatusField> {
    section
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter_map(|line| line.split_once(':'))
        .map(|(label, value)| StatusField {
            label: label.trim().to_string(),
            value: value.trim().to_string(),
        })
        .collect()
}

fn first_paragraph(section: &str) -> String {
    let mut lines = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }

        if trimmed.ends_with(':') {
            break;
        }

        lines.push(trimmed.to_string());
    }

    lines.join(" ")
}

fn extract_list_block(section: &str, label: &str) -> Vec<String> {
    let target = format!("{label}:");
    let mut capture = false;
    let mut items = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();

        if !capture {
            if trimmed == target {
                capture = true;
            }
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        if is_block_label(trimmed) || trimmed.starts_with("### ") {
            break;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            items.push(item.trim().to_string());
            continue;
        }

        items.push(trimmed.to_string());
    }

    items
}

fn is_block_label(line: &str) -> bool {
    line.ends_with(':')
        && !line.starts_with("- ")
        && !line.starts_with("*")
        && !line.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn first_h3_heading(section: &str) -> Result<String, String> {
    section
        .lines()
        .find_map(|line| line.trim().strip_prefix("### "))
        .map(|value| value.trim().to_string())
        .ok_or_else(|| "missing h3 heading in Active Milestone section".to_string())
}

fn extract_inline_backtick_item(section: &str, label_prefix: &str) -> Option<String> {
    let mut capture = false;

    for line in section.lines() {
        let trimmed = line.trim();

        if !capture {
            if trimmed.starts_with(label_prefix) && trimmed.ends_with(':') {
                capture = true;
            }
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- `") {
            return item.strip_suffix('`').map(|value| value.to_string());
        }

        if is_block_label(trimmed) || trimmed.starts_with("### ") {
            break;
        }
    }

    None
}

fn parse_priority_queue(section: &str) -> Result<Vec<PriorityQueueEntry>, String> {
    let mut entries = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((rank_text, remainder)) = trimmed.split_once('.') else {
            continue;
        };

        let Ok(rank) = rank_text.trim().parse::<u32>() else {
            continue;
        };

        let (row_source, summary) = split_priority_line(remainder.trim());
        let row_ids = extract_row_ids(row_source).unwrap_or_default();
        entries.push(PriorityQueueEntry {
            rank,
            row_ids,
            summary: summary.to_string(),
        });
    }

    Ok(entries)
}

fn split_priority_line(line: &str) -> (&str, &str) {
    if let Some((left, right)) = line.split_once(" — ") {
        return (left, right);
    }

    if let Some((left, right)) = line.split_once(" - ") {
        return (left, right);
    }

    (line, "")
}

fn extract_row_ids(text: &str) -> Result<Vec<String>, String> {
    let mut row_ids = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            current.push(ch);
            continue;
        }

        push_row_token(&mut row_ids, &mut current);
    }

    push_row_token(&mut row_ids, &mut current);

    if row_ids.is_empty() {
        return Err(format!("no row ids found in priority line: {text}"));
    }

    Ok(row_ids)
}

fn push_row_token(row_ids: &mut Vec<String>, current: &mut String) {
    if current.len() == 7 {
        let prefix = &current[0..3];
        let digits = &current[4..7];
        if matches!(prefix, "CLI" | "CDC" | "HIS")
            && current.as_bytes().get(3) == Some(&b'-')
            && digits.chars().all(|ch| ch.is_ascii_digit())
        {
            row_ids.push(current.clone());
        }
    }

    current.clear();
}

fn parse_canonical_links(section: &str) -> Vec<CanonicalLink> {
    section
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter_map(|line| line.split_once(':'))
        .map(|(label, path)| CanonicalLink {
            label: label.trim().to_string(),
            path: path.trim().trim_matches('`').to_string(),
        })
        .collect()
}

fn parse_milestones(text: &str) -> Vec<MilestoneRecord> {
    parse_sections(text, "### ")
        .into_iter()
        .map(|(title, content)| MilestoneRecord {
            name: normalize_milestone_heading(&title),
            content,
        })
        .collect()
}

fn normalize_milestone_heading(raw: &str) -> String {
    raw.split_once(". ")
        .map(|(_, remainder)| remainder.trim().to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

fn parse_roadmap_index(repo_root: &Path) -> Result<HashMap<String, RoadmapIndexEntry>, String> {
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

fn rows_for_milestone(index: &HashMap<String, RoadmapIndexEntry>, milestone: &str) -> Vec<String> {
    let mut row_ids: Vec<String> = index
        .values()
        .filter(|entry| entry.milestone == milestone)
        .map(|entry| entry.row_id.clone())
        .collect();
    row_ids.sort();
    row_ids
}

fn parse_ledger_rows(repo_root: &Path, domain: &str) -> Result<Vec<ParityRowRecord>, String> {
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

                while index < lines.len() {
                    let row_line = lines[index].trim();
                    if !row_line.starts_with('|') {
                        break;
                    }

                    let cells = split_markdown_row(row_line);
                    if cells.len() != header.len() {
                        break;
                    }

                    let row = map_row(&header, &cells);
                    let Some(row_id) = row.get("ID") else {
                        index += 1;
                        continue;
                    };

                    if !row_id.starts_with(domain) {
                        index += 1;
                        continue;
                    }

                    rows.push(ParityRowRecord {
                        row_id: row_id.to_string(),
                        domain: domain.to_string(),
                        section: current_section.clone(),
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

                    index += 1;
                }

                continue;
            }
        }

        index += 1;
    }

    Ok(rows)
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

fn map_row(header: &[String], cells: &[String]) -> HashMap<String, String> {
    header
        .iter()
        .cloned()
        .zip(cells.iter().cloned())
        .collect::<HashMap<_, _>>()
}

fn value_or_empty(row: &HashMap<String, String>, key: &str) -> String {
    row.get(key).cloned().unwrap_or_default()
}

fn normalize_row_id(row_id: &str) -> String {
    row_id.trim().to_uppercase()
}

fn domain_for_row(row_id: &str) -> Result<String, String> {
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

fn normalize_domain(domain: &str) -> Result<String, String> {
    let normalized = domain.trim().to_uppercase();
    if matches!(normalized.as_str(), "CLI" | "CDC" | "HIS") {
        return Ok(normalized);
    }

    Err(format!("unsupported domain: {domain}"))
}

fn ledger_path_for_domain(domain: &str) -> &'static str {
    match domain {
        "CLI" => "docs/parity/cli/implementation-checklist.md",
        "CDC" => "docs/parity/cdc/implementation-checklist.md",
        "HIS" => "docs/parity/his/implementation-checklist.md",
        _ => "docs/parity/README.md",
    }
}

fn parse_source_map(repo_root: &Path) -> Result<HashMap<String, SourceMapEntry>, String> {
    let text = read_repo_text(repo_root, SOURCE_MAP_PATH)?;
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Err("source map is empty".to_string());
    };

    let columns = split_csv_row(header);
    let expected_header = vec![
        "row_id".to_string(),
        "domain".to_string(),
        "feature_doc".to_string(),
        "baseline_paths".to_string(),
        "symbol_hints".to_string(),
    ];
    if columns != expected_header {
        return Err("source map header does not match the expected contract".to_string());
    }

    let mut rows = HashMap::new();

    for (line_number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let columns = split_csv_row(line);
        if columns.len() != 5 {
            return Err(format!(
                "source map row {} has {} columns, expected 5",
                line_number + 2,
                columns.len()
            ));
        }

        let row_id = columns[0].to_string();
        rows.insert(
            row_id,
            SourceMapEntry {
                feature_doc: columns[2].to_string(),
                baseline_paths: split_semicolon_list(&columns[3]),
                symbol_hints: split_semicolon_list(&columns[4]),
            },
        );
    }

    Ok(rows)
}

fn split_csv_row(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
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
                columns.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    columns.push(current);
    columns
}

fn split_semicolon_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_source_paths(paths: &[String]) -> Vec<String> {
    let mut unique = Vec::new();

    for path in paths {
        if !unique.contains(path) {
            unique.push(path.clone());
        }
    }

    unique
}

fn is_closed_row(row: &ParityRowRecord) -> bool {
    matches!(
        row.rust_status_now.as_str(),
        "audited, parity-backed" | "audited, intentional divergence"
    )
}

fn compare_gap_entries(
    left: &DomainGapEntry,
    right: &DomainGapEntry,
    active_milestone: &str,
) -> std::cmp::Ordering {
    gap_sort_key(left, active_milestone).cmp(&gap_sort_key(right, active_milestone))
}

fn gap_sort_key(entry: &DomainGapEntry, active_milestone: &str) -> (u8, u8, usize, String) {
    let active_rank = if entry.milestone == active_milestone { 0 } else { 1 };
    let priority_rank = match entry.priority.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    };
    let milestone_rank = milestone_rank(&entry.milestone);

    (active_rank, priority_rank, milestone_rank, entry.row_id.clone())
}

fn milestone_rank(name: &str) -> usize {
    MILESTONE_ORDER
        .iter()
        .position(|candidate| *candidate == name)
        .unwrap_or(MILESTONE_ORDER.len())
}

#[cfg(test)]
mod tests {
    use super::{
        baseline_source_mapping, domain_gaps_ranked, parity_row_details, parse_roadmap_index,
        phase5_priority, status_summary,
    };
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf())
            .expect("repo root")
    }

    #[test]
    fn parses_status_summary() {
        let summary = status_summary(&repo_root()).expect("status summary");

        assert_eq!(summary.active_milestone, "CDC Contract Foundation");
        assert_eq!(
            summary.next_milestone.as_deref(),
            Some("Host and Runtime Foundation")
        );
        assert!(!summary.priority_rows.is_empty());
    }

    #[test]
    fn parses_phase5_priority() {
        let priority = phase5_priority(&repo_root()).expect("phase5 priority");

        assert_eq!(priority.active_milestone.name, "CDC Contract Foundation");
        assert!(priority.active_milestone.row_count > 0);
        assert_eq!(priority.final_milestone, "Performance Architecture Overhaul");
    }

    #[test]
    fn roadmap_index_is_populated() {
        let index = parse_roadmap_index(&repo_root()).expect("roadmap index");

        assert_eq!(index.len(), 150);
        assert!(index.contains_key("CLI-001"));
        assert!(index.contains_key("CDC-001"));
        assert!(index.contains_key("HIS-001"));
    }

    #[test]
    fn finds_parity_row_details() {
        let row = parity_row_details(&repo_root(), "CLI-001").expect("row details");

        assert_eq!(row.roadmap.milestone, "CLI Foundation");
        assert_eq!(row.domain, "CLI");
        assert_eq!(row.feature_doc, "docs/parity/cli/root-and-global-flags.md");
        assert!(!row.baseline_paths.is_empty());
        assert!(!row.symbol_hints.is_empty());
    }

    #[test]
    fn ranks_domain_gaps() {
        let ranked = domain_gaps_ranked(&repo_root(), "CDC", 5).expect("domain gaps");

        assert_eq!(ranked.domain, "CDC");
        assert!(ranked.total_open_rows >= ranked.rows.len());
        assert!(!ranked.rows.is_empty());
    }

    #[test]
    fn maps_baseline_sources() {
        let mapping = baseline_source_mapping(&repo_root(), "CLI-001").expect("baseline mapping");

        assert_eq!(mapping.domain, "CLI");
        assert_eq!(mapping.feature_doc, "docs/parity/cli/root-and-global-flags.md");
        assert!(!mapping.baseline_paths.is_empty());
        assert!(!mapping.symbol_hints.is_empty());
    }
}
