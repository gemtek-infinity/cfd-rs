use super::DomainGapEntry;
use std::cmp::Ordering;

pub(super) fn compare_gap_entries(
    left: &DomainGapEntry,
    right: &DomainGapEntry,
    active_milestone: &str,
    milestone_order: &[String],
) -> Ordering {
    gap_sort_key(left, active_milestone, milestone_order).cmp(&gap_sort_key(
        right,
        active_milestone,
        milestone_order,
    ))
}

fn gap_sort_key(
    entry: &DomainGapEntry,
    active_milestone: &str,
    milestone_order: &[String],
) -> (u8, u8, u8, usize, String) {
    let actionable_rank = if entry.actionable_now { 0 } else { 1 };
    let active_rank = if entry.milestone == active_milestone { 0 } else { 1 };
    let priority_rank = match entry.priority.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    };
    let ms_rank = milestone_order
        .iter()
        .position(|m| m == &entry.milestone)
        .unwrap_or(milestone_order.len());

    (
        actionable_rank,
        active_rank,
        priority_rank,
        ms_rank,
        entry.row_id.clone(),
    )
}
