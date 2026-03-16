use super::{DomainGapEntry, MILESTONE_ORDER};
use std::cmp::Ordering;

pub(super) fn compare_gap_entries(
    left: &DomainGapEntry,
    right: &DomainGapEntry,
    active_milestone: &str,
) -> Ordering {
    gap_sort_key(left, active_milestone).cmp(&gap_sort_key(right, active_milestone))
}

fn gap_sort_key(entry: &DomainGapEntry, active_milestone: &str) -> (u8, u8, u8, usize, String) {
    let actionable_rank = if entry.actionable_now { 0 } else { 1 };
    let active_rank = if entry.milestone == active_milestone { 0 } else { 1 };
    let priority_rank = match entry.priority.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    };
    let milestone_rank = milestone_rank(&entry.milestone);

    (
        actionable_rank,
        active_rank,
        priority_rank,
        milestone_rank,
        entry.row_id.clone(),
    )
}

fn milestone_rank(name: &str) -> usize {
    MILESTONE_ORDER
        .iter()
        .position(|candidate| *candidate == name)
        .unwrap_or(MILESTONE_ORDER.len())
}
