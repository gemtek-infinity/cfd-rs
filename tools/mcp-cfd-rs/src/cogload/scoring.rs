use super::types::{
    FileScoreCategory, FunctionEntry, MetricComplexityCategory, RecommendedAction, TotalComplexityCategory,
};

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

pub const FILE_REVIEWABLE_SCORE: f64 = 15.0;
pub const FILE_REDUCE_WHEN_TOUCHED_SCORE: f64 = 30.0;
pub const FILE_REFACTOR_NOW_SCORE: f64 = 45.0;
pub const FILE_CRITICAL_HOTSPOT_SCORE: f64 = 75.0;

const FUNCTION_CYCLOMATIC_MODERATE: u32 = 11;
const FUNCTION_CYCLOMATIC_HIGH: u32 = 21;
const FUNCTION_CYCLOMATIC_VERY_HIGH: u32 = 31;

const FUNCTION_COGNITIVE_MODERATE: u32 = 10;
const FUNCTION_COGNITIVE_HIGH: u32 = 15;
const FUNCTION_COGNITIVE_VERY_HIGH: u32 = 25;

const FUNCTION_TOTAL_MODERATE: u32 = 20;
const FUNCTION_TOTAL_HIGH: u32 = 36;
const FUNCTION_TOTAL_EXCESSIVE: u32 = 50;

// ---------------------------------------------------------------------------
// File-level categories — f64 ranges use early-return guards
// ---------------------------------------------------------------------------

pub fn categorize_file_score(score: f64) -> FileScoreCategory {
    if score >= FILE_CRITICAL_HOTSPOT_SCORE {
        return FileScoreCategory::CriticalHotspot;
    }
    if score >= FILE_REFACTOR_NOW_SCORE {
        return FileScoreCategory::HighHotspot;
    }
    if score >= FILE_REDUCE_WHEN_TOUCHED_SCORE {
        return FileScoreCategory::Hotspot;
    }
    if score >= FILE_REVIEWABLE_SCORE {
        return FileScoreCategory::Reviewable;
    }
    FileScoreCategory::Negligible
}

pub fn recommended_action_for_file_score(score: f64) -> RecommendedAction {
    if score >= FILE_REFACTOR_NOW_SCORE {
        return RecommendedAction::RefactorNow;
    }
    if score >= FILE_REDUCE_WHEN_TOUCHED_SCORE {
        return RecommendedAction::ReduceWhenTouched;
    }
    if score >= FILE_REVIEWABLE_SCORE {
        return RecommendedAction::Review;
    }
    RecommendedAction::Ignore
}

// ---------------------------------------------------------------------------
// Function-level categories — u32 ranges use match
// ---------------------------------------------------------------------------

pub fn categorize_cyclomatic(value: u32) -> MetricComplexityCategory {
    match value {
        0..FUNCTION_CYCLOMATIC_MODERATE => MetricComplexityCategory::Low,
        FUNCTION_CYCLOMATIC_MODERATE..FUNCTION_CYCLOMATIC_HIGH => MetricComplexityCategory::Moderate,
        FUNCTION_CYCLOMATIC_HIGH..FUNCTION_CYCLOMATIC_VERY_HIGH => MetricComplexityCategory::High,
        FUNCTION_CYCLOMATIC_VERY_HIGH.. => MetricComplexityCategory::VeryHigh,
    }
}

pub fn categorize_cognitive(value: u32) -> MetricComplexityCategory {
    match value {
        0..FUNCTION_COGNITIVE_MODERATE => MetricComplexityCategory::Low,
        FUNCTION_COGNITIVE_MODERATE..FUNCTION_COGNITIVE_HIGH => MetricComplexityCategory::Moderate,
        FUNCTION_COGNITIVE_HIGH..FUNCTION_COGNITIVE_VERY_HIGH => MetricComplexityCategory::High,
        FUNCTION_COGNITIVE_VERY_HIGH.. => MetricComplexityCategory::VeryHigh,
    }
}

pub fn categorize_total_complexity(value: u32) -> TotalComplexityCategory {
    match value {
        0..FUNCTION_TOTAL_MODERATE => TotalComplexityCategory::Trivial,
        FUNCTION_TOTAL_MODERATE..FUNCTION_TOTAL_HIGH => TotalComplexityCategory::Moderate,
        FUNCTION_TOTAL_HIGH..FUNCTION_TOTAL_EXCESSIVE => TotalComplexityCategory::High,
        FUNCTION_TOTAL_EXCESSIVE.. => TotalComplexityCategory::Excessive,
    }
}

/// Derive recommended action from the worst of three metrics.
pub fn recommended_action_for_function(cyclomatic: u32, cognitive: u32) -> RecommendedAction {
    let total = cyclomatic + cognitive;

    let worst_level = worst_of_three(
        severity_level_cyclomatic(cyclomatic),
        severity_level_cognitive(cognitive),
        severity_level_total(total),
    );

    match worst_level {
        3.. => RecommendedAction::RefactorNow,
        2 => RecommendedAction::ReduceWhenTouched,
        1 => RecommendedAction::Review,
        _ => RecommendedAction::Ignore,
    }
}

fn severity_level_cyclomatic(v: u32) -> u8 {
    match v {
        FUNCTION_CYCLOMATIC_VERY_HIGH.. => 3,
        FUNCTION_CYCLOMATIC_HIGH..FUNCTION_CYCLOMATIC_VERY_HIGH => 2,
        FUNCTION_CYCLOMATIC_MODERATE..FUNCTION_CYCLOMATIC_HIGH => 1,
        _ => 0,
    }
}

fn severity_level_cognitive(v: u32) -> u8 {
    match v {
        FUNCTION_COGNITIVE_VERY_HIGH.. => 3,
        FUNCTION_COGNITIVE_HIGH..FUNCTION_COGNITIVE_VERY_HIGH => 2,
        FUNCTION_COGNITIVE_MODERATE..FUNCTION_COGNITIVE_HIGH => 1,
        _ => 0,
    }
}

fn severity_level_total(v: u32) -> u8 {
    match v {
        FUNCTION_TOTAL_EXCESSIVE.. => 3,
        FUNCTION_TOTAL_HIGH..FUNCTION_TOTAL_EXCESSIVE => 2,
        FUNCTION_TOTAL_MODERATE..FUNCTION_TOTAL_HIGH => 1,
        _ => 0,
    }
}

fn worst_of_three(a: u8, b: u8, c: u8) -> u8 {
    a.max(b).max(c)
}

// ---------------------------------------------------------------------------
// Function entry builder
// ---------------------------------------------------------------------------

pub fn build_function_entry(
    name: String,
    line: usize,
    length: usize,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
) -> FunctionEntry {
    let total_complexity = cyclomatic + cognitive;

    FunctionEntry {
        name,
        line,
        length,
        cyclomatic,
        cyclomatic_category: categorize_cyclomatic(cyclomatic),
        cognitive,
        cognitive_category: categorize_cognitive(cognitive),
        nesting,
        total_complexity,
        total_complexity_category: categorize_total_complexity(total_complexity),
        recommended_action: recommended_action_for_function(cyclomatic, cognitive),
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Round to two decimal places for stable JSON output.
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
