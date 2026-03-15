use super::{
    CfdRsMemory, DebtmapCiGateRequest, DebtmapCodeSmellsRequest, DebtmapFileSummaryRequest,
    DebtmapFunctionComplexityRequest, DebtmapHotspotsRequest, DebtmapTouchedFilesRequest,
    DebtmapUnifiedAnalysisRequest, Parameters,
};
#[cfg(feature = "debtmap")]
use super::{log, path_error, repo, to_json};
#[cfg(feature = "debtmap")]
use crate::cogload;
use rmcp::{tool, tool_router};

#[tool_router(router = debtmap_tools_router, vis = "pub")]
impl CfdRsMemory {
    #[tool(
        description = "Return top cognitive-load hotspot files for the repo or a bounded path prefix. Each \
                       file includes score, score_category \
                       (negligible/reviewable/hotspot/high_hotspot/critical_hotspot), and \
                       recommended_action. Scores >= 30.0 are high priority, >= 45.0 are refactor-now, >= \
                       75.0 are critical. Use for refactor triage, not as always-on context."
    )]
    async fn debtmap_top_hotspots(
        &self,
        Parameters(DebtmapHotspotsRequest { limit, path_prefix }): Parameters<DebtmapHotspotsRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&limit, &path_prefix);
            super::debtmap_unavailable("debtmap_top_hotspots")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_top_hotspots");
            let limit = limit.unwrap_or(10).clamp(1, 50) as usize;

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
                    Err(error) => {
                        span.error(error);
                        return path_error(error, prefix);
                    }
                },
                None => None,
            };

            span.detail(&format!("limit={} prefix={:?}", limit, path_prefix));
            let hotspots = cogload::top_hotspots(&self.repo_root, scope.as_deref(), limit).await;

            span.done(&format!("hotspots={}", hotspots.len()));
            to_json(hotspots)
        }
    }

    #[tool(
        description = "Return a focused debtmap summary for one file, including per-function complexity \
                       breakdown, code smells, TODO locations, and long-function line numbers. Use to \
                       understand why a file scores high and identify specific functions to fix."
    )]
    async fn debtmap_file_summary(
        &self,
        Parameters(DebtmapFileSummaryRequest { path }): Parameters<DebtmapFileSummaryRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            super::debtmap_unavailable("debtmap_file_summary")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_file_summary");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
                Err(error) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

            match cogload::file_summary(&self.repo_root, &resolved).await {
                Ok(summary) => {
                    span.done(&format!("path={} score={}", summary.path, summary.score));
                    to_json(summary)
                }
                Err(error) => {
                    span.error(error);
                    path_error(error, &path)
                }
            }
        }
    }

    #[tool(
        description = "Score a list of touched files for bounded cognitive-load review. Returns per-file \
                       scores with categories and recommended actions. Files scoring >= 30.0 should be \
                       reduced before merge. Use after editing files to verify cognitive load is acceptable."
    )]
    async fn debtmap_touched_files_review(
        &self,
        Parameters(DebtmapTouchedFilesRequest { paths }): Parameters<DebtmapTouchedFilesRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &paths;
            super::debtmap_unavailable("debtmap_touched_files_review")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_touched_files_review");

            if paths.is_empty() {
                span.error("paths must not be empty");
                return path_error("paths must not be empty", "");
            }

            let resolved = match super::resolve_paths(&self.repo_root, &self.repo_root_canon, &paths) {
                Ok(resolved) => resolved,
                Err((error, path)) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

            let review = cogload::touched_files_review(&self.repo_root, &resolved).await;

            span.done(&format!(
                "files={} total_score={} skipped={}",
                review.files.len(),
                review.total_score,
                review.skipped.len()
            ));
            to_json(review)
        }
    }

    #[tool(description = "Detect code smells in a single file using debtmap AST analysis.")]
    async fn debtmap_code_smells(
        &self,
        Parameters(DebtmapCodeSmellsRequest { path }): Parameters<DebtmapCodeSmellsRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            super::debtmap_unavailable("debtmap_code_smells")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_code_smells");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
                Err(error) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

            match cogload::code_smells(&self.repo_root, &resolved).await {
                Ok(report) => {
                    span.done(&format!("path={} smells={}", report.path, report.total));
                    to_json(report)
                }
                Err(error) => {
                    span.error(error);
                    path_error(error, &path)
                }
            }
        }
    }

    #[tool(description = "Return per-function complexity breakdown for a single file.")]
    async fn debtmap_function_complexity(
        &self,
        Parameters(DebtmapFunctionComplexityRequest { path }): Parameters<DebtmapFunctionComplexityRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = &path;
            super::debtmap_unavailable("debtmap_function_complexity")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_function_complexity");

            let resolved = match repo::resolve(&self.repo_root, &self.repo_root_canon, &path) {
                Ok(path) => path,
                Err(error) => {
                    span.error(error);
                    return path_error(error, &path);
                }
            };

            match cogload::function_complexity(&self.repo_root, &resolved).await {
                Ok(report) => {
                    span.done(&format!(
                        "path={} fn_count={} method={}",
                        report.path, report.fn_count, report.analysis_method
                    ));
                    to_json(report)
                }
                Err(error) => {
                    span.error(error);
                    path_error(error, &path)
                }
            }
        }
    }

    #[tool(description = "Run full unified debtmap analysis for deep structural review.")]
    async fn debtmap_unified_analysis(
        &self,
        Parameters(DebtmapUnifiedAnalysisRequest { limit, path_prefix }): Parameters<
            DebtmapUnifiedAnalysisRequest,
        >,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&limit, &path_prefix);
            super::debtmap_unavailable("debtmap_unified_analysis")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_unified_analysis");
            let limit = limit.unwrap_or(20).clamp(1, 100) as usize;

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
                    Err(error) => {
                        span.error(error);
                        return path_error(error, prefix);
                    }
                },
                None => None,
            };

            span.detail(&format!("limit={} prefix={:?}", limit, path_prefix));

            match cogload::run_unified_analysis(&self.repo_root, scope.as_deref(), limit).await {
                Ok(report) => {
                    span.done(&format!(
                        "items={} density={:.1} loc={}",
                        report.total_items, report.debt_density, report.total_loc
                    ));
                    to_json(report)
                }
                Err(error) => {
                    span.error(&error);
                    to_json(serde_json::json!({ "error": error }))
                }
            }
        }
    }

    #[tool(
        description = "Evaluate debtmap CI gate rules against the repo or a bounded file set. Returns \
                       pass/fail with blocking violations (must fix) and warnings (monitor). Blocking \
                       rules: function score >= 30.0, god_object_score >= 45.0, density > 50.0/1K LOC, \
                       cyclomatic >= 31, cognitive >= 25. Each violation includes path, line, score, and a \
                       suggestion for how to fix it. Run on touched files before completing a task."
    )]
    async fn debtmap_ci_gate(
        &self,
        Parameters(DebtmapCiGateRequest { path_prefix, paths }): Parameters<DebtmapCiGateRequest>,
    ) -> String {
        #[cfg(not(feature = "debtmap"))]
        {
            let _ = (&path_prefix, &paths);
            super::debtmap_unavailable("debtmap_ci_gate")
        }

        #[cfg(feature = "debtmap")]
        {
            let span = log::ToolSpan::start("debtmap_ci_gate");

            let scope = match &path_prefix {
                Some(prefix) => match repo::resolve(&self.repo_root, &self.repo_root_canon, prefix) {
                    Ok(path) => Some(path),
                    Err(error) => {
                        span.error(error);
                        return path_error(error, prefix);
                    }
                },
                None => None,
            };

            let touched_filter: Option<std::collections::HashSet<String>> =
                paths.map(|items| items.into_iter().collect());

            span.detail(&format!(
                "prefix={:?} touched_filter={}",
                path_prefix,
                touched_filter
                    .as_ref()
                    .map_or("none".to_string(), |items| format!("{} files", items.len()))
            ));

            match cogload::run_unified_analysis(&self.repo_root, scope.as_deref(), 500).await {
                Ok(report) => {
                    let gate = cogload::evaluate_ci_gate_filtered(&report, touched_filter.as_ref());
                    span.done(&format!(
                        "pass={} blocking={} warnings={}",
                        gate.pass,
                        gate.blocking.len(),
                        gate.warnings.len()
                    ));
                    to_json(gate)
                }
                Err(error) => {
                    span.error(&error);
                    to_json(serde_json::json!({ "error": error }))
                }
            }
        }
    }
}
