mod helpers;
mod ledger;
mod roadmap;
mod sorting;
mod source_map;
mod text;

use self::helpers::{
    collect_source_paths, domain_for_row, is_closed_row, is_partial_status, ledger_path_for_domain,
    normalize_domain, normalize_row_id, read_repo_text,
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
            let absent = total - closed - partial;

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
        assert_eq!(ranked.partial_rows + ranked.absent_rows, ranked.total_open_rows);
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
