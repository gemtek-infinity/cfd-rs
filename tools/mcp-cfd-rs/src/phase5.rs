mod helpers;
mod ledger;
mod roadmap;
mod sorting;
mod source_map;
mod text;

use self::helpers::{
    collect_source_paths, is_closed_row, is_not_audited_status, is_partial_status, is_row_id,
    ledger_path_for_domain, normalize_domain, normalize_row_id, read_repo_text,
};
use self::ledger::parse_ledger_rows;
use self::roadmap::{parse_milestones, parse_roadmap_index, rows_for_milestone};
use self::sorting::compare_gap_entries;
use self::source_map::parse_source_map;
use self::text::{
    extract_inline_backtick_item, extract_list_block, first_h3_heading, first_paragraph,
    parse_canonical_links, parse_priority_queue, parse_sections, parse_status_fields, section_content,
};
use serde::Serialize;
use std::collections::HashMap;
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
pub struct ParityDomainProgress {
    pub domain: String,
    pub total: usize,
    pub closed: usize,
    pub partial: usize,
    pub absent: usize,
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
    pub parity_progress: Vec<ParityDomainProgress>,
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
    pub next_actionable_row_id: Option<String>,
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
    pub evidence_ref: String,
    pub actionable_now: bool,
    pub actionability_reason: String,
    pub blocked_by_satisfied: bool,
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
    pub parity_evidence_status: String,
    pub divergence_status: String,
    pub blocked_by: String,
    pub evidence_ref: String,
    pub actionable_now: bool,
    pub actionability_reason: String,
    pub baseline_source: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainGapsRankedResponse {
    pub source_paths: Vec<String>,
    pub domain: String,
    pub active_milestone: String,
    pub total_open_rows: usize,
    pub partial_rows: usize,
    pub absent_rows: usize,
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

#[derive(Debug, Clone, Serialize)]
pub struct NextParityTicketResponse {
    pub source_paths: Vec<String>,
    pub row_id: String,
    pub domain: String,
    pub feature_group: String,
    pub owner_crate: String,
    pub priority: String,
    pub milestone: String,
    pub rust_status_now: String,
    pub parity_evidence_status: String,
    pub blocked_by: String,
    pub actionable_now: bool,
    pub actionability_reason: String,
    pub selection_basis: String,
    pub feature_doc: String,
    pub baseline_paths: Vec<String>,
    pub required_tests: String,
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

#[derive(Debug, Clone)]
struct Actionability {
    actionable_now: bool,
    actionability_reason: String,
    blocked_by_satisfied: bool,
}

#[derive(Debug, Clone)]
struct ResolvedParityRow {
    row: ParityRowRecord,
    roadmap: RoadmapIndexEntry,
    source_map: SourceMapEntry,
    actionability: Actionability,
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

    let parity_progress = compute_parity_progress(repo_root)?;

    Ok(StatusSummaryResponse {
        source_path: STATUS_PATH,
        active_snapshot: parse_status_fields(active_snapshot),
        current_reality_summary: first_paragraph(current_reality),
        exists_now: extract_list_block(current_reality, "What exists now"),
        missing_now: extract_list_block(current_reality, "What does not exist yet"),
        active_milestone: first_h3_heading(active_milestone)?,
        next_milestone: extract_inline_backtick_item(active_milestone, "Next milestone after"),
        parity_progress,
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
    let next_actionable_row_id = next_parity_ticket(repo_root, None, false)
        .ok()
        .map(|ticket| ticket.row_id);

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
        next_actionable_row_id,
        priority_queue: status.priority_rows,
        final_milestone: MILESTONE_ORDER
            .last()
            .map(|value| (*value).to_string())
            .ok_or_else(|| "no final milestone configured".to_string())?,
    })
}

pub fn parity_row_details(repo_root: &Path, row_id: &str) -> Result<ParityRowDetailsResponse, String> {
    let active_milestone = status_summary(repo_root)?.active_milestone;
    let all_rows = parse_all_ledger_rows(repo_root)?;
    let index = parse_roadmap_index(repo_root)?;
    let source_map = parse_source_map(repo_root)?;
    let resolved = resolve_parity_row(
        &normalize_row_id(row_id),
        &all_rows,
        &index,
        &source_map,
        &active_milestone,
    )?;

    Ok(ParityRowDetailsResponse {
        source_paths: collect_source_paths(&[
            ledger_path_for_domain(&resolved.row.domain).to_string(),
            ROADMAP_INDEX_PATH.to_string(),
            SOURCE_MAP_PATH.to_string(),
            resolved.source_map.feature_doc.clone(),
        ]),
        row_id: resolved.row.row_id.clone(),
        domain: resolved.row.domain.clone(),
        section: resolved.row.section.clone(),
        feature_group: resolved.row.feature_group.clone(),
        feature_doc: resolved.source_map.feature_doc.clone(),
        baseline_source: resolved.row.baseline_source.clone(),
        baseline_paths: resolved.source_map.baseline_paths.clone(),
        symbol_hints: resolved.source_map.symbol_hints.clone(),
        baseline_behavior_or_contract: resolved.row.baseline_behavior_or_contract.clone(),
        evidence_ref: resolved.roadmap.evidence_ref.clone(),
        actionable_now: resolved.actionability.actionable_now,
        actionability_reason: resolved.actionability.actionability_reason.clone(),
        blocked_by_satisfied: resolved.actionability.blocked_by_satisfied,
        ledger_status: LedgerStatus {
            rust_owner_now: resolved.row.rust_owner_now.clone(),
            rust_status_now: resolved.row.rust_status_now.clone(),
            parity_evidence_status: resolved.row.parity_evidence_status.clone(),
            divergence_status: resolved.row.divergence_status.clone(),
            required_tests: resolved.row.required_tests.clone(),
            priority: resolved.row.priority.clone(),
            notes: resolved.row.notes.clone(),
        },
        roadmap: RoadmapStatus {
            milestone: resolved.roadmap.milestone.clone(),
            owner_crate: resolved.roadmap.owner_crate.clone(),
            status_bucket: resolved.roadmap.status_bucket.clone(),
            blocked_by: resolved.roadmap.blocked_by.clone(),
            evidence_ref: resolved.roadmap.evidence_ref.clone(),
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
    let all_rows = parse_all_ledger_rows(repo_root)?;
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

        let actionability = evaluate_actionability(&row, index_row, &active_milestone, &all_rows)?;

        open_rows.push(DomainGapEntry {
            row_id: row.row_id,
            feature_group: row.feature_group,
            priority: row.priority,
            milestone: index_row.milestone.clone(),
            owner_crate: index_row.owner_crate.clone(),
            status_bucket: index_row.status_bucket.clone(),
            rust_status_now: row.rust_status_now,
            parity_evidence_status: row.parity_evidence_status,
            divergence_status: row.divergence_status,
            blocked_by: index_row.blocked_by.clone(),
            evidence_ref: index_row.evidence_ref.clone(),
            actionable_now: actionability.actionable_now,
            actionability_reason: actionability.actionability_reason,
            baseline_source: row.baseline_source,
            notes: row.notes,
        });
    }

    open_rows.sort_by(|left, right| compare_gap_entries(left, right, &active_milestone));
    let total_open_rows = open_rows.len();
    let partial_rows = open_rows
        .iter()
        .filter(|row| is_partial_status(&row.rust_status_now))
        .count();
    let absent_rows = total_open_rows - partial_rows;

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
        partial_rows,
        absent_rows,
        rows: open_rows,
    })
}

pub fn next_parity_ticket(
    repo_root: &Path,
    domain: Option<&str>,
    include_blocked: bool,
) -> Result<NextParityTicketResponse, String> {
    let status = status_summary(repo_root)?;
    let active_milestone = status.active_milestone.clone();
    let normalized_domain = domain.map(normalize_domain).transpose()?;
    let all_rows = parse_all_ledger_rows(repo_root)?;
    let index = parse_roadmap_index(repo_root)?;
    let source_map = parse_source_map(repo_root)?;

    if let Some(resolved) = next_ticket_from_priority_queue(
        &status.priority_rows,
        normalized_domain.as_deref(),
        include_blocked,
        &all_rows,
        &index,
        &source_map,
        &active_milestone,
    )? {
        return Ok(build_next_ticket_response(
            &resolved,
            "status_priority_queue",
            true,
        ));
    }

    let Some(row) = next_ticket_from_open_rows(
        normalized_domain.as_deref(),
        include_blocked,
        &all_rows,
        &index,
        &source_map,
        &active_milestone,
    )?
    else {
        return Err(no_matching_ticket_error(normalized_domain.as_deref()));
    };

    Ok(build_next_ticket_response(&row, "fallback_ranked_gap", false))
}

fn next_ticket_from_priority_queue(
    priority_rows: &[PriorityQueueEntry],
    domain_filter: Option<&str>,
    include_blocked: bool,
    all_rows: &HashMap<String, ParityRowRecord>,
    index: &HashMap<String, RoadmapIndexEntry>,
    source_map: &HashMap<String, SourceMapEntry>,
    active_milestone: &str,
) -> Result<Option<ResolvedParityRow>, String> {
    for entry in priority_rows {
        for row_id in &entry.row_ids {
            let resolved = resolve_parity_row(row_id, all_rows, index, source_map, active_milestone)?;
            if next_ticket_candidate(&resolved, domain_filter, include_blocked) {
                return Ok(Some(resolved));
            }
        }
    }

    Ok(None)
}

fn next_ticket_from_open_rows(
    domain_filter: Option<&str>,
    include_blocked: bool,
    all_rows: &HashMap<String, ParityRowRecord>,
    index: &HashMap<String, RoadmapIndexEntry>,
    source_map: &HashMap<String, SourceMapEntry>,
    active_milestone: &str,
) -> Result<Option<ResolvedParityRow>, String> {
    let mut open_rows = collect_open_rows(all_rows, index, source_map, active_milestone)?;
    open_rows.retain(|row| matches_domain_filter(&row.row.domain, domain_filter));
    if !include_blocked {
        open_rows.retain(|row| row.actionability.actionable_now);
    }
    open_rows.sort_by(|left, right| compare_resolved_rows(left, right, active_milestone));

    Ok(open_rows.into_iter().next())
}

fn next_ticket_candidate(
    resolved: &ResolvedParityRow,
    domain_filter: Option<&str>,
    include_blocked: bool,
) -> bool {
    matches_domain_filter(&resolved.row.domain, domain_filter)
        && !is_closed_row(&resolved.row)
        && (include_blocked || resolved.actionability.actionable_now)
}

fn matches_domain_filter(row_domain: &str, domain_filter: Option<&str>) -> bool {
    domain_filter.is_none_or(|expected| row_domain == expected)
}

fn compare_resolved_rows(
    left: &ResolvedParityRow,
    right: &ResolvedParityRow,
    active_milestone: &str,
) -> std::cmp::Ordering {
    let left_gap = resolved_row_to_gap_entry(left);
    let right_gap = resolved_row_to_gap_entry(right);
    compare_gap_entries(&left_gap, &right_gap, active_milestone)
}

fn no_matching_ticket_error(domain_filter: Option<&str>) -> String {
    match domain_filter {
        Some(domain) => format!("no matching parity ticket found for domain: {domain}"),
        None => "no matching parity ticket found".to_string(),
    }
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

fn parse_all_ledger_rows(repo_root: &Path) -> Result<HashMap<String, ParityRowRecord>, String> {
    let mut rows = HashMap::new();

    for domain in ["CLI", "CDC", "HIS"] {
        for row in parse_ledger_rows(repo_root, domain)? {
            rows.insert(row.row_id.clone(), row);
        }
    }

    Ok(rows)
}

fn resolve_parity_row(
    row_id: &str,
    all_rows: &HashMap<String, ParityRowRecord>,
    index: &HashMap<String, RoadmapIndexEntry>,
    source_map: &HashMap<String, SourceMapEntry>,
    active_milestone: &str,
) -> Result<ResolvedParityRow, String> {
    let normalized = normalize_row_id(row_id);
    let row = all_rows
        .get(&normalized)
        .cloned()
        .ok_or_else(|| format!("row not found in ledger set: {normalized}"))?;
    let roadmap = index
        .get(&normalized)
        .cloned()
        .ok_or_else(|| format!("row not found in roadmap index: {normalized}"))?;
    let source_map = source_map
        .get(&normalized)
        .cloned()
        .ok_or_else(|| format!("row not found in source map: {normalized}"))?;
    let actionability = evaluate_actionability(&row, &roadmap, active_milestone, all_rows)?;

    Ok(ResolvedParityRow {
        row,
        roadmap,
        source_map,
        actionability,
    })
}

fn collect_open_rows(
    all_rows: &HashMap<String, ParityRowRecord>,
    index: &HashMap<String, RoadmapIndexEntry>,
    source_map: &HashMap<String, SourceMapEntry>,
    active_milestone: &str,
) -> Result<Vec<ResolvedParityRow>, String> {
    let mut rows = Vec::new();

    for row_id in all_rows.keys() {
        let resolved = resolve_parity_row(row_id, all_rows, index, source_map, active_milestone)?;
        if !is_closed_row(&resolved.row) {
            rows.push(resolved);
        }
    }

    Ok(rows)
}

fn evaluate_actionability(
    row: &ParityRowRecord,
    roadmap: &RoadmapIndexEntry,
    active_milestone: &str,
    all_rows: &HashMap<String, ParityRowRecord>,
) -> Result<Actionability, String> {
    let (blocked_by_satisfied, blocked_reason) =
        evaluate_blocker(&roadmap.blocked_by, active_milestone, all_rows)?;

    let (actionable_now, actionability_reason) = if is_closed_row(row) {
        (false, "row already closed".to_string())
    } else if matches!(
        roadmap.status_bucket.as_str(),
        "already_proven" | "intentional_divergence"
    ) {
        (
            false,
            format!("status bucket {} is not actionable work", roadmap.status_bucket),
        )
    } else if roadmap.status_bucket == "deferred" {
        (
            false,
            format!("status bucket deferred until {}", roadmap.blocked_by),
        )
    } else if roadmap.status_bucket == "non_lane" {
        (
            false,
            "status bucket non_lane is out of the admitted lane".to_string(),
        )
    } else if !blocked_by_satisfied {
        (false, blocked_reason)
    } else {
        (true, blocked_reason)
    };

    Ok(Actionability {
        actionable_now,
        actionability_reason,
        blocked_by_satisfied,
    })
}

fn evaluate_blocker(
    blocked_by: &str,
    active_milestone: &str,
    all_rows: &HashMap<String, ParityRowRecord>,
) -> Result<(bool, String), String> {
    let normalized = blocked_by.trim();

    if normalized.is_empty() || normalized == "none" {
        return Ok((true, "no blocker recorded".to_string()));
    }

    if is_row_id(normalized) {
        let Some(row) = all_rows.get(normalized) else {
            return Err(format!("blocked_by row not found in ledger set: {normalized}"));
        };
        return Ok(if is_closed_row(row) {
            (true, format!("blocked_by row {normalized} is already closed"))
        } else {
            (false, format!("blocked_by row {normalized} is not yet closed"))
        });
    }

    let blocked_rank =
        milestone_rank(normalized).ok_or_else(|| format!("unknown blocked_by milestone: {normalized}"))?;
    let active_rank = milestone_rank(active_milestone)
        .ok_or_else(|| format!("unknown active milestone: {active_milestone}"))?;

    Ok(if blocked_rank < active_rank {
        (true, format!("prerequisite milestone {normalized} is complete"))
    } else {
        (
            false,
            format!("prerequisite milestone {normalized} is not yet complete"),
        )
    })
}

fn milestone_rank(name: &str) -> Option<usize> {
    MILESTONE_ORDER.iter().position(|candidate| *candidate == name)
}

fn resolved_row_to_gap_entry(row: &ResolvedParityRow) -> DomainGapEntry {
    DomainGapEntry {
        row_id: row.row.row_id.clone(),
        feature_group: row.row.feature_group.clone(),
        priority: row.row.priority.clone(),
        milestone: row.roadmap.milestone.clone(),
        owner_crate: row.roadmap.owner_crate.clone(),
        status_bucket: row.roadmap.status_bucket.clone(),
        rust_status_now: row.row.rust_status_now.clone(),
        parity_evidence_status: row.row.parity_evidence_status.clone(),
        divergence_status: row.row.divergence_status.clone(),
        blocked_by: row.roadmap.blocked_by.clone(),
        evidence_ref: row.roadmap.evidence_ref.clone(),
        actionable_now: row.actionability.actionable_now,
        actionability_reason: row.actionability.actionability_reason.clone(),
        baseline_source: row.row.baseline_source.clone(),
        notes: row.row.notes.clone(),
    }
}

fn build_next_ticket_response(
    row: &ResolvedParityRow,
    selection_basis: &str,
    include_status_path: bool,
) -> NextParityTicketResponse {
    let mut source_paths = vec![
        ledger_path_for_domain(&row.row.domain).to_string(),
        ROADMAP_INDEX_PATH.to_string(),
        SOURCE_MAP_PATH.to_string(),
        row.source_map.feature_doc.clone(),
    ];
    if include_status_path {
        source_paths.insert(0, STATUS_PATH.to_string());
    }

    NextParityTicketResponse {
        source_paths: collect_source_paths(&source_paths),
        row_id: row.row.row_id.clone(),
        domain: row.row.domain.clone(),
        feature_group: row.row.feature_group.clone(),
        owner_crate: row.roadmap.owner_crate.clone(),
        priority: row.row.priority.clone(),
        milestone: row.roadmap.milestone.clone(),
        rust_status_now: row.row.rust_status_now.clone(),
        parity_evidence_status: row.row.parity_evidence_status.clone(),
        blocked_by: row.roadmap.blocked_by.clone(),
        actionable_now: row.actionability.actionable_now,
        actionability_reason: row.actionability.actionability_reason.clone(),
        selection_basis: selection_basis.to_string(),
        feature_doc: row.source_map.feature_doc.clone(),
        baseline_paths: row.source_map.baseline_paths.clone(),
        required_tests: row.row.required_tests.clone(),
    }
}

fn compute_parity_progress(repo_root: &Path) -> Result<Vec<ParityDomainProgress>, String> {
    ["CLI", "CDC", "HIS"]
        .iter()
        .map(|domain| {
            let rows = parse_ledger_rows(repo_root, domain)?;
            let total = rows.len();
            let closed = rows.iter().filter(|row| is_closed_row(row)).count();
            let partial = rows
                .iter()
                .filter(|row| !is_closed_row(row) && is_partial_status(&row.rust_status_now))
                .count();
            let not_audited = rows
                .iter()
                .filter(|row| is_not_audited_status(&row.rust_status_now))
                .count();
            let absent = total - closed - partial;
            debug_assert!(not_audited <= absent);

            Ok(ParityDomainProgress {
                domain: domain.to_string(),
                total,
                closed,
                partial,
                absent,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        baseline_source_mapping, domain_gaps_ranked, helpers::is_row_id, next_parity_ticket,
        parity_row_details, parse_roadmap_index, phase5_priority, status_summary,
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

        assert_eq!(summary.active_milestone, "Command Family Closure");
        assert_eq!(summary.next_milestone.as_deref(), Some("Proof Closure"));
        assert!(!summary.priority_rows.is_empty());

        assert_eq!(summary.parity_progress.len(), 3);

        let cli = summary
            .parity_progress
            .iter()
            .find(|p| p.domain == "CLI")
            .expect("CLI progress");
        assert_eq!(cli.total, 32);
        assert!(cli.partial > 0);
        assert_eq!(cli.closed + cli.partial + cli.absent, cli.total);

        let cdc = summary
            .parity_progress
            .iter()
            .find(|p| p.domain == "CDC")
            .expect("CDC progress");
        assert_eq!(cdc.total, 44);
        assert_eq!(cdc.closed + cdc.partial + cdc.absent, cdc.total);

        let his = summary
            .parity_progress
            .iter()
            .find(|p| p.domain == "HIS")
            .expect("HIS progress");
        assert_eq!(his.total, 74);
        assert_eq!(his.closed + his.partial + his.absent, his.total);
    }

    #[test]
    fn parses_phase5_priority() {
        let priority = phase5_priority(&repo_root()).expect("phase5 priority");

        assert_eq!(priority.active_milestone.name, "Command Family Closure");
        assert!(priority.active_milestone.row_count > 0);
        assert_eq!(priority.final_milestone, "Performance Architecture Overhaul");
        if let Some(next_id) = &priority.next_actionable_row_id {
            assert!(
                is_row_id(next_id),
                "next_actionable_row_id should be a valid parity row ID, got: {next_id}",
            );
        }
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
        assert!(row.blocked_by_satisfied);
        assert_eq!(
            row.evidence_ref,
            "docs/parity/cli/implementation-checklist.md#cli-001"
        );
        // actionable_now is derived from ledger status — verify consistency
        // rather than hardcoding a boolean that flips on every ticket close
        let is_closed = row.ledger_status.rust_status_now == "audited, parity-backed"
            || row.ledger_status.rust_status_now == "audited, intentional divergence";
        assert_eq!(
            row.actionable_now, !is_closed,
            "actionable_now should be false when row is closed (status: {})",
            row.ledger_status.rust_status_now,
        );
    }

    #[test]
    fn ranks_domain_gaps() {
        let ranked = domain_gaps_ranked(&repo_root(), "CDC", 5).expect("domain gaps");

        assert_eq!(ranked.domain, "CDC");
        assert!(ranked.total_open_rows >= ranked.rows.len());
        assert!(!ranked.rows.is_empty());
        assert_eq!(ranked.partial_rows + ranked.absent_rows, ranked.total_open_rows);
        assert!(ranked.rows.iter().all(|row| !row.actionability_reason.is_empty()));
    }

    #[test]
    fn maps_baseline_sources() {
        let mapping = baseline_source_mapping(&repo_root(), "CLI-001").expect("baseline mapping");

        assert_eq!(mapping.domain, "CLI");
        assert_eq!(mapping.feature_doc, "docs/parity/cli/root-and-global-flags.md");
        assert!(!mapping.baseline_paths.is_empty());
        assert!(!mapping.symbol_hints.is_empty());
    }

    #[test]
    fn next_ticket_prefers_status_priority_queue() {
        let ticket = next_parity_ticket(&repo_root(), None, false).expect("next ticket");

        assert!(
            is_row_id(&ticket.row_id),
            "next ticket should be a valid parity row ID, got: {}",
            ticket.row_id,
        );
        assert!(ticket.actionable_now);
        assert_eq!(ticket.selection_basis, "status_priority_queue");
    }

    #[test]
    fn blocked_rows_are_skipped_unless_requested() {
        // All remaining partial HIS rows are deferred — none are actionable
        let no_ticket = next_parity_ticket(&repo_root(), Some("HIS"), false);
        assert!(
            no_ticket.is_err(),
            "all HIS partial rows are deferred; none should be returned without include_blocked"
        );

        // With include_blocked=true, deferred HIS rows become visible
        let blocked_ticket = next_parity_ticket(&repo_root(), Some("HIS"), true).expect("blocked HIS ticket");
        assert!(blocked_ticket.row_id.starts_with("HIS-"));
        assert!(!blocked_ticket.actionable_now);
    }

    #[test]
    fn closed_his_metrics_rows_do_not_rank_as_open_gaps() {
        let ranked = domain_gaps_ranked(&repo_root(), "HIS", 40).expect("HIS gaps");
        let row_ids = ranked
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>();

        for row_id in ["HIS-024", "HIS-025", "HIS-026", "HIS-027"] {
            assert!(!row_ids.contains(&row_id));
        }
    }

    #[test]
    fn blocked_row_details_report_actionability() {
        let row = parity_row_details(&repo_root(), "HIS-016").expect("blocked row details");

        assert!(!row.actionable_now);
        assert!(!row.blocked_by_satisfied);
        assert_eq!(
            row.actionability_reason,
            "status bucket deferred until Command Family Closure"
        );
    }
}
