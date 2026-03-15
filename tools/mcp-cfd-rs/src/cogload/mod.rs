mod analysis;
mod collection;
mod queries;
pub mod scoring;
pub mod types;
pub mod unified;

// Re-export the public query API — this is what `server.rs` calls.
pub use queries::{code_smells, file_summary, function_complexity, top_hotspots, touched_files_review};
pub use unified::{evaluate_ci_gate_filtered, run_unified_analysis};

// ---------------------------------------------------------------------------
// Tests — kept here to exercise the integrated behavior across submodules.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::analysis::*;
    use super::scoring::*;
    use super::types::*;
    use std::path::Path;

    #[test]
    fn crate_analyzes_rust_code() {
        let text = "fn main() {\n    if true {\n        println!(\"hello\");\n    }\n}\n";
        let path = Path::new("test.rs");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some(), "Rust should be analyzed by the crate");
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 1);
        assert!(a.line_count >= 5);
        assert!(a.total_cyclomatic > 0);
    }

    #[test]
    fn manual_fallback_for_go() {
        let text = "func main() {\n    fmt.Println(\"hello\")\n}\n";
        let path = Path::new("main.go");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_none(), "Go should fall back to manual");

        let m = analyze_manual(text);
        assert_eq!(m.fn_count, 1);
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn scores_simple_text() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let metrics = analyze_manual(text);

        assert_eq!(metrics.line_count, 3);
        assert_eq!(metrics.fn_count, 1);
        assert_eq!(metrics.todo_count, 0);
        assert!(metrics.max_indent_depth >= 1);
        assert!(compute_score_manual(&metrics) > 0.0);
    }

    #[test]
    fn counts_todos_and_fixmes() {
        let text = "// TODO: fix this\nlet x = 1;\n// FIXME: broken\n";
        let metrics = analyze_manual(text);

        assert_eq!(metrics.todo_count, 2);
    }

    #[test]
    fn counts_various_fn_forms() {
        let text = "\
fn foo() {}
pub fn bar() {}
async fn baz() {}
pub async fn qux() {}
pub(crate) fn internal() {}
pub(crate) async fn internal_async() {}
func goFunc() {}
def pyFunc():
";
        let metrics = analyze_manual(text);

        assert_eq!(metrics.fn_count, 8);
    }

    #[test]
    fn collects_todo_entries_with_line_numbers() {
        let text = "line one\n// TODO: first\nline three\n// FIXME: second\n";
        let todos = collect_todos(text, 10);

        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].line, 2);
        assert_eq!(todos[1].line, 4);
    }

    #[test]
    fn detects_long_functions() {
        // 65-line function body should exceed the 60-line threshold.
        let mut lines = vec!["fn long_fn() {".to_string()];
        for i in 0..63 {
            lines.push(format!("    let x{i} = {i};"));
        }
        lines.push("}".to_string());
        let text = lines.join("\n");

        let long = collect_long_fns(&text, 60);

        assert_eq!(long.len(), 1);
        assert_eq!(long[0], 1);
    }

    #[test]
    fn score_increases_with_complexity() {
        let simple = "fn a() {}\n";
        let complex = "fn a() {\n    // TODO: fix\n    if true {\n        if true {\n            let x = \
                       1;\n        }\n    }\n}\nfn b() {}\nfn c() {}\n";

        let simple_score = compute_score_manual(&analyze_manual(simple));
        let complex_score = compute_score_manual(&analyze_manual(complex));

        assert!(complex_score > simple_score);
    }

    #[test]
    fn empty_text_scores_zero() {
        let metrics = analyze_manual("");

        assert_eq!(metrics.line_count, 0);
        assert_eq!(compute_score_manual(&metrics), 0.0);
    }

    #[test]
    fn crate_detects_code_smells_in_rust() {
        let text = "fn main() {\n    // TODO: fix this later\n    let x = 42;\n}\n";
        let path = Path::new("test.rs");
        let smells = debtmap::find_code_smells(text, path);

        // The call succeeds without panicking.
        let _ = smells;
    }

    #[test]
    fn crate_finds_todos_and_fixmes() {
        let text = "fn main() {\n    // TODO: first\n    // FIXME: second\n}\n";
        let path = Path::new("test.rs");
        let todos = debtmap::find_todos_and_fixmes(text, path);

        assert!(todos.len() >= 2);
    }

    #[test]
    fn crate_reports_function_complexity() {
        let text = r#"
fn simple() {
    let x = 1;
}

fn complex(a: i32, b: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            match a + b {
                0 => 0,
                1 => 1,
                _ => a + b,
            }
        } else {
            -1
        }
    } else {
        0
    }
}
"#;
        let path = Path::new("test.rs");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some());
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 2);

        let complex_fn = a.functions.iter().find(|f| f.name == "complex");
        assert!(complex_fn.is_some());
        let cf = complex_fn.expect("complex function should be found");
        assert!(cf.cyclomatic > 1, "complex fn should have cyclomatic > 1");
    }

    #[test]
    fn crate_analyzes_javascript_code() {
        let text = "function sum(a, b) {\n  if (a > 0) {\n    return a + b;\n  }\n  return b;\n}\n";
        let path = Path::new("test.js");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_some(), "JavaScript should be analyzed by the crate");
        let a = analysis.expect("analysis should be Some");
        assert!(a.fn_count >= 1);
        assert!(a.total_cyclomatic > 0);
    }

    #[test]
    fn python_uses_manual_fallback_until_supported() {
        let text = "def add(a, b):\n    return a + b\n";
        let path = Path::new("test.py");
        let analysis = analyze_with_crate(text, path);

        assert!(analysis.is_none(), "Python should use manual fallback for now");

        let score = score_text(
            Path::new("/tmp/test_repo"),
            Path::new("/tmp/test_repo/test.py"),
            text,
        );
        assert_eq!(score.analysis_method, "heuristic");
    }

    #[test]
    fn unified_scoring_uses_ast_for_rust() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let root = Path::new("/tmp/test_repo");
        let path = Path::new("/tmp/test_repo/src/main.rs");
        let score = score_text(root, path, text);

        assert_eq!(score.analysis_method, "ast");
        assert!(score.avg_cyclomatic.is_some());
    }

    #[test]
    fn unified_scoring_uses_heuristic_for_go() {
        let text = "func main() {\n    fmt.Println(\"hello\")\n}\n";
        let root = Path::new("/tmp/test_repo");
        let path = Path::new("/tmp/test_repo/main.go");
        let score = score_text(root, path, text);

        assert_eq!(score.analysis_method, "heuristic");
        assert!(score.avg_cyclomatic.is_none());
    }

    #[test]
    fn file_score_categories_match_thresholds() {
        assert!(matches!(
            categorize_file_score(14.99),
            FileScoreCategory::Negligible
        ));
        assert!(matches!(
            categorize_file_score(15.0),
            FileScoreCategory::Reviewable
        ));
        assert!(matches!(categorize_file_score(30.0), FileScoreCategory::Hotspot));
        assert!(matches!(
            categorize_file_score(45.0),
            FileScoreCategory::HighHotspot
        ));
        assert!(matches!(
            categorize_file_score(75.0),
            FileScoreCategory::CriticalHotspot
        ));
    }

    #[test]
    fn file_score_actions_match_operational_limits() {
        assert!(matches!(
            recommended_action_for_file_score(10.0),
            RecommendedAction::Ignore
        ));
        assert!(matches!(
            recommended_action_for_file_score(20.0),
            RecommendedAction::Review
        ));
        assert!(matches!(
            recommended_action_for_file_score(35.0),
            RecommendedAction::ReduceWhenTouched
        ));
        assert!(matches!(
            recommended_action_for_file_score(45.0),
            RecommendedAction::RefactorNow
        ));
    }

    #[test]
    fn function_entry_carries_categories_and_action() {
        // cyclomatic=31 → VeryHigh (≥31), cognitive=25 → VeryHigh (≥25)
        let entry = build_function_entry("complex".to_string(), 10, 40, 31, 25, 4);

        assert_eq!(entry.total_complexity, 56);
        assert!(matches!(
            entry.cyclomatic_category,
            MetricComplexityCategory::VeryHigh
        ));
        assert!(matches!(
            entry.cognitive_category,
            MetricComplexityCategory::VeryHigh
        ));
        assert!(matches!(
            entry.total_complexity_category,
            TotalComplexityCategory::Excessive
        ));
        assert!(matches!(entry.recommended_action, RecommendedAction::RefactorNow));
    }

    #[test]
    fn marker_debt_excluded_from_file_score() {
        assert!(is_marker_debt(&debtmap::DebtType::Todo { reason: None }));
        assert!(is_marker_debt(&debtmap::DebtType::Fixme { reason: None }));
        assert!(is_marker_debt(&debtmap::DebtType::TestTodo {
            priority: debtmap::core::Priority::Low,
            reason: None,
        }));
        assert!(!is_marker_debt(&debtmap::DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 5,
        }));
        assert!(!is_marker_debt(&debtmap::DebtType::CodeSmell {
            smell_type: None,
        }));
    }

    #[test]
    fn todos_do_not_inflate_crate_file_score() {
        let text = "\
fn main() {
    // TODO: port this
    // TODO: port that
    // FIXME: fixme later
    // TODO: another one
    // TODO: fifth marker
    let x = 1;
}
";
        let path = Path::new("test.rs");
        let analysis = analyze_with_crate(text, path).expect("Rust should be analyzed");
        let score = compute_score_crate(&analysis);

        assert!(
            score < FILE_REDUCE_WHEN_TOUCHED_SCORE,
            "file with trivial code + many TODOs should score below {FILE_REDUCE_WHEN_TOUCHED_SCORE}, got \
             {score}",
        );
    }

    #[test]
    fn ci_gate_blocks_on_high_density() {
        use super::unified::*;

        let report = UnifiedReport {
            total_items: 0,
            total_debt_score: 100.0,
            debt_density: 75.0,
            total_loc: 1000,
            items: Vec::new(),
        };

        let result = evaluate_ci_gate(&report);
        assert!(!result.pass);
        assert!(result.blocking.iter().any(|v| v.rule == "debt_density"));
        assert!(result.thresholds.density_limit == 50.0);
        assert!(result.thresholds.score_blocking == 30.0);
    }

    #[test]
    fn ci_gate_blocks_on_god_object_high_score() {
        use super::unified::*;

        let report = UnifiedReport {
            total_items: 1,
            total_debt_score: 50.0,
            debt_density: 10.0,
            total_loc: 5000,
            items: vec![UnifiedItem {
                item_type: UnifiedItemType::File,
                score: 88.5,
                priority: UnifiedPriority::Critical,
                path: "src/big.rs".to_string(),
                line: None,
                function: None,
                god_object: Some(GodObjectInfo {
                    is_god_object: true,
                    detection_type: "GodModule".to_string(),
                    methods: 55,
                    fields: 77,
                    responsibilities: 6,
                    god_object_score: 88.5,
                    responsibility_names: vec!["Computation".to_string()],
                }),
                coupling: None,
                cohesion: None,
                metrics: None,
            }],
        };

        let result = evaluate_ci_gate(&report);
        assert!(!result.pass);
        let god_violation = result
            .blocking
            .iter()
            .find(|v| v.rule == "god_object_blocking")
            .expect("should have god_object_blocking");
        assert!(!god_violation.suggestion.is_empty());
    }

    #[test]
    fn ci_gate_warns_on_god_object_low_score() {
        use super::unified::*;

        let report = UnifiedReport {
            total_items: 1,
            total_debt_score: 30.0,
            debt_density: 10.0,
            total_loc: 3000,
            items: vec![UnifiedItem {
                item_type: UnifiedItemType::File,
                score: 27.0,
                priority: UnifiedPriority::Medium,
                path: "src/status.rs".to_string(),
                line: None,
                function: None,
                god_object: Some(GodObjectInfo {
                    is_god_object: true,
                    detection_type: "GodClass".to_string(),
                    methods: 21,
                    fields: 12,
                    responsibilities: 8,
                    god_object_score: 30.0,
                    responsibility_names: Vec::new(),
                }),
                coupling: None,
                cohesion: None,
                metrics: None,
            }],
        };

        let result = evaluate_ci_gate(&report);
        // medium priority doesn't block, god_object_score < 45 doesn't block
        assert!(result.warnings.iter().any(|v| v.rule == "god_object_watch"));
    }

    #[test]
    fn ci_gate_passes_clean_report() {
        use super::unified::*;

        let report = UnifiedReport {
            total_items: 0,
            total_debt_score: 5.0,
            debt_density: 3.0,
            total_loc: 1500,
            items: Vec::new(),
        };

        let result = evaluate_ci_gate(&report);
        assert!(result.pass);
        assert!(result.blocking.is_empty());
    }
}
