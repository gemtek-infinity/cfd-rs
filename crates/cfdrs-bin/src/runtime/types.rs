use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use cfdrs_his::updater::AutoUpdateSettings;
use cfdrs_shared::{ConfigSource, DiscoveryOutcome, NormalizedConfig};
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
    /// Whether this binary runs in a container ("virtual") runtime.
    ///
    /// Matches Go compile-time `metrics.Runtime` variable. When true,
    /// the metrics server binds to `0.0.0.0` instead of `localhost`.
    is_container_runtime: bool,
    icmp_sources: Vec<String>,
    diagnostic_configuration: BTreeMap<String, String>,
    auto_update: Option<RuntimeAutoUpdate>,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct RuntimeAutoUpdate {
    settings: AutoUpdateSettings,
    target_path_override: Option<PathBuf>,
    base_url_override: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RuntimeAutoUpdate {
    pub(crate) fn new(settings: AutoUpdateSettings) -> Self {
        Self {
            settings,
            target_path_override: None,
            base_url_override: None,
        }
    }

    pub(crate) fn with_target_path_override(mut self, target_path: PathBuf) -> Self {
        self.target_path_override = Some(target_path);
        self
    }

    pub(crate) fn with_base_url_override(mut self, base_url: String) -> Self {
        self.base_url_override = Some(base_url);
        self
    }

    pub(crate) fn settings(&self) -> &AutoUpdateSettings {
        &self.settings
    }

    pub(crate) fn target_path_override(&self) -> Option<&PathBuf> {
        self.target_path_override.as_ref()
    }

    pub(crate) fn base_url_override(&self) -> Option<&str> {
        self.base_url_override.as_deref()
    }
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
            is_container_runtime: false,
            icmp_sources: Vec::new(),
            diagnostic_configuration: BTreeMap::new(),
            auto_update: None,
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

    pub(crate) fn with_container_runtime(mut self, is_container: bool) -> Self {
        self.is_container_runtime = is_container;
        self
    }

    pub(crate) fn with_icmp_sources(mut self, icmp_sources: Vec<String>) -> Self {
        self.icmp_sources = icmp_sources;
        self
    }

    pub(crate) fn with_diagnostic_configuration(
        mut self,
        diagnostic_configuration: BTreeMap<String, String>,
    ) -> Self {
        self.diagnostic_configuration = diagnostic_configuration;
        self
    }

    pub(crate) fn with_auto_update(mut self, auto_update: RuntimeAutoUpdate) -> Self {
        self.auto_update = Some(auto_update);
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

    pub(crate) fn is_container_runtime(&self) -> bool {
        self.is_container_runtime
    }

    pub(crate) fn quick_tunnel_hostname(&self) -> Option<String> {
        self.normalized()
            .ingress
            .iter()
            .filter_map(|rule| rule.matcher.hostname.clone())
            .find(|hostname| !hostname.is_empty())
    }

    pub(crate) fn tunnel_id(&self) -> Option<Uuid> {
        self.normalized().tunnel.as_ref().and_then(|tunnel| tunnel.uuid)
    }

    pub(crate) fn icmp_sources(&self) -> &[String] {
        &self.icmp_sources
    }

    pub(crate) fn diagnostic_configuration(&self) -> &BTreeMap<String, String> {
        &self.diagnostic_configuration
    }

    pub(crate) fn auto_update(&self) -> Option<&RuntimeAutoUpdate> {
        self.auto_update.as_ref()
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
    Updated { version: String },
    Deferred { phase: &'static str, detail: String },
    Failed { detail: String },
}

impl RuntimeExit {
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::Updated { .. } => cfdrs_his::updater::UPDATE_EXIT_SUCCESS as u8,
            Self::Deferred { .. } | Self::Failed { .. } => 1,
        }
    }

    pub(crate) fn stderr_message(&self) -> Option<String> {
        match self {
            Self::Clean => None,
            Self::Updated { .. } => None,
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
    TunnelConnectionObserved {
        index: u8,
        protocol: String,
        edge_address: String,
    },
    ServiceExited(ServiceExit),
    ShutdownRequested(ShutdownReason),
    ControlPlaneFailure {
        detail: String,
    },
    AutoUpdateApplied {
        version: String,
    },
    ConfigFileChanged {
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShutdownReason {
    Signal(&'static str),
    Harness,
    AutoUpdate,
    ServiceFailure(&'static str),
}

impl std::fmt::Display for ShutdownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal(name) => write!(f, "signal:{name}"),
            Self::Harness => f.write_str("harness"),
            Self::AutoUpdate => f.write_str("auto-update"),
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
    ConfigWatcher,
    AutoUpdater,
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
        assert_eq!(ShutdownReason::AutoUpdate.to_string(), "auto-update");
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
        .with_diagnostic_configuration(BTreeMap::from([("uid".to_owned(), "1000".to_owned())]))
        .with_metrics_bind_address("127.0.0.1:8080".parse().expect("socket address"))
        .with_auto_update(
            RuntimeAutoUpdate::new(AutoUpdateSettings::new(true, Duration::from_secs(15), None))
                .with_target_path_override(PathBuf::from("/tmp/cloudflared"))
                .with_base_url_override("http://127.0.0.1:8787".to_owned()),
        );

        assert_eq!(config.shutdown_grace_period(), Some(Duration::from_secs(45)));
        assert_eq!(
            config.pidfile_path(),
            Some(&PathBuf::from("/tmp/cloudflared.pid"))
        );
        assert_eq!(
            config.metrics_bind_address(),
            Some("127.0.0.1:8080".parse().expect("socket address"))
        );
        assert_eq!(
            config.diagnostic_configuration().get("uid"),
            Some(&"1000".to_owned())
        );
        assert!(config.auto_update().is_some());
    }
}
