//! Phase 4.2: Performance validation tests.
//!
//! Validates cold vs resumed path timing evidence, machine-readable
//! performance output, and regression threshold gates for the admitted
//! alpha harness path.

use std::time::Duration;

use crate::runtime::{RuntimeExit, RuntimeHarness, run_with_factory};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, TestFactory};

// -- Cold path evidence --

#[test]
fn cold_path_emits_performance_evidence() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "perf-evidence-path: resumed"));
    assert!(summary_contains(&execution, "perf-restart-overhead-ms:"));
    assert!(summary_contains(&execution, "perf-stage-ms[service-ready]:"));
}

#[test]
fn resumed_path_passes_regression_threshold_gate() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::FatalFailure]),
        RuntimeHarness::for_tests(),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);

    let perf_lines: Vec<&String> = execution
        .summary_lines
        .iter()
        .filter(|line| line.starts_with("perf-"))
        .collect();

    assert!(
        perf_lines.len() >= 4,
        "expected at least 4 perf evidence lines, found {}: {:?}",
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
