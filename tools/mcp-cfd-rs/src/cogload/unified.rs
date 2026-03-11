use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

// ---------------------------------------------------------------------------
// Output types — aligned with the debtmap CLI `analyze -f json` output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct UnifiedReport {
    pub total_items: usize,
    pub total_debt_score: f64,
    pub debt_density: f64,
    pub total_loc: usize,
    pub items: Vec<UnifiedItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnifiedItem {
    pub item_type: UnifiedItemType,
    pub score: f64,
    pub priority: UnifiedPriority,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub god_object: Option<GodObjectInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupling: Option<CouplingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion: Option<CohesionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ItemMetrics>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedItemType {
    Function,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct GodObjectInfo {
    pub is_god_object: bool,
    pub detection_type: String,
    pub methods: usize,
    pub fields: usize,
    pub responsibilities: usize,
    pub god_object_score: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub responsibility_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CouplingInfo {
    pub afferent: usize,
    pub efferent: usize,
    pub instability: f64,
    pub classification: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CohesionInfo {
    pub score: f64,
    pub internal_calls: usize,
    pub external_calls: usize,
    pub classification: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cyclomatic: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cognitive: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nesting_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blast_radius: Option<usize>,
}

// ---------------------------------------------------------------------------
// CI gate types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CiGateResult {
    pub pass: bool,
    pub blocking: Vec<CiViolation>,
    pub warnings: Vec<CiViolation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CiViolation {
    pub rule: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    pub detail: String,
}

// ---------------------------------------------------------------------------
// Full unified analysis via debtmap crate pipeline
// ---------------------------------------------------------------------------

/// Run the debtmap unified analysis pipeline on the workspace (or a
/// sub-path) and return a structured report aligned with the debtmap CLI
/// `analyze -f json` output.
///
/// This uses the same code path as the CLI: `analyze_project` →
/// `perform_unified_analysis`, giving identical God Object, coupling,
/// cohesion, and call-graph results.
pub async fn run_unified_analysis(
    repo_root: &Path,
    scope: Option<&Path>,
    limit: usize,
) -> Result<UnifiedReport, String> {
    let analysis_path = scope.unwrap_or(repo_root).to_path_buf();
    let repo_root = repo_root.to_path_buf();

    tokio::task::spawn_blocking(move || run_pipeline(&repo_root, &analysis_path, limit))
        .await
        .map_err(|e| format!("analysis task failed: {e}"))?
}

fn run_pipeline(repo_root: &Path, analysis_path: &Path, limit: usize) -> Result<UnifiedReport, String> {
    let languages = vec![
        debtmap::Language::Rust,
        debtmap::Language::JavaScript,
        debtmap::Language::TypeScript,
    ];

    // Read .debtmap.toml (same file the CLI reads) for shared thresholds.
    let (complexity_threshold, duplication_threshold) = load_project_thresholds();

    let results = debtmap::commands::analyze::analyze_project(
        analysis_path.to_path_buf(),
        languages,
        complexity_threshold,
        duplication_threshold,
        true, // parallel
        debtmap::formatting::FormattingConfig::from_env(),
    )
    .map_err(|e| format!("project analysis failed: {e}"))?;

    let unified = debtmap::builders::unified_analysis::perform_unified_analysis(
        &results,
        None,  // no coverage file
        false, // semantic analysis on
        analysis_path,
        false, // no verbose macro warnings
        false, // no macro stats
    )
    .map_err(|e| format!("unified analysis failed: {e}"))?;

    Ok(build_report(repo_root, &unified, limit))
}

/// Load complexity and duplication thresholds from `.debtmap.toml` via the
/// debtmap crate's multi-source config loader.  Falls back to the same
/// defaults the CLI uses when the config file is absent or unreadable.
fn load_project_thresholds() -> (u32, usize) {
    let default_complexity: u32 = 10;
    let default_duplication: usize = 50;

    let Ok(traced) = debtmap::config::load_multi_source_config() else {
        return (default_complexity, default_duplication);
    };

    let cfg = traced.config();

    let complexity = cfg
        .thresholds
        .as_ref()
        .and_then(|t| t.complexity)
        .unwrap_or(default_complexity);

    let duplication = cfg
        .thresholds
        .as_ref()
        .and_then(|t| t.duplication)
        .map(|v| v as usize)
        .unwrap_or(default_duplication);

    (complexity, duplication)
}

fn build_report(
    repo_root: &Path,
    unified: &debtmap::priority::UnifiedAnalysis,
    limit: usize,
) -> UnifiedReport {
    let mut items = Vec::new();

    // Collect function-level items
    for item in &unified.items {
        items.push(convert_function_item(repo_root, item));
    }

    // Collect file-level items
    for item in &unified.file_items {
        items.push(convert_file_item(repo_root, item));
    }

    // Sort by score descending, then truncate
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    items.truncate(limit);

    UnifiedReport {
        total_items: items.len(),
        total_debt_score: super::scoring::round2(unified.total_debt_score),
        debt_density: super::scoring::round2(unified.debt_density),
        total_loc: unified.total_lines_of_code,
        items,
    }
}

fn convert_function_item(repo_root: &Path, item: &debtmap::priority::UnifiedDebtItem) -> UnifiedItem {
    let path = relativize(&item.location.file, repo_root);

    let god_object = item
        .god_object_indicators
        .as_ref()
        .filter(|go| go.is_god_object)
        .map(|go| GodObjectInfo {
            is_god_object: true,
            detection_type: format!("{:?}", go.detection_type),
            methods: go.method_count,
            fields: go.field_count,
            responsibilities: go.responsibility_count,
            god_object_score: go.god_object_score,
            responsibility_names: go.responsibilities.clone(),
        });

    let blast_radius = item.upstream_dependencies + item.downstream_dependencies;
    let coupling = if item.upstream_dependencies > 0 || item.downstream_dependencies > 0 {
        let total = item.upstream_dependencies + item.downstream_dependencies;
        let instability = if total > 0 {
            item.downstream_dependencies as f64 / total as f64
        } else {
            0.0
        };
        Some(CouplingInfo {
            afferent: item.upstream_dependencies,
            efferent: item.downstream_dependencies,
            instability,
            classification: classify_coupling(item.upstream_dependencies + item.downstream_dependencies),
        })
    } else {
        None
    };

    let metrics = Some(ItemMetrics {
        cyclomatic: Some(item.cyclomatic_complexity),
        cognitive: Some(item.cognitive_complexity),
        lines: Some(item.function_length),
        functions: None,
        nesting_depth: Some(item.nesting_depth),
        blast_radius: Some(blast_radius),
    });

    UnifiedItem {
        item_type: UnifiedItemType::Function,
        score: super::scoring::round2(item.unified_score.final_score),
        priority: score_to_priority(item.unified_score.final_score),
        path,
        line: Some(item.location.line),
        function: Some(item.location.function.clone()),
        god_object,
        coupling,
        cohesion: None,
        metrics,
    }
}

fn convert_file_item(repo_root: &Path, item: &debtmap::priority::FileDebtItem) -> UnifiedItem {
    let m = &item.metrics;
    let path = relativize(&m.path, repo_root);

    let god_object = m
        .god_object_analysis
        .as_ref()
        .filter(|go| go.is_god_object)
        .map(|go| GodObjectInfo {
            is_god_object: true,
            detection_type: format!("{:?}", go.detection_type),
            methods: go.method_count,
            fields: go.field_count,
            responsibilities: go.responsibility_count,
            god_object_score: go.god_object_score,
            responsibility_names: go.responsibilities.clone(),
        });

    let coupling = if m.afferent_coupling > 0 || m.efferent_coupling > 0 {
        Some(CouplingInfo {
            afferent: m.afferent_coupling,
            efferent: m.efferent_coupling,
            instability: m.instability,
            classification: classify_coupling(m.afferent_coupling + m.efferent_coupling),
        })
    } else {
        None
    };

    let metrics = Some(ItemMetrics {
        cyclomatic: Some(m.total_complexity),
        cognitive: None,
        lines: Some(m.total_lines),
        functions: Some(m.function_count),
        nesting_depth: None,
        blast_radius: None,
    });

    UnifiedItem {
        item_type: UnifiedItemType::File,
        score: super::scoring::round2(item.score),
        priority: score_to_priority(item.score),
        path,
        line: None,
        function: None,
        god_object,
        coupling,
        cohesion: None,
        metrics,
    }
}

/// Derive priority from the unified score value.
fn score_to_priority(score: f64) -> UnifiedPriority {
    if score >= 75.0 {
        return UnifiedPriority::Critical;
    }
    if score >= 45.0 {
        return UnifiedPriority::High;
    }
    if score >= 15.0 {
        return UnifiedPriority::Medium;
    }
    UnifiedPriority::Low
}

/// Classify coupling intensity from total dependency count.
fn classify_coupling(total: usize) -> String {
    match total {
        0..=4 => "low",
        5..=9 => "moderate",
        10..=19 => "highly_coupled",
        _ => "Hub",
    }
    .to_string()
}

fn relativize(file_path: &Path, repo_root: &Path) -> String {
    if let Ok(rel) = file_path.strip_prefix(repo_root) {
        rel.display().to_string()
    } else {
        let s = file_path.display().to_string();
        s.strip_prefix("./").unwrap_or(&s).to_string()
    }
}

// ---------------------------------------------------------------------------
// CI gate evaluation
// ---------------------------------------------------------------------------

/// Evaluate CI gate rules against a unified report.
///
/// Blocking rules (cause CI failure):
/// - priority `critical` or `high`
/// - god_object_score >= 45.0
/// - debt_density > 50.0 per 1K LOC
/// - function cyclomatic >= 31 or cognitive >= 25
///
/// Warning rules (visible but non-blocking):
/// - priority `medium`
/// - god_object_score < 45.0 (monitor)
/// - coupling classification `highly_coupled` or `Hub`
/// - function cyclomatic 21-30 or cognitive 15-24
pub fn evaluate_ci_gate(report: &UnifiedReport) -> CiGateResult {
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();

    // Density gate
    if report.debt_density > 50.0 {
        blocking.push(CiViolation {
            rule: "debt_density".to_string(),
            path: String::new(),
            function: None,
            detail: format!(
                "debt density {:.1} per 1K LOC exceeds limit of 50.0",
                report.debt_density
            ),
        });
    }

    for item in &report.items {
        evaluate_item_gates(item, &mut blocking, &mut warnings);
    }

    let pass = blocking.is_empty();

    CiGateResult {
        pass,
        blocking,
        warnings,
    }
}

/// Evaluate CI gate rules, optionally filtering violations to only the
/// given set of repo-relative paths.  When a filter is provided the
/// whole-scope `debt_density` gate is skipped because it is not
/// meaningful for a subset of files.
pub fn evaluate_ci_gate_filtered(report: &UnifiedReport, touched: Option<&HashSet<String>>) -> CiGateResult {
    let mut raw = evaluate_ci_gate(report);

    let Some(touched) = touched else {
        return raw;
    };

    let keep = |v: &CiViolation| -> bool {
        if v.path.is_empty() {
            // Whole-scope metrics (e.g. debt_density) — skip when filtering.
            return false;
        }
        touched.contains(&v.path)
    };

    raw.blocking.retain(keep);
    raw.warnings.retain(keep);
    raw.pass = raw.blocking.is_empty();
    raw
}

fn evaluate_item_gates(item: &UnifiedItem, blocking: &mut Vec<CiViolation>, warnings: &mut Vec<CiViolation>) {
    let path = &item.path;
    let function = item.function.clone();

    // Priority gates
    match item.priority {
        UnifiedPriority::Critical | UnifiedPriority::High => {
            blocking.push(CiViolation {
                rule: format!("priority_{:?}", item.priority).to_lowercase(),
                path: path.clone(),
                function: function.clone(),
                detail: format!("score {:.1}, priority {:?}", item.score, item.priority),
            });
        }
        UnifiedPriority::Medium => {
            warnings.push(CiViolation {
                rule: "priority_medium".to_string(),
                path: path.clone(),
                function: function.clone(),
                detail: format!("score {:.1}", item.score),
            });
        }
        UnifiedPriority::Low => {}
    }

    // God object gates
    if let Some(god) = &item.god_object {
        if god.god_object_score >= 45.0 {
            blocking.push(CiViolation {
                rule: "god_object_blocking".to_string(),
                path: path.clone(),
                function: function.clone(),
                detail: format!(
                    "{} with score {:.1} (methods={}, responsibilities={})",
                    god.detection_type, god.god_object_score, god.methods, god.responsibilities,
                ),
            });
        } else {
            warnings.push(CiViolation {
                rule: "god_object_watch".to_string(),
                path: path.clone(),
                function: function.clone(),
                detail: format!(
                    "{} with score {:.1} (methods={}, responsibilities={})",
                    god.detection_type, god.god_object_score, god.methods, god.responsibilities,
                ),
            });
        }
    }

    // Coupling warnings
    if let Some(coupling) = &item.coupling {
        let cls = coupling.classification.as_str();

        if cls == "highly_coupled" || cls == "Hub" {
            warnings.push(CiViolation {
                rule: "coupling_watch".to_string(),
                path: path.clone(),
                function: function.clone(),
                detail: format!(
                    "classification={}, Ca={}, Ce={}, instability={:.2}",
                    cls, coupling.afferent, coupling.efferent, coupling.instability,
                ),
            });
        }
    }

    // Function-level complexity gates — only apply to individual function
    // items. File-level items carry aggregate totals that must not be compared
    // against per-function thresholds. God-object aggregates (impl-block
    // roll-ups) are already covered by the god_object gate above.
    if matches!(item.item_type, UnifiedItemType::File) {
        return;
    }

    if item.god_object.is_some() {
        return;
    }

    let Some(metrics) = &item.metrics else {
        return;
    };

    let cyc = metrics.cyclomatic.unwrap_or(0);
    let cog = metrics.cognitive.unwrap_or(0);

    if cyc >= 31 || cog >= 25 {
        blocking.push(CiViolation {
            rule: "function_complexity_blocking".to_string(),
            path: path.clone(),
            function: function.clone(),
            detail: format!("cyclomatic={}, cognitive={}", cyc, cog),
        });
        return;
    }

    if cyc >= 21 || cog >= 15 {
        warnings.push(CiViolation {
            rule: "function_complexity_watch".to_string(),
            path: path.clone(),
            function: function.clone(),
            detail: format!("cyclomatic={}, cognitive={}", cyc, cog),
        });
    }
}
