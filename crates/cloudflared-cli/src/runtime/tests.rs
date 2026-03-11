use super::{
    ChildTask, RuntimeCommand, RuntimeConfig, RuntimeExecution, RuntimeExit, RuntimeHarness, RuntimeService,
    RuntimeServiceFactory, ServiceExit, run_with_factory,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use cloudflared_config::{ConfigSource, DiscoveryAction, DiscoveryOutcome, NormalizedConfig, RawConfig};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
enum TestBehavior {
    WaitForShutdown,
    RetryableFailure,
    FatalFailure,
}

#[derive(Clone)]
struct TestFactory {
    behaviors: std::sync::Arc<Mutex<VecDeque<TestBehavior>>>,
}

impl TestFactory {
    fn new(behaviors: impl IntoIterator<Item = TestBehavior>) -> Self {
        Self {
            behaviors: std::sync::Arc::new(Mutex::new(behaviors.into_iter().collect())),
        }
    }
}

impl RuntimeServiceFactory for TestFactory {
    fn create_primary(
        &self,
        _config: std::sync::Arc<RuntimeConfig>,
        _attempt: u32,
    ) -> Box<dyn RuntimeService> {
        let behavior = self
            .behaviors
            .lock()
            .expect("test factory lock should not be poisoned")
            .pop_front()
            .unwrap_or(TestBehavior::WaitForShutdown);

        Box::new(TestService { behavior })
    }
}

struct TestService {
    behavior: TestBehavior,
}

impl RuntimeService for TestService {
    fn name(&self) -> &'static str {
        "test-service"
    }

    fn spawn(
        self: Box<Self>,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        let behavior = self.behavior.clone();
        child_tasks.spawn(async move {
            let _ = command_tx
                .send(RuntimeCommand::ServiceReady {
                    service: "test-service",
                })
                .await;

            match behavior {
                TestBehavior::WaitForShutdown => {
                    shutdown.cancelled().await;
                    let _ = command_tx
                        .send(RuntimeCommand::ServiceExited(ServiceExit::Completed {
                            service: "test-service",
                        }))
                        .await;
                }
                TestBehavior::RetryableFailure => {
                    let _ = command_tx
                        .send(RuntimeCommand::ServiceExited(ServiceExit::RetryableFailure {
                            service: "test-service",
                            detail: "retry requested by lifecycle policy".to_owned(),
                        }))
                        .await;
                }
                TestBehavior::FatalFailure => {
                    let _ = command_tx
                        .send(RuntimeCommand::ServiceExited(ServiceExit::Fatal {
                            service: "test-service",
                            detail: "fatal lifecycle boundary triggered".to_owned(),
                        }))
                        .await;
                }
            }

            ChildTask::Service("test-service")
        });
    }
}

fn runtime_config() -> RuntimeConfig {
    let raw = RawConfig::from_yaml_str(
        "runtime-test.yaml",
        "tunnel: runtime-test\ningress:\n  - service: http_status:503\n",
    )
    .expect("runtime config fixture should parse");
    let normalized =
        NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/runtime-test.yml".into()), raw)
            .expect("runtime config fixture should normalize");
    let discovery = DiscoveryOutcome {
        action: DiscoveryAction::UseExisting,
        source: ConfigSource::ExplicitPath(PathBuf::from("/tmp/runtime-test.yml")),
        path: PathBuf::from("/tmp/runtime-test.yml"),
        created_paths: Vec::new(),
        written_config: None,
    };

    RuntimeConfig::new(discovery, normalized)
}

fn summary_contains(execution: &RuntimeExecution, needle: &str) -> bool {
    execution.summary_lines.iter().any(|line| line.contains(needle))
}

#[test]
fn runtime_owns_config_after_startup_handoff() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
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
fn runtime_requires_transport_identity_for_real_quic_core() {
    let execution = super::run(runtime_config());

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(&execution, "primary-service=quic-tunnel-core"));
    assert!(summary_contains(&execution, "lifecycle-state: failed"));
    assert!(summary_contains(
        &execution,
        "operability-status: lifecycle=failed readiness=failed"
    ));
}

#[test]
fn runtime_reports_security_compliance_boundary_as_bounded() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
    assert!(!super::deployment::glibc_runtime_marker_present(&[
        "/this/path/does/not/exist/libc.so.6",
        "/this/path/also/does/not/exist/ld-linux.so",
    ]));
}

#[test]
fn runtime_fails_fatal_service_without_restart() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::FatalFailure]),
        RuntimeHarness::for_tests(),
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

#[test]
fn runtime_admits_proxy_seam_with_origin_path() {
    let execution = run_with_factory(
        runtime_config(),
        TestFactory::new([TestBehavior::WaitForShutdown]),
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
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
        RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
        None,
    );

    assert_eq!(execution.exit, RuntimeExit::Clean);
    assert!(summary_contains(&execution, "proxy-seam: origin-proxy admitted"));
    assert!(summary_contains(&execution, "supervision-restart-attempt: 1"));
    assert!(summary_contains(&execution, "child-task-stopped: proxy-seam"));
}

#[test]
fn runtime_reports_operability_snapshot_when_transport_inputs_are_missing() {
    let execution = super::run(runtime_config());

    assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
    assert!(summary_contains(&execution, "protocol-state: bridge-created"));
    assert!(summary_contains(&execution, "proxy-state: admitted"));
    assert!(summary_contains(
        &execution,
        "operability-metrics: restart-attempts=0 proxy-admissions=1"
    ));
}
