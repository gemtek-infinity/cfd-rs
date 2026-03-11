use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::protocol::{self, ProtocolBridgeState, ProtocolReceiver};
use crate::proxy::ProxySeamState;
use crate::transport::{QuicTunnelServiceFactory, TransportLifecycleStage};

use cloudflared_config::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt;

mod command_dispatch;
mod deployment;
mod state;
mod tasks;

#[cfg(test)]
mod tests;

use self::state::{LifecycleState, ReadinessState, RuntimeStatus};

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
    status: RuntimeStatus,
    protocol_receiver: Option<ProtocolReceiver>,
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
            status: RuntimeStatus::new(protocol_receiver.is_some()),
            protocol_receiver,
        }
    }

    async fn run(mut self) -> RuntimeExecution {
        self.status.record_runtime_owner();
        self.status.record_runtime_config(self.config.as_ref());
        self.status.record_supervision_policy(&self.policy);
        self.status.record_readiness_scope();

        if let Err(detail) = self.record_security_compliance_boundary() {
            return self.finish(RuntimeExit::Failed { detail }).await;
        }

        self.status
            .record_state(LifecycleState::Starting, "startup sequencing entered");
        self.status
            .record_readiness(ReadinessState::Starting, "runtime startup sequencing entered");
        self.status.record_protocol_state(
            self.status.protocol_state(),
            if self.status.protocol_bridge_is_present() {
                "runtime created protocol bridge endpoints"
            } else {
                "protocol bridge omitted by runtime harness"
            },
        );

        self.spawn_signal_bridge();
        self.spawn_harness_shutdown();
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

    async fn finish(mut self, exit: RuntimeExit) -> RuntimeExecution {
        let stopping_reason = match &exit {
            RuntimeExit::Clean => "graceful shutdown requested".to_owned(),
            RuntimeExit::Deferred { detail, .. } => {
                format!("deferred service boundary reached: {detail}")
            }
            RuntimeExit::Failed { detail } => format!("runtime failure: {detail}"),
        };
        self.status
            .record_state(LifecycleState::Stopping, stopping_reason);
        self.status
            .record_readiness(ReadinessState::Stopping, "runtime shutdown sequencing entered");

        if matches!(exit, RuntimeExit::Deferred { .. }) {
            self.status
                .record_shutdown_reason(&ShutdownReason::ServiceFailure(PRIMARY_SERVICE_NAME));
        }

        self.shutdown.cancel();
        self.drain_child_tasks().await;

        match exit {
            RuntimeExit::Clean => {
                self.status
                    .record_state(LifecycleState::Stopped, "runtime stopped cleanly");
                self.status
                    .record_readiness(ReadinessState::Stopping, "runtime stopped after clean shutdown");
            }
            RuntimeExit::Deferred { .. } | RuntimeExit::Failed { .. } => {
                self.status.record_state(
                    LifecycleState::Failed,
                    "runtime stopped with a deferred or failed service boundary",
                );
                self.status.record_readiness(
                    ReadinessState::Failed,
                    "runtime stopped after deferred or failed service boundary",
                );
            }
        }

        self.status.record_operability_summary();

        RuntimeExecution {
            summary_lines: self.status.into_summary_lines(),
            exit,
        }
    }
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
