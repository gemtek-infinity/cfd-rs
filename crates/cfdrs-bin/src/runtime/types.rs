use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cfdrs_shared::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::transport::TransportLifecycleStage;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    discovery: DiscoveryOutcome,
    normalized: NormalizedConfig,
    connector_id: Uuid,
    shutdown_grace_period: Option<Duration>,
    pidfile_path: Option<PathBuf>,
    metrics_bind_address: Option<SocketAddr>,
}

impl RuntimeConfig {
    pub(crate) fn new(discovery: DiscoveryOutcome, normalized: NormalizedConfig) -> Self {
        Self {
            discovery,
            normalized,
            connector_id: Uuid::new_v4(),
            shutdown_grace_period: None,
            pidfile_path: None,
            metrics_bind_address: None,
        }
    }

    pub(crate) fn with_shutdown_grace_period(mut self, shutdown_grace_period: Duration) -> Self {
        self.shutdown_grace_period = Some(shutdown_grace_period);
        self
    }

    pub(crate) fn with_pidfile_path(mut self, pidfile_path: PathBuf) -> Self {
        self.pidfile_path = Some(pidfile_path);
        self
    }

    pub(crate) fn with_metrics_bind_address(mut self, metrics_bind_address: SocketAddr) -> Self {
        self.metrics_bind_address = Some(metrics_bind_address);
        self
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

    pub(super) fn connector_id(&self) -> Uuid {
        self.connector_id
    }

    pub(super) fn shutdown_grace_period(&self) -> Option<Duration> {
        self.shutdown_grace_period
    }

    pub(super) fn pidfile_path(&self) -> Option<&PathBuf> {
        self.pidfile_path.as_ref()
    }

    pub(super) fn metrics_bind_address(&self) -> Option<SocketAddr> {
        self.metrics_bind_address
    }
}

#[derive(Debug, Clone)]
pub(super) struct RuntimePolicy {
    pub(super) max_restart_attempts: u32,
    pub(super) restart_backoff: Duration,
    pub(super) shutdown_grace_period: Duration,
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

impl std::fmt::Display for ShutdownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal(name) => write!(f, "signal:{name}"),
            Self::Harness => f.write_str("harness"),
            Self::ServiceFailure(name) => write!(f, "service-failure:{name}"),
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

#[derive(Debug, Clone)]
pub(crate) struct RuntimeHarness {
    pub(super) enable_signals: bool,
    pub(super) injected_shutdown_after: Option<Duration>,
}

/// Typestate markers for [`HarnessBuilder`] construction modes.
pub(crate) mod harness_mode {
    /// Production mode: signals are enabled, test injection is unavailable.
    pub struct Production;
    /// Test mode: signals are disabled, shutdown injection is available.
    #[cfg(test)]
    pub struct Testing;
}

/// Typestate builder for [`RuntimeHarness`].
///
/// The mode parameter prevents production harnesses from accidentally
/// using test-only configuration like
/// [`with_shutdown_after`](Self::with_shutdown_after), and ensures test
/// harnesses always disable signal handlers.
pub(crate) struct HarnessBuilder<Mode> {
    enable_signals: bool,
    injected_shutdown_after: Option<Duration>,
    _mode: std::marker::PhantomData<Mode>,
}

impl HarnessBuilder<harness_mode::Production> {
    pub(crate) fn production() -> Self {
        Self {
            enable_signals: true,
            injected_shutdown_after: None,
            _mode: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
impl HarnessBuilder<harness_mode::Testing> {
    pub(crate) fn for_tests() -> Self {
        Self {
            enable_signals: false,
            injected_shutdown_after: None,
            _mode: std::marker::PhantomData,
        }
    }

    pub(crate) fn with_shutdown_after(mut self, duration: Duration) -> Self {
        self.injected_shutdown_after = Some(duration);
        self
    }
}

impl<Mode> HarnessBuilder<Mode> {
    pub(crate) fn build(self) -> RuntimeHarness {
        RuntimeHarness {
            enable_signals: self.enable_signals,
            injected_shutdown_after: self.injected_shutdown_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_reason_display() {
        assert_eq!(ShutdownReason::Signal("SIGTERM").to_string(), "signal:SIGTERM");
        assert_eq!(ShutdownReason::Harness.to_string(), "harness");
        assert_eq!(
            ShutdownReason::ServiceFailure("quic-transport").to_string(),
            "service-failure:quic-transport"
        );
    }

    #[test]
    fn runtime_config_retains_optional_runtime_overrides() {
        let config = RuntimeConfig::new(
            DiscoveryOutcome {
                action: cfdrs_shared::DiscoveryAction::UseExisting,
                path: PathBuf::from("/tmp/config.yml"),
                source: ConfigSource::DiscoveredPath(PathBuf::from("/tmp/config.yml")),
                created_paths: vec![],
                written_config: None,
            },
            NormalizedConfig {
                source: ConfigSource::DiscoveredPath(PathBuf::from("/tmp/config.yml")),
                tunnel: None,
                credentials: cfdrs_shared::CredentialSurface::default(),
                ingress: vec![],
                origin_request: cfdrs_shared::OriginRequestConfig::default(),
                warp_routing: cfdrs_shared::WarpRoutingConfig::default(),
                log_directory: None,
                warnings: vec![],
            },
        )
        .with_shutdown_grace_period(Duration::from_secs(45))
        .with_pidfile_path(PathBuf::from("/tmp/cloudflared.pid"))
        .with_metrics_bind_address("127.0.0.1:8080".parse().expect("socket address"));

        assert_eq!(config.shutdown_grace_period(), Some(Duration::from_secs(45)));
        assert_eq!(
            config.pidfile_path(),
            Some(&PathBuf::from("/tmp/cloudflared.pid"))
        );
        assert_eq!(
            config.metrics_bind_address(),
            Some("127.0.0.1:8080".parse().expect("socket address"))
        );
    }
}
