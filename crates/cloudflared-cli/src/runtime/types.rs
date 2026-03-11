use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cloudflared_config::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::transport::TransportLifecycleStage;

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

impl ShutdownReason {
    pub(super) fn as_str(&self) -> String {
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
    pub(super) enable_signals: bool,
    pub(super) injected_shutdown_after: Option<Duration>,
}

impl RuntimeHarness {
    pub(super) fn production() -> Self {
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
    pub(super) fn with_shutdown_after(mut self, duration: Duration) -> Self {
        self.injected_shutdown_after = Some(duration);
        self
    }
}
