use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use crate::protocol::{self, ProtocolBridgeState, ProtocolReceiver};
use crate::proxy::{PingoraProxySeam, ProxySeamState};
use crate::startup::config_source_label;
use crate::transport::{QuicTunnelServiceFactory, TransportLifecycleStage};

use cloudflared_config::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::fmt;

#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};

const PRIMARY_SERVICE_NAME: &str = "quic-tunnel-core";
const FROZEN_TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
const TRANSPORT_CRYPTO_LANE: &str = "quiche+boringssl";
const READINESS_SCOPE: &str = "narrow-alpha-control-plane-only";
const GLIBC_RUNTIME_MARKERS: &[&str] = &[
    "/lib64/ld-linux-x86-64.so.2",
    "/lib/x86_64-linux-gnu/libc.so.6",
    "/usr/lib64/libc.so.6",
];

static RUNTIME_LOGGING: std::sync::OnceLock<()> = std::sync::OnceLock::new();

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

pub(crate) fn install_runtime_logging() {
    RUNTIME_LOGGING.get_or_init(|| {
        let subscriber = fmt()
            .with_writer(std::io::stderr)
            .without_time()
            .with_target(false)
            .compact()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadinessState {
    Starting,
    WaitingForProxyAdmission,
    WaitingForTransport,
    WaitingForProtocolBridge,
    Ready,
    Stopping,
    Failed,
}

impl ReadinessState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::WaitingForProxyAdmission => "waiting-for-proxy-admission",
            Self::WaitingForTransport => "waiting-for-transport",
            Self::WaitingForProtocolBridge => "waiting-for-protocol-bridge",
            Self::Ready => "ready",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub(crate) enum RuntimeCommand {
    ServiceReady {
        service: &'static str,
    },
    ServiceStatus {
        service: &'static str,
        detail: String,
    },
    TransportStage {
        service: &'static str,
        stage: TransportLifecycleStage,
        detail: String,
    },
    ProtocolState {
        state: ProtocolBridgeState,
        detail: String,
    },
    ProxyState {
        state: ProxySeamState,
        detail: String,
    },
    ServiceExited(ServiceExit),
    ShutdownRequested(ShutdownReason),
    ControlPlaneFailure {
        detail: String,
    },
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
    readiness_state: ReadinessState,
    restart_attempts: u32,
    protocol_receiver: Option<ProtocolReceiver>,
    transport_stage: Option<TransportLifecycleStage>,
    protocol_state: ProtocolBridgeState,
    proxy_state: Option<ProxySeamState>,
    proxy_admissions: u32,
    protocol_registrations: u32,
    transport_failures: u32,
    failure_events: u32,
}

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    fn new(
        config: RuntimeConfig,
        factory: F,
        harness: RuntimeHarness,
        protocol_receiver: Option<ProtocolReceiver>,
    ) -> Self {
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
            readiness_state: ReadinessState::Starting,
            restart_attempts: 0,
            transport_stage: None,
            protocol_state: if protocol_receiver.is_some() {
                ProtocolBridgeState::BridgeCreated
            } else {
                ProtocolBridgeState::BridgeUnavailable
            },
            protocol_receiver,
            proxy_state: None,
            proxy_admissions: 0,
            protocol_registrations: 0,
            transport_failures: 0,
            failure_events: 0,
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
        self.summary_lines
            .push(format!("readiness-scope: {READINESS_SCOPE}"));

        if let Err(detail) = self.record_security_compliance_boundary() {
            return self.finish(RuntimeExit::Failed { detail }).await;
        }

        self.record_state(LifecycleState::Starting, "startup sequencing entered");
        self.record_readiness(ReadinessState::Starting, "runtime startup sequencing entered");
        self.record_protocol_state(
            self.protocol_state,
            if matches!(self.protocol_state, ProtocolBridgeState::BridgeCreated) {
                "runtime created protocol bridge endpoints"
            } else {
                "protocol bridge omitted by runtime harness"
            },
        );

        self.spawn_signal_bridge();
        self.spawn_harness_shutdown();
        // 3.4b+c + 4.1: proxy seam enters lifecycle before the primary
        // transport service, receives ingress rules from the runtime,
        // provides the first admitted origin/proxy path, and reports
        // owner-scoped proxy/protocol operability state back to runtime.
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

    fn record_security_compliance_boundary(&mut self) -> Result<(), String> {
        self.summary_lines.push(format!(
            "security-boundary: runtime-crypto-surface=transport-tls-only lane={TRANSPORT_CRYPTO_LANE}"
        ));
        self.summary_lines.push(
            "security-boundary-claims: bounded-surface-only, not-whole-program, not-certification".to_owned(),
        );
        self.summary_lines.push(format!(
            "security-build-contract: target={FROZEN_TARGET_TRIPLE} \
             pingora-role=application-layer-above-transport"
        ));
        self.summary_lines.push(
            "security-deployment-contract: linux-gnu-glibc supervised-host-service systemd-expected \
             bare-metal-first"
                .to_owned(),
        );

        self.validate_deployment_contract()?;
        let systemd = if is_systemd_supervision_detected() {
            "detected"
        } else {
            "not-detected"
        };
        self.summary_lines.push(format!(
            "security-supervision-signal: {systemd} (systemd expected by deployment contract)"
        ));

        Ok(())
    }

    fn validate_deployment_contract(&mut self) -> Result<(), String> {
        if !cfg!(target_os = "linux") {
            return Err(format!(
                "security/compliance operational boundary requires Linux host runtime, current target_os={} ",
                env::consts::OS
            ));
        }

        if !cfg!(target_arch = "x86_64") {
            return Err(format!(
                "security/compliance operational boundary requires x86_64 host runtime, current \
                 target_arch={} ",
                env::consts::ARCH
            ));
        }

        if !cfg!(target_env = "gnu") {
            return Err(
                "security/compliance operational boundary requires GNU/glibc build contract for the \
                 admitted lane"
                    .to_owned(),
            );
        }

        if !glibc_runtime_marker_present(GLIBC_RUNTIME_MARKERS) {
            return Err(format!(
                "security/compliance operational boundary requires GNU/glibc host runtime markers; none \
                 found in {}",
                GLIBC_RUNTIME_MARKERS.join(", ")
            ));
        }

        self.summary_lines
            .push("security-host-contract: linux-x86_64-gnu-glibc markers present".to_owned());

        Ok(())
    }

    async fn handle_command(&mut self, command: RuntimeCommand) -> Option<RuntimeExit> {
        match command {
            RuntimeCommand::ServiceReady { service } => {
                if self.lifecycle_state == LifecycleState::Starting {
                    self.record_state(LifecycleState::Running, format!("service ready: {service}"));
                }
                self.refresh_readiness(format!("{service} reported ready"));
                None
            }
            RuntimeCommand::ServiceStatus { service, detail } => {
                self.summary_lines
                    .push(format!("service-status[{service}]: {detail}"));
                info!("service-status service={service} detail={detail}");
                None
            }
            RuntimeCommand::TransportStage {
                service,
                stage,
                detail,
            } => {
                self.transport_stage = Some(stage);
                self.summary_lines
                    .push(format!("transport-stage[{service}]: {}", stage.as_str()));
                self.summary_lines
                    .push(format!("transport-detail[{service}]: {detail}"));
                info!(
                    "transport-stage service={service} stage={} detail={detail}",
                    stage.as_str()
                );
                self.refresh_readiness(format!("transport reached {}", stage.as_str()));
                None
            }
            RuntimeCommand::ProtocolState { state, detail } => {
                self.record_protocol_state(state, detail);
                self.refresh_readiness(format!("protocol bridge reached {}", state.as_str()));
                None
            }
            RuntimeCommand::ProxyState { state, detail } => {
                self.record_proxy_state(state, detail);
                self.refresh_readiness(format!("proxy seam reached {}", state.as_str()));
                None
            }
            RuntimeCommand::ServiceExited(ServiceExit::Completed { service }) => Some(RuntimeExit::Failed {
                detail: format!("{service} exited without a runtime shutdown request"),
            }),
            RuntimeCommand::ServiceExited(ServiceExit::RetryableFailure { service, detail }) => {
                self.record_failure_boundary(service, "retryable", &detail);
                self.transport_failures += 1;
                if self.restart_attempts < self.policy.max_restart_attempts {
                    self.restart_attempts += 1;
                    self.summary_lines.push(format!(
                        "supervision-restart-attempt: {} service={} detail={detail}",
                        self.restart_attempts, service
                    ));
                    warn!(
                        "runtime-restart service={service} attempt={} detail={detail}",
                        self.restart_attempts
                    );
                    self.record_state(
                        LifecycleState::Starting,
                        format!("restarting {service} after retryable failure"),
                    );
                    self.refresh_readiness(format!("runtime restarting {service} after retryable failure"));
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
                detail: {
                    self.record_failure_boundary(service, "deferred", &detail);
                    format!("{service}: {detail}")
                },
            }),
            RuntimeCommand::ServiceExited(ServiceExit::Fatal { service, detail }) => {
                self.record_failure_boundary(service, "fatal", &detail);
                Some(RuntimeExit::Failed {
                    detail: format!("{service}: {detail}"),
                })
            }
            RuntimeCommand::ShutdownRequested(reason) => {
                self.summary_lines
                    .push(format!("shutdown-reason: {}", reason.as_str()));
                info!("runtime-shutdown-request reason={}", reason.as_str());
                Some(RuntimeExit::Clean)
            }
            RuntimeCommand::ControlPlaneFailure { detail } => {
                self.record_failure_boundary("runtime-control-plane", "fatal", &detail);
                Some(RuntimeExit::Failed { detail })
            }
        }
    }

    async fn finish(mut self, exit: RuntimeExit) -> RuntimeExecution {
        let stopping_reason = match &exit {
            RuntimeExit::Clean => "graceful shutdown requested".to_owned(),
            RuntimeExit::Deferred { detail, .. } => format!("deferred service boundary reached: {detail}"),
            RuntimeExit::Failed { detail } => format!("runtime failure: {detail}"),
        };
        self.record_state(LifecycleState::Stopping, stopping_reason);
        self.record_readiness(ReadinessState::Stopping, "runtime shutdown sequencing entered");

        if matches!(exit, RuntimeExit::Deferred { .. }) {
            self.summary_lines.push(format!(
                "shutdown-reason: {}",
                ShutdownReason::ServiceFailure(PRIMARY_SERVICE_NAME).as_str()
            ));
        }

        self.shutdown.cancel();
        self.drain_child_tasks().await;

        match exit {
            RuntimeExit::Clean => {
                self.record_state(LifecycleState::Stopped, "runtime stopped cleanly");
                self.record_readiness(ReadinessState::Stopping, "runtime stopped after clean shutdown");
            }
            RuntimeExit::Deferred { .. } | RuntimeExit::Failed { .. } => {
                self.record_state(
                    LifecycleState::Failed,
                    "runtime stopped with a deferred or failed service boundary",
                );
                self.record_readiness(
                    ReadinessState::Failed,
                    "runtime stopped after deferred or failed service boundary",
                );
            }
        }

        self.record_operability_summary();

        RuntimeExecution {
            summary_lines: self.summary_lines,
            exit,
        }
    }

    fn record_proxy_state(&mut self, state: ProxySeamState, detail: String) {
        self.proxy_state = Some(state);
        if state == ProxySeamState::Admitted {
            self.proxy_admissions += 1;
        }

        self.summary_lines
            .push(format!("proxy-state: {}", state.as_str()));
        self.summary_lines.push(format!("proxy-detail: {detail}"));
        info!("proxy-state state={} detail={detail}", state.as_str());
    }

    fn record_protocol_state(&mut self, state: ProtocolBridgeState, detail: impl Into<String>) {
        self.protocol_state = state;
        if state == ProtocolBridgeState::RegistrationObserved {
            self.protocol_registrations += 1;
        }

        let detail = detail.into();
        self.summary_lines
            .push(format!("protocol-state: {}", state.as_str()));
        self.summary_lines.push(format!("protocol-detail: {detail}"));
        info!("protocol-state state={} detail={detail}", state.as_str());
    }

    fn record_failure_boundary(&mut self, owner: &'static str, class: &'static str, detail: &str) {
        self.failure_events += 1;
        self.summary_lines.push(format!(
            "failure-visibility: owner={owner} class={class} detail={detail}"
        ));
        error!("failure-boundary owner={owner} class={class} detail={detail}");
    }

    fn refresh_readiness(&mut self, reason: impl Into<String>) {
        let next = if self.lifecycle_state == LifecycleState::Failed {
            ReadinessState::Failed
        } else if matches!(
            self.lifecycle_state,
            LifecycleState::Stopping | LifecycleState::Stopped
        ) {
            ReadinessState::Stopping
        } else if !matches!(
            self.proxy_state,
            Some(
                ProxySeamState::Admitted
                    | ProxySeamState::RegistrationObserved
                    | ProxySeamState::ShutdownAcknowledged
            )
        ) {
            ReadinessState::WaitingForProxyAdmission
        } else if !matches!(
            self.transport_stage,
            Some(
                TransportLifecycleStage::Established
                    | TransportLifecycleStage::ControlStreamOpened
                    | TransportLifecycleStage::Teardown
            )
        ) {
            ReadinessState::WaitingForTransport
        } else if self.protocol_state != ProtocolBridgeState::RegistrationObserved {
            ReadinessState::WaitingForProtocolBridge
        } else {
            ReadinessState::Ready
        };

        if next != self.readiness_state {
            self.record_readiness(next, reason);
        }
    }

    fn record_readiness(&mut self, state: ReadinessState, reason: impl Into<String>) {
        let reason = reason.into();
        self.readiness_state = state;
        self.summary_lines
            .push(format!("readiness-state: {}", state.as_str()));
        self.summary_lines.push(format!("readiness-reason: {reason}"));
        info!(
            "readiness-transition state={} scope={READINESS_SCOPE} reason={reason}",
            state.as_str()
        );
    }

    fn record_operability_summary(&mut self) {
        let transport_stage = self
            .transport_stage
            .map(|stage| stage.as_str())
            .unwrap_or("not-reported");
        let proxy_state = self
            .proxy_state
            .map(|state| state.as_str())
            .unwrap_or("not-reported");

        self.summary_lines.push(format!(
            "operability-status: lifecycle={} readiness={} transport-stage={} protocol-state={} \
             proxy-state={}",
            self.lifecycle_state.as_str(),
            self.readiness_state.as_str(),
            transport_stage,
            self.protocol_state.as_str(),
            proxy_state
        ));
        self.summary_lines.push(format!(
            "operability-metrics: restart-attempts={} proxy-admissions={} protocol-registrations={} \
             transport-failures={} failure-events={}",
            self.restart_attempts,
            self.proxy_admissions,
            self.protocol_registrations,
            self.transport_failures,
            self.failure_events
        ));
        info!(
            "operability-summary lifecycle={} readiness={} transport-stage={} protocol-state={} \
             proxy-state={} restart-attempts={} proxy-admissions={} protocol-registrations={} \
             transport-failures={} failure-events={}",
            self.lifecycle_state.as_str(),
            self.readiness_state.as_str(),
            transport_stage,
            self.protocol_state.as_str(),
            proxy_state,
            self.restart_attempts,
            self.proxy_admissions,
            self.protocol_registrations,
            self.transport_failures,
            self.failure_events
        );
    }

    fn spawn_proxy_seam(&mut self) {
        let ingress = self.config.normalized().ingress.clone();
        let seam = PingoraProxySeam::new(ingress);
        let protocol_rx = self.protocol_receiver.take();
        self.summary_lines.push(format!(
            "proxy-seam: origin-proxy admitted, ingress-rules={}",
            seam.ingress_count()
        ));
        seam.spawn(
            self.command_tx.clone(),
            protocol_rx,
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
        let reason = reason.into();
        self.summary_lines
            .push(format!("lifecycle-state: {}", state.as_str()));
        self.summary_lines.push(format!("lifecycle-reason: {reason}"));
        info!("lifecycle-transition state={} reason={reason}", state.as_str());
    }
}

fn glibc_runtime_marker_present(candidates: &[&str]) -> bool {
    candidates.iter().any(|path| fs::metadata(path).is_ok())
}

fn is_systemd_supervision_detected() -> bool {
    env::var_os("INVOCATION_ID").is_some()
        || env::var_os("NOTIFY_SOCKET").is_some()
        || env::var_os("JOURNAL_STREAM").is_some()
}

pub(crate) fn run(config: RuntimeConfig) -> RuntimeExecution {
    let (protocol_sender, protocol_receiver) = protocol::protocol_bridge();
    run_with_factory(
        config,
        QuicTunnelServiceFactory::production(protocol_sender),
        RuntimeHarness::production(),
        Some(protocol_receiver),
    )
}

pub(crate) fn run_with_factory<F>(
    config: RuntimeConfig,
    factory: F,
    harness: RuntimeHarness,
    protocol_receiver: Option<ProtocolReceiver>,
) -> RuntimeExecution
where
    F: RuntimeServiceFactory,
{
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for the admitted production-alpha shell");

    runtime.block_on(ApplicationRuntime::new(config, factory, harness, protocol_receiver).run())
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
        assert!(!super::glibc_runtime_marker_present(&[
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
        // Proxy seam admitted once, persists across primary service restarts.
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
}
