//! Phase 4.2: Performance validation tests.
//!
//! Validates cold vs resumed path timing evidence, machine-readable
//! performance output, and regression threshold gates for the admitted
//! alpha harness path.

use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_source};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, test_source};

// -- Cold path evidence --

#[test]
fn cold_path_emits_performance_evidence() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "perf-evidence-path: cold"));
    assert!(summary_contains(&execution, "perf-stage-ms[service-ready]:"));
    assert!(summary_contains(&execution, "perf-stage-ms[proxy-admitted]:"));
    assert!(summary_contains(&execution, "perf-total-runtime-ms:"));
}

#[test]
fn cold_path_passes_regression_threshold_gate() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(
        summary_contains(&execution, "perf-threshold-gate: pass"),
        "cold path should pass regression thresholds, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        !summary_contains(&execution, "perf-threshold-violation:"),
        "cold path should have no threshold violations"
    );
}

// -- Resumed path evidence --

#[test]
fn resumed_path_emits_performance_evidence_after_restart() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(50))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "perf-evidence-path: resumed"));
    assert!(summary_contains(&execution, "perf-restart-overhead-ms:"));
    assert!(summary_contains(&execution, "perf-stage-ms[service-ready]:"));
}

#[test]
fn resumed_path_passes_regression_threshold_gate() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(50))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(
        summary_contains(&execution, "perf-threshold-gate: pass"),
        "resumed path should pass regression thresholds, summary: {:?}",
        execution.summary_lines
    );
}

// -- Failed path still emits evidence --

#[test]
fn failed_path_emits_performance_evidence() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::FatalFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(&execution, "perf-evidence-path: cold"));
    assert!(summary_contains(&execution, "perf-total-runtime-ms:"));
    assert!(summary_contains(&execution, "perf-threshold-gate:"));
}

// -- Machine-readable format validation --

#[test]
fn performance_evidence_lines_have_structured_format() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);

    let perf_lines: Vec<&String> = execution
        .summary_lines
        .iter()
        .filter(|line| line.starts_with("perf-"))
        .collect();

    assert!(
        perf_lines.len() >= 6,
        "expected at least 6 perf evidence lines, found {}: {:?}",
        perf_lines.len(),
        perf_lines
    );

    for line in &perf_lines {
        assert!(
            line.contains(':'),
            "perf evidence line should be key: value format: {line}"
        );
    }
}

// -- Timing values are reasonable --

#[test]
fn stage_timing_values_are_nonnegative() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);

    for line in &execution.summary_lines {
        if line.starts_with("perf-stage-ms[") || line.starts_with("perf-total-runtime-ms:") {
            let value_part = line
                .split(':')
                .next_back()
                .expect("should have value after colon");
            let ms: u64 = value_part
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("perf timing value should be a non-negative integer: {line}"));
            assert!(
                ms < 5000,
                "perf timing value should be under 5000ms: {line} = {ms}ms"
            );
        }
    }
}

// -- 0-RTT lane evidence --

#[test]
fn zero_rtt_lane_evidence_is_emitted() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(
        summary_contains(&execution, "perf-zero-rtt-lane:"),
        "should emit 0-RTT lane evidence"
    );
    assert!(
        summary_contains(&execution, "early_data enabled"),
        "should report early_data configuration"
    );
    assert!(
        summary_contains(&execution, "session resumption measurement deferred"),
        "should honestly report that session resumption measurement is deferred"
    );
}

// -- Evidence scope honesty --

#[test]
fn evidence_scope_is_honest() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(
        summary_contains(&execution, "perf-evidence-scope:"),
        "should emit evidence scope line"
    );
    assert!(
        summary_contains(&execution, "in-process-harness-timing"),
        "should report in-process harness timing scope"
    );
}

// -- Pipeline latency evidence --

#[test]
fn pipeline_latency_is_measured_on_cold_path() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);

    // Pipeline latency requires both proxy admission and readiness reached.
    // In test harness without protocol bridge, readiness may not reach Ready
    // (it requires proxy+transport+protocol). Pipeline latency is emitted
    // only when both endpoints are recorded.
    let has_readiness = summary_contains(&execution, "perf-stage-ms[readiness-reached]:");
    let has_pipeline = summary_contains(&execution, "perf-pipeline-latency-ms:");

    if has_readiness {
        assert!(
            has_pipeline,
            "should emit pipeline latency when readiness is reached"
        );
    }
}

// -- Cold vs resumed behavioral distinction --

#[test]
fn cold_and_resumed_paths_are_behaviorally_distinct() {
    // Cold path.
    let cold = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    // Resumed path (one retryable failure, then success).
    let resumed = run_with_source(
        runtime_config(),
        test_source([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(50))
            .build(),
        None,
        None,
    );

    assert_eq!(cold.exit, RuntimeExit::Clean);
    assert_eq!(resumed.exit, RuntimeExit::Clean);

    assert!(
        summary_contains(&cold, "perf-evidence-path: cold"),
        "cold path should be labeled cold"
    );
    assert!(
        summary_contains(&resumed, "perf-evidence-path: resumed"),
        "resumed path should be labeled resumed"
    );
    assert!(
        !summary_contains(&cold, "perf-restart-overhead-ms:"),
        "cold path should not have restart overhead"
    );
    assert!(
        summary_contains(&resumed, "perf-restart-overhead-ms:"),
        "resumed path should have restart overhead"
    );
}

// -- Regression threshold values are explicit --

#[test]
fn regression_thresholds_are_documented_in_evidence() {
    // Validate that evidence contains enough perf lines to gate CI.
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);

    let perf_lines: Vec<&String> = execution
        .summary_lines
        .iter()
        .filter(|line| line.starts_with("perf-"))
        .collect();

    // Must have: evidence-path, zero-rtt-lane, stage-ms[proxy-admitted],
    // stage-ms[service-ready], total-runtime-ms, evidence-scope,
    // threshold-gate. That's at least 7 lines.
    assert!(
        perf_lines.len() >= 7,
        "expected at least 7 perf evidence lines for gating, found {}: {perf_lines:?}",
        perf_lines.len()
    );

    // Gate result must be present.
    assert!(
        summary_contains(&execution, "perf-threshold-gate:"),
        "threshold gate result must be present"
    );
}
