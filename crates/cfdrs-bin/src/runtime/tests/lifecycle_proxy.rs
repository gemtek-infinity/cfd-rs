use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_factory};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, TestFactory};

#[test]
fn runtime_admits_proxy_seam_with_origin_path() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "proxy-seam: origin-proxy admitted"));
    assert!(summary_contains(
        &execution,
        "service-status[pingora-proxy-seam]: origin-proxy-admitted"
    ));
    assert!(summary_contains(&execution, "proxy-state: admitted"));
    assert!(summary_contains(&execution, "child-task-stopped: proxy-seam"));
}

#[test]
fn proxy_seam_survives_primary_service_restart() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(50))
            .build(),
        None,
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "proxy-seam: origin-proxy admitted"));
    assert!(summary_contains(&execution, "supervision-restart-attempt: 1"));
    assert!(summary_contains(&execution, "child-task-stopped: proxy-seam"));
}

#[test]
fn runtime_reports_operability_snapshot_when_transport_inputs_are_missing() {
    let execution = super::super::run(runtime_config());

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(&execution, "protocol-state: bridge-created"));
    assert!(summary_contains(&execution, "proxy-state: admitted"));
    assert!(summary_contains(
        &execution,
        "operability-metrics: restart-attempts=0 proxy-admissions=1"
    ));
}
