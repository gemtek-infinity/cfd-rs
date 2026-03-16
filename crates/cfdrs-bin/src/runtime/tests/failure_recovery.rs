//! Phase 4.3: Failure-mode and recovery proof tests.
//!
//! Proves the admitted alpha surface behaves sanely under disruption:
//! - reconnect/retry behavior is bounded and visible
//! - shutdown behavior is observable under realistic states
//! - dependency-boundary failures are visible at the correct owner
//! - config-reload is honestly declared as not supported
//! - failure evidence is machine-readable and scoped honestly

use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_source};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, test_source};

// -- Reconnect / retry proof --

#[test]
fn restart_exhaustion_is_bounded_and_visible() {
    // Policy allows max 2 restart attempts. Supply 3 retryable failures
    // so the budget is exhausted before any success.
    let execution = run_with_source(
        runtime_config(),
        test_source([
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
        ]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(
        matches!(execution.exit, RuntimeExit::Failed { .. }),
        "should fail after exhausting restart budget"
    );
    assert!(
        summary_contains(&execution, "exhausted restart policy after 2 attempts"),
        "should report exhaustion with attempt count, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "failure-restart-budget: used=2 max=2 exhausted=true"),
        "failure evidence should report exhausted restart budget, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn each_retryable_failure_records_failure_visibility() {
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
        summary_contains(
            &execution,
            "failure-visibility: owner=test-service class=retryable"
        ),
        "retryable failure should have a failure-visibility line"
    );
    assert!(
        summary_contains(&execution, "failure-events-total: 1"),
        "should count exactly one failure event, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn restart_resets_lifecycle_to_starting() {
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
        summary_contains(&execution, "restarting test-service after retryable failure"),
        "should log restart reason"
    );
    assert!(
        summary_contains(&execution, "supervision-restart-attempt: 1"),
        "should record restart attempt number"
    );
}

#[test]
fn restart_budget_zero_means_no_recovery() {
    // Even with zero restarts exhausted, a clean path has no exhaustion.
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
        summary_contains(&execution, "failure-restart-budget: used=0 max=2 exhausted=false"),
        "clean path should show no restarts used, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn transport_failure_counter_tracks_retryable_exits() {
    let execution = run_with_source(
        runtime_config(),
        test_source([
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
            TestBehavior::WaitForShutdown,
        ]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(75))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(
        summary_contains(&execution, "failure-transport-failures: 2"),
        "should track transport failure counter across retries, summary: {:?}",
        execution.summary_lines
    );
}

// -- Shutdown proof --

#[test]
fn shutdown_during_starting_state_is_clean() {
    // Immediate shutdown injection with no service ready delay.
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(5))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "shutdown-reason: harness"));
    assert!(
        summary_contains(&execution, "lifecycle-state: stopped"),
        "should reach stopped state after clean shutdown"
    );
}

#[test]
fn shutdown_records_child_task_cleanup() {
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
        summary_contains(&execution, "child-task-stopped:"),
        "should record child task cleanup during shutdown drain"
    );
}

#[test]
fn shutdown_after_fatal_records_failure_state() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::FatalFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(&execution, "lifecycle-state: failed"),
        "should reach failed state after fatal exit"
    );
    assert!(
        summary_contains(&execution, "readiness-state: failed"),
        "readiness should reflect failure"
    );
}

#[test]
fn shutdown_after_restart_exhaustion_records_failure() {
    let execution = run_with_source(
        runtime_config(),
        test_source([
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
        ]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(&execution, "lifecycle-state: failed"),
        "should reach failed state after restart exhaustion"
    );
}

// -- Dependency-boundary failure visibility --

#[test]
fn deferred_service_exit_reports_boundary() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::DeferredExit]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(
        matches!(execution.exit, RuntimeExit::Deferred { .. }),
        "should exit with deferred status"
    );
    assert!(
        summary_contains(
            &execution,
            "failure-visibility: owner=test-service class=deferred"
        ),
        "should record deferred failure boundary, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "deferred boundary reached in test"),
        "should include deferred detail in summary"
    );
}

#[test]
fn control_plane_failure_is_visible() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::ControlPlaneFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(
            &execution,
            "failure-visibility: owner=runtime-control-plane class=fatal"
        ),
        "should record control plane failure boundary, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn fatal_failure_records_dependency_boundary() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::FatalFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(&execution, "failure-visibility: owner=test-service class=fatal"),
        "fatal failure should record dependency boundary"
    );
    assert!(
        summary_contains(&execution, "failure-events-total: 1"),
        "should count fatal as a failure event, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn dependency_boundary_summary_is_emitted() {
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
        summary_contains(&execution, "failure-dependency-boundaries:"),
        "should emit dependency boundary summary, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "transport(failures=1)"),
        "should report transport failure count in boundary summary, summary: {:?}",
        execution.summary_lines
    );
}

// -- Config-reload non-support --

#[test]
fn config_reload_is_not_supported() {
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
        summary_contains(&execution, "failure-config-reload: not-supported"),
        "should honestly declare config-reload as not supported, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "no reload surface exists"),
        "should explain why reload is not supported"
    );
}

// -- Failure evidence structured output --

#[test]
fn failure_evidence_is_emitted_at_runtime_finish() {
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

    let failure_lines: Vec<&String> = execution
        .summary_lines
        .iter()
        .filter(|line| line.starts_with("failure-"))
        .collect();

    assert!(
        failure_lines.len() >= 5,
        "expected at least 5 failure evidence lines, found {}: {:?}",
        failure_lines.len(),
        failure_lines
    );

    for line in &failure_lines {
        assert!(
            line.contains(':'),
            "failure evidence line should be key: value format: {line}"
        );
    }
}

#[test]
fn failure_evidence_scope_is_honest() {
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
        summary_contains(&execution, "failure-evidence-scope:"),
        "should emit failure evidence scope"
    );
    assert!(
        summary_contains(&execution, "in-process-harness-failure-proof"),
        "should report in-process harness scope"
    );
    assert!(
        summary_contains(&execution, "config-reload behavior are deferred"),
        "should honestly defer config-reload proof"
    );
}

#[test]
fn failure_evidence_under_exhausted_restarts() {
    let execution = run_with_source(
        runtime_config(),
        test_source([
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
            TestBehavior::RetryableFailure,
        ]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(&execution, "failure-restart-budget: used=2 max=2 exhausted=true"),
        "should report exhausted budget in failure evidence"
    );
    assert!(
        summary_contains(&execution, "failure-transport-failures: 3"),
        "three retryable exits should each increment transport failure counter, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "failure-events-total: 3"),
        "three retryable exits should each count as a failure event, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn failure_evidence_under_deferred_exit() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::DeferredExit]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Deferred { .. }));
    assert!(
        summary_contains(&execution, "failure-events-total: 1"),
        "deferred exit should count as a failure event, summary: {:?}",
        execution.summary_lines
    );
    assert!(
        summary_contains(&execution, "failure-restart-budget: used=0 max=2 exhausted=false"),
        "deferred exit should not consume restart budget"
    );
}
