//! Phase 4.4: Deployment proof tests.
//!
//! Proves the admitted alpha surface is believable in real operational
//! use by validating that deployment evidence is emitted, the deployment
//! contract is visible, known gaps are declared, and operational caveats
//! are explicit.
//!
//! What this validates:
//! - deployment evidence lines are emitted at runtime finish
//! - deployment contract satisfaction is visible
//! - known deployment gaps are declared honestly
//! - operational caveats are explicit and reviewable
//! - evidence scope is honestly bounded
//!
//! What this does not validate:
//! - real systemd unit integration
//! - real package manager delivery
//! - container deployment flows
//! - log rotation or journal integration

use std::time::Duration;

use crate::runtime::{HarnessBuilder, RuntimeExit, run_with_factory};

use super::fixtures::{runtime_config, summary_contains};
use super::harness::{TestBehavior, TestFactory};

// -- Deployment contract evidence --

#[test]
fn deployment_evidence_emits_contract_line() {
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
    assert!(
        summary_contains(
            &execution,
            "deploy-contract: linux-x86_64-gnu-glibc bare-metal-first systemd-expected"
        ),
        "should emit deployment contract line, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_host_validation() {
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
    assert!(
        summary_contains(&execution, "deploy-host-validation: passed"),
        "host validation should pass on Linux GNU/glibc, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_glibc_markers() {
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
    assert!(
        summary_contains(&execution, "deploy-glibc-markers: present"),
        "glibc markers should be present on test host, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_systemd_supervision() {
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
    // Systemd detection depends on environment; accept either value.
    let has_detected = summary_contains(&execution, "deploy-systemd-supervision: detected");
    let has_not_detected = summary_contains(&execution, "deploy-systemd-supervision: not-detected");
    assert!(
        has_detected || has_not_detected,
        "should report systemd supervision status, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_binary_path() {
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
    assert!(
        summary_contains(&execution, "deploy-binary-path:"),
        "should report binary path, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_config_path() {
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
    assert!(
        summary_contains(&execution, "deploy-config-path: /tmp/runtime-test.yml"),
        "should report config path from runtime config, summary: {:?}",
        execution.summary_lines
    );
}

#[test]
fn deployment_evidence_reports_filesystem_contract() {
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
    assert!(
        summary_contains(&execution, "deploy-filesystem-contract: operator-managed"),
        "should report operator-managed filesystem contract, summary: {:?}",
        execution.summary_lines
    );
}

// -- Known gaps and operational caveats --

#[test]
fn deployment_evidence_declares_known_gaps() {
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
    assert!(
        summary_contains(&execution, "deploy-known-gaps:"),
        "should declare known deployment gaps, summary: {:?}",
        execution.summary_lines
    );
    assert!(summary_contains(&execution, "no-systemd-unit"));
    assert!(summary_contains(&execution, "no-installer"));
    assert!(summary_contains(&execution, "no-container-image"));
    assert!(summary_contains(&execution, "no-updater"));
    assert!(summary_contains(&execution, "no-log-rotation"));
}

#[test]
fn deployment_evidence_declares_operational_caveats() {
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
    assert!(
        summary_contains(&execution, "deploy-operational-caveats:"),
        "should declare operational caveats, summary: {:?}",
        execution.summary_lines
    );
    assert!(summary_contains(&execution, "alpha-only"));
    assert!(summary_contains(
        &execution,
        "limited-origin-dispatch(http_status+hello_world+http-wired-no-proxy)"
    ));
    assert!(summary_contains(&execution, "no-capnp-registration-rpc"));
    assert!(summary_contains(
        &execution,
        "no-origin-cert-registration-content"
    ));
    assert!(summary_contains(&execution, "no-stream-roundtrip"));
    assert!(summary_contains(&execution, "no-config-reload"));
}

#[test]
fn deployment_evidence_declares_evidence_scope() {
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
    assert!(
        summary_contains(
            &execution,
            "deploy-evidence-scope: in-process-contract-validation"
        ),
        "should report evidence scope honestly, summary: {:?}",
        execution.summary_lines
    );
}

// -- Deployment evidence on failure path --

#[test]
fn deployment_evidence_emitted_on_fatal_exit() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::FatalFailure]),
        HarnessBuilder::for_tests().build(),
        None,
        None,
    );

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(
        summary_contains(&execution, "deploy-contract:"),
        "deployment evidence should be emitted even on fatal exit"
    );
    assert!(
        summary_contains(&execution, "deploy-known-gaps:"),
        "known gaps should be declared even on fatal exit"
    );
    assert!(
        summary_contains(&execution, "deploy-evidence-scope:"),
        "evidence scope should be declared even on fatal exit"
    );
}

// -- Structured format validation --

#[test]
fn deployment_evidence_lines_have_structured_format() {
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

    let deploy_lines: Vec<&String> = execution
        .summary_lines
        .iter()
        .filter(|line| line.starts_with("deploy-"))
        .collect();

    assert!(
        deploy_lines.len() >= 9,
        "expected at least 9 deploy evidence lines, found {}: {:?}",
        deploy_lines.len(),
        deploy_lines
    );

    for line in &deploy_lines {
        assert!(
            line.contains(':'),
            "deploy evidence line should be key: value format: {line}"
        );
    }
}
