use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_factory};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, TestFactory};

#[test]
fn runtime_reports_security_compliance_boundary_as_bounded() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(25))
            .build(),
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(
        &execution,
        "security-boundary: runtime-crypto-surface=transport-tls-only"
    ));
    assert!(summary_contains(
        &execution,
        "security-boundary-claims: bounded-surface-only, not-whole-program, not-certification"
    ));
    assert!(summary_contains(
        &execution,
        "security-host-contract: linux-x86_64-gnu-glibc markers present"
    ));
    assert!(summary_contains(
        &execution,
        "readiness-scope: narrow-alpha-control-plane-only"
    ));
}

#[test]
fn glibc_marker_probe_is_false_for_missing_markers() {
    assert!(!super::super::deployment::glibc_runtime_marker_present(&[
        "/this/path/does/not/exist/libc.so.6",
        "/this/path/also/does/not/exist/ld-linux.so",
    ]));
}

#[test]
fn runtime_requires_transport_identity_for_real_quic_core() {
    let execution = super::super::run(runtime_config());

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(&execution, "primary-service=quic-tunnel-core"));
    assert!(summary_contains(&execution, "lifecycle-state: failed"));
    assert!(summary_contains(
        &execution,
        "operability-status: lifecycle=failed readiness=failed"
    ));
}
