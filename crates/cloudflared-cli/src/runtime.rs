use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::proxy::PingoraProxySeam;
use crate::startup::config_source_label;
use crate::transport::QuicTunnelServiceFactory;

use cloudflared_config::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;

#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};

const PRIMARY_SERVICE_NAME: &str = "quic-tunnel-core";
#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    discovery: DiscoveryOutcome,
    normalized: NormalizedConfig,
}

impl RuntimeConfig {
    pub(crate) fn new(discovery: DiscoveryOutcome, normalized: NormalizedConfig) -> Self {
        Self {
            discovery,
            normalized,
        }
    }

    pub(crate) fn config_path(&self) -> &PathBuf {
        &self.discovery.path
    }

    pub(crate) fn config_source(&self) -> &ConfigSource {
        &self.discovery.source
    }

    pub(crate) fn normalized(&self) -> &NormalizedConfig {
        &self.normalized
    }
}

#[derive(Debug, Clone)]
struct RuntimePolicy {
    max_restart_attempts: u32,
    restart_backoff: Duration,
    shutdown_grace_period: Duration,
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self {
            max_restart_attempts: 2,
            restart_backoff: Duration::from_millis(25),
            shutdown_grace_period: Duration::from_millis(100),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeExecution {
    pub(crate) summary_lines: Vec<String>,
    pub(crate) exit: RuntimeExit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeExit {
    Clean,
    Deferred { phase: &'static str, detail: String },
    Failed { detail: String },
}

impl RuntimeExit {
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::Deferred { .. } | Self::Failed { .. } => 1,
        }
    }

    pub(crate) fn stderr_message(&self) -> Option<String> {
        match self {
            Self::Clean => None,
            Self::Deferred { phase, detail } => Some(format!(
                "error: runtime ownership is active, but later slice work is still deferred in {phase}: \
                 {detail}\n"
            )),
            Self::Failed { detail } => Some(format!("error: runtime lifecycle failed: {detail}\n")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

impl LifecycleState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub(crate) enum RuntimeCommand {
    ServiceReady { service: &'static str },
    ServiceStatus { service: &'static str, detail: String },
    ServiceExited(ServiceExit),
    ShutdownRequested(ShutdownReason),
    ControlPlaneFailure { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShutdownReason {
    Signal(&'static str),
    Harness,
    ServiceFailure(&'static str),
}

impl ShutdownReason {
    fn as_str(&self) -> String {
        match self {
            Self::Signal(name) => format!("signal:{name}"),
            Self::Harness => "harness".to_owned(),
            Self::ServiceFailure(name) => format!("service-failure:{name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum ServiceExit {
    Completed {
        service: &'static str,
    },
    RetryableFailure {
        service: &'static str,
        detail: String,
    },
    Deferred {
        service: &'static str,
        phase: &'static str,
        detail: String,
    },
    Fatal {
        service: &'static str,
        detail: String,
    },
}

#[derive(Debug)]
pub(crate) enum ChildTask {
    Service(&'static str),
    ProxySeam,
    SignalBridge,
    HarnessBridge,
}

pub(crate) trait RuntimeServiceFactory: Send + Sync + 'static {
    fn create_primary(&self, config: Arc<RuntimeConfig>, attempt: u32) -> Box<dyn RuntimeService>;
}

pub(crate) trait RuntimeService: Send + 'static {
    fn name(&self) -> &'static str;

    fn spawn(
        self: Box<Self>,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    );
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeHarness {
    enable_signals: bool,
    injected_shutdown_after: Option<Duration>,
}

impl RuntimeHarness {
    fn production() -> Self {
        Self {
            enable_signals: true,
            injected_shutdown_after: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self {
            enable_signals: false,
            injected_shutdown_after: None,
        }
    }

    #[cfg(test)]
    fn with_shutdown_after(mut self, duration: Duration) -> Self {
        self.injected_shutdown_after = Some(duration);
        self
    }
}

struct ApplicationRuntime<F> {
    config: Arc<RuntimeConfig>,
    factory: F,
    policy: RuntimePolicy,
    harness: RuntimeHarness,
    command_tx: mpsc::Sender<RuntimeCommand>,
    command_rx: mpsc::Receiver<RuntimeCommand>,
    child_tasks: JoinSet<ChildTask>,
    shutdown: CancellationToken,
    summary_lines: Vec<String>,
    lifecycle_state: LifecycleState,
    restart_attempts: u32,
}

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    fn new(config: RuntimeConfig, factory: F, harness: RuntimeHarness) -> Self {
        let (command_tx, command_rx) = mpsc::channel(16);

        Self {
            config: Arc::new(config),
            factory,
            policy: RuntimePolicy::default(),
            harness,
            command_tx,
            command_rx,
            child_tasks: JoinSet::new(),
            shutdown: CancellationToken::new(),
            summary_lines: Vec::new(),
            lifecycle_state: LifecycleState::Starting,
            restart_attempts: 0,
        }
    }

    async fn run(mut self) -> RuntimeExecution {
        self.summary_lines.push("runtime-owner: initialized".to_owned());
        self.summary_lines
            .push("config-ownership: runtime-owned".to_owned());
        self.summary_lines.push(format!(
            "runtime-config-source: {}",
            config_source_label(self.config.config_source())
        ));
        self.summary_lines.push(format!(
            "runtime-config-path: {}",
            self.config.config_path().display()
        ));
        self.summary_lines.push(format!(
            "runtime-ingress-rules: {}",
            self.config.normalized().ingress.len()
        ));
        self.summary_lines.push(format!(
            "supervision-policy: primary-service={PRIMARY_SERVICE_NAME} max-restarts={} \
             restart-backoff-ms={} shutdown-grace-ms={}",
            self.policy.max_restart_attempts,
            self.policy.restart_backoff.as_millis(),
            self.policy.shutdown_grace_period.as_millis()
        ));
        self.record_state(LifecycleState::Starting, "startup sequencing entered");

        self.spawn_signal_bridge();
        self.spawn_harness_shutdown();
        // 3.4b: proxy seam enters lifecycle before the primary transport service,
        // so it is ready to receive the transport→proxy handoff in later slices.
        self.spawn_proxy_seam();
        self.spawn_primary_service(0);

        loop {
            let Some(command) = self.command_rx.recv().await else {
                return self
                    .finish(RuntimeExit::Failed {
                        detail: "runtime command channel closed unexpectedly".to_owned(),
                    })
                    .await;
            };

            if let Some(exit) = self.handle_command(command).await {
                return self.finish(exit).await;
            }
        }
    }

    async fn handle_command(&mut self, command: RuntimeCommand) -> Option<RuntimeExit> {
        match command {
            RuntimeCommand::ServiceReady { service } => {
                if self.lifecycle_state == LifecycleState::Starting {
                    self.record_state(LifecycleState::Running, format!("service ready: {service}"));
                }
                None
            }
            RuntimeCommand::ServiceStatus { service, detail } => {
                self.summary_lines
                    .push(format!("service-status[{service}]: {detail}"));
                None
            }
            RuntimeCommand::ServiceExited(ServiceExit::Completed { service }) => Some(RuntimeExit::Failed {
                detail: format!("{service} exited without a runtime shutdown request"),
            }),
            RuntimeCommand::ServiceExited(ServiceExit::RetryableFailure { service, detail }) => {
                if self.restart_attempts < self.policy.max_restart_attempts {
                    self.restart_attempts += 1;
                    self.summary_lines.push(format!(
                        "supervision-restart-attempt: {} service={} detail={detail}",
                        self.restart_attempts, service
                    ));
                    self.record_state(
                        LifecycleState::Starting,
                        format!("restarting {service} after retryable failure"),
                    );
                    time::sleep(self.policy.restart_backoff).await;
                    self.spawn_primary_service(self.restart_attempts);
                    None
                } else {
                    Some(RuntimeExit::Failed {
                        detail: format!(
                            "{service} exhausted restart policy after {} attempts: {detail}",
                            self.restart_attempts
                        ),
                    })
                }
            }
            RuntimeCommand::ServiceExited(ServiceExit::Deferred {
                service,
                phase,
                detail,
            }) => Some(RuntimeExit::Deferred {
                phase,
                detail: format!("{service}: {detail}"),
            }),
            RuntimeCommand::ServiceExited(ServiceExit::Fatal { service, detail }) => {
                Some(RuntimeExit::Failed {
                    detail: format!("{service}: {detail}"),
                })
            }
            RuntimeCommand::ShutdownRequested(reason) => {
                self.summary_lines
                    .push(format!("shutdown-reason: {}", reason.as_str()));
                Some(RuntimeExit::Clean)
            }
            RuntimeCommand::ControlPlaneFailure { detail } => Some(RuntimeExit::Failed { detail }),
        }
    }

    async fn finish(mut self, exit: RuntimeExit) -> RuntimeExecution {
        let stopping_reason = match &exit {
            RuntimeExit::Clean => "graceful shutdown requested".to_owned(),
            RuntimeExit::Deferred { detail, .. } => format!("deferred service boundary reached: {detail}"),
            RuntimeExit::Failed { detail } => format!("runtime failure: {detail}"),
        };
        self.record_state(LifecycleState::Stopping, stopping_reason);

        if matches!(exit, RuntimeExit::Deferred { .. }) {
            self.summary_lines.push(format!(
                "shutdown-reason: {}",
                ShutdownReason::ServiceFailure(PRIMARY_SERVICE_NAME).as_str()
            ));
        }

        self.shutdown.cancel();
        self.drain_child_tasks().await;

        match exit {
            RuntimeExit::Clean => self.record_state(LifecycleState::Stopped, "runtime stopped cleanly"),
            RuntimeExit::Deferred { .. } | RuntimeExit::Failed { .. } => self.record_state(
                LifecycleState::Failed,
                "runtime stopped with a deferred or failed service boundary",
            ),
        }

        RuntimeExecution {
            summary_lines: self.summary_lines,
            exit,
        }
    }

    fn spawn_proxy_seam(&mut self) {
        let seam = PingoraProxySeam::new();
        self.summary_lines
            .push("proxy-seam: lifecycle participant admitted".to_owned());
        seam.spawn(
            self.command_tx.clone(),
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    fn spawn_primary_service(&mut self, attempt: u32) {
        let service = self.factory.create_primary(self.config.clone(), attempt);
        self.summary_lines.push(format!(
            "primary-service-attempt: {} service={}",
            attempt + 1,
            service.name()
        ));
        service.spawn(
            self.command_tx.clone(),
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    fn spawn_signal_bridge(&mut self) {
        if !self.harness.enable_signals {
            return;
        }

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            #[cfg(target_family = "unix")]
            {
                let mut sigint = match signal(SignalKind::interrupt()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ControlPlaneFailure {
                                detail: format!("failed to register SIGINT handler: {error}"),
                            })
                            .await;
                        return ChildTask::SignalBridge;
                    }
                };
                let mut sigterm = match signal(SignalKind::terminate()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ControlPlaneFailure {
                                detail: format!("failed to register SIGTERM handler: {error}"),
                            })
                            .await;
                        return ChildTask::SignalBridge;
                    }
                };

                tokio::select! {
                    _ = shutdown.cancelled() => {}
                    _ = sigint.recv() => {
                        let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Signal("SIGINT"))).await;
                    }
                    _ = sigterm.recv() => {
                        let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Signal("SIGTERM"))).await;
                    }
                }
            }

            ChildTask::SignalBridge
        });
    }

    fn spawn_harness_shutdown(&mut self) {
        let Some(duration) = self.harness.injected_shutdown_after else {
            return;
        };

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            tokio::select! {
                _ = shutdown.cancelled() => {}
                _ = time::sleep(duration) => {
                    let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Harness)).await;
                }
            }

            ChildTask::HarnessBridge
        });
    }

    async fn drain_child_tasks(&mut self) {
        loop {
            let joined = time::timeout(self.policy.shutdown_grace_period, self.child_tasks.join_next()).await;

            match joined {
                Ok(Some(Ok(child_task))) => match child_task {
                    ChildTask::Service(name) => {
                        self.summary_lines
                            .push(format!("child-task-stopped: service={name}"));
                    }
                    ChildTask::ProxySeam => {
                        self.summary_lines
                            .push("child-task-stopped: proxy-seam".to_owned());
                    }
                    ChildTask::SignalBridge => {
                        self.summary_lines
                            .push("child-task-stopped: signal-bridge".to_owned());
                    }
                    ChildTask::HarnessBridge => {
                        self.summary_lines
                            .push("child-task-stopped: harness-bridge".to_owned());
                    }
                },
                Ok(Some(Err(error))) => {
                    self.summary_lines.push(format!("child-task-error: {error}"));
                }
                Ok(None) => break,
                Err(_) => {
                    self.summary_lines.push(
                        "shutdown-action: aborting remaining child tasks after grace timeout".to_owned(),
                    );
                    self.child_tasks.abort_all();
                    while let Some(result) = self.child_tasks.join_next().await {
                        if let Err(error) = result {
                            self.summary_lines.push(format!("child-task-error: {error}"));
                        }
                    }
                    break;
                }
            }
        }
    }

    fn record_state(&mut self, state: LifecycleState, reason: impl Into<String>) {
        self.lifecycle_state = state;
        self.summary_lines
            .push(format!("lifecycle-state: {}", state.as_str()));
        self.summary_lines
            .push(format!("lifecycle-reason: {}", reason.into()));
    }
}

pub(crate) fn run(config: RuntimeConfig) -> RuntimeExecution {
    run_with_factory(
        config,
        QuicTunnelServiceFactory::production(),
        RuntimeHarness::production(),
    )
}

pub(crate) fn run_with_factory<F>(
    config: RuntimeConfig,
    factory: F,
    harness: RuntimeHarness,
) -> RuntimeExecution
where
    F: RuntimeServiceFactory,
{
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for the admitted Phase 3.3 shell");

    runtime.block_on(ApplicationRuntime::new(config, factory, harness).run())
}

#[cfg(test)]
mod tests {
    use super::{
        ChildTask, RuntimeCommand, RuntimeConfig, RuntimeExecution, RuntimeExit, RuntimeHarness,
        RuntimeService, RuntimeServiceFactory, ServiceExit, run_with_factory,
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
        );

        assert_eq!(execution.exit, RuntimeExit::Clean);
        assert!(summary_contains(&execution, "config-ownership: runtime-owned"));
        assert!(summary_contains(&execution, "runtime-config-source: explicit"));
        assert!(summary_contains(
            &execution,
            "runtime-config-path: /tmp/runtime-test.yml"
        ));
    }

    #[test]
    fn runtime_orders_shutdown_of_ready_service() {
        let execution = run_with_factory(
            runtime_config(),
            TestFactory::new([TestBehavior::WaitForShutdown]),
            RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
        );

        assert_eq!(execution.exit, RuntimeExit::Clean);
        assert!(summary_contains(&execution, "lifecycle-state: starting"));
        assert!(summary_contains(&execution, "lifecycle-state: running"));
        assert!(summary_contains(&execution, "shutdown-reason: harness"));
        assert!(summary_contains(&execution, "lifecycle-state: stopping"));
        assert!(summary_contains(&execution, "lifecycle-state: stopped"));
    }

    #[test]
    fn runtime_restarts_retryable_service_before_shutdown() {
        let execution = run_with_factory(
            runtime_config(),
            TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
            RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
        );

        assert_eq!(execution.exit, RuntimeExit::Clean);
        assert!(summary_contains(&execution, "supervision-restart-attempt: 1"));
        assert!(summary_contains(
            &execution,
            "restarting test-service after retryable failure"
        ));
    }

    #[test]
    fn runtime_requires_transport_identity_for_real_quic_core() {
        let execution = super::run(runtime_config());

        assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
        assert!(summary_contains(&execution, "primary-service=quic-tunnel-core"));
        assert!(summary_contains(&execution, "lifecycle-state: failed"));
    }

    #[test]
    fn runtime_fails_fatal_service_without_restart() {
        let execution = run_with_factory(
            runtime_config(),
            TestFactory::new([TestBehavior::FatalFailure]),
            RuntimeHarness::for_tests(),
        );

        assert!(matches!(execution.exit, RuntimeExit::Failed { .. }));
        assert!(summary_contains(
            &execution,
            "runtime failure: test-service: fatal lifecycle boundary triggered"
        ));
        assert!(!summary_contains(&execution, "supervision-restart-attempt:"));
    }

    #[test]
    fn runtime_admits_proxy_seam_lifecycle_participant() {
        let execution = run_with_factory(
            runtime_config(),
            TestFactory::new([TestBehavior::WaitForShutdown]),
            RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(25)),
        );

        assert_eq!(execution.exit, RuntimeExit::Clean);
        assert!(summary_contains(
            &execution,
            "proxy-seam: lifecycle participant admitted"
        ));
        assert!(summary_contains(
            &execution,
            "service-status[pingora-proxy-seam]:"
        ));
        assert!(summary_contains(&execution, "child-task-stopped: proxy-seam"));
    }

    #[test]
    fn proxy_seam_survives_primary_service_restart() {
        let execution = run_with_factory(
            runtime_config(),
            TestFactory::new([TestBehavior::RetryableFailure, TestBehavior::WaitForShutdown]),
            RuntimeHarness::for_tests().with_shutdown_after(Duration::from_millis(50)),
        );

        assert_eq!(execution.exit, RuntimeExit::Clean);
        // Proxy seam admitted once, persists across primary service restarts.
        assert!(summary_contains(
            &execution,
            "proxy-seam: lifecycle participant admitted"
        ));
        assert!(summary_contains(&execution, "supervision-restart-attempt: 1"));
        assert!(summary_contains(&execution, "child-task-stopped: proxy-seam"));
    }
}
