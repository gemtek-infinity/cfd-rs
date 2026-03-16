use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_source};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, test_source};

#[test]
fn runtime_owns_config_after_startup_handoff() {
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
    assert!(summary_contains(&execution, "config-ownership: runtime-owned"));
    assert!(summary_contains(&execution, "runtime-config-source: explicit"));
    assert!(summary_contains(
        &execution,
        "runtime-config-path: /tmp/runtime-test.yml"
    ));
    assert!(summary_contains(&execution, "protocol-state: bridge-unavailable"));
}

#[test]
fn runtime_orders_shutdown_of_ready_service() {
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
    assert!(summary_contains(&execution, "lifecycle-state: starting"));
    assert!(summary_contains(&execution, "lifecycle-state: running"));
    assert!(summary_contains(&execution, "shutdown-reason: harness"));
    assert!(summary_contains(&execution, "lifecycle-state: stopping"));
    assert!(summary_contains(&execution, "lifecycle-state: stopped"));
    assert!(summary_contains(&execution, "readiness-state: stopping"));
}

#[test]
fn runtime_restarts_retryable_service_before_shutdown() {
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
    assert!(summary_contains(&execution, "supervision-restart-attempt: 1"));
    assert!(summary_contains(
        &execution,
        "restarting test-service after retryable failure"
    ));
    assert!(summary_contains(
        &execution,
        "operability-metrics: restart-attempts=1"
    ));
}

#[test]
fn runtime_fails_fatal_service_without_restart() {
    let execution = run_with_source(
        runtime_config(),
        test_source([TestBehavior::FatalFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(
        &execution,
        "runtime failure: test-service: fatal lifecycle boundary triggered"
    ));
    assert!(summary_contains(
        &execution,
        "failure-visibility: owner=test-service class=fatal"
    ));
    assert!(!summary_contains(&execution, "supervision-restart-attempt:"));
}
