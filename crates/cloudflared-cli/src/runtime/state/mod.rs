use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::startup::config_source_label;
use crate::transport::TransportLifecycleStage;

use tracing::{error, info, warn};

use super::{READINESS_SCOPE, RuntimeConfig, RuntimePolicy, ShutdownReason};

mod operability;
mod readiness;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LifecycleState {
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

    fn is_shutting_down(self) -> bool {
        matches!(self, Self::Stopping | Self::Stopped)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadinessState {
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

pub(super) struct RuntimeStatus {
    summary_lines: Vec<String>,
    lifecycle_state: LifecycleState,
    readiness_state: ReadinessState,
    restart_attempts: u32,
    transport_stage: Option<TransportLifecycleStage>,
    protocol_state: ProtocolBridgeState,
    proxy_state: Option<ProxySeamState>,
    proxy_admissions: u32,
    protocol_registrations: u32,
    transport_failures: u32,
    failure_events: u32,
    protocol_bridge_present: bool,
}

impl RuntimeStatus {
    pub(super) fn new(protocol_bridge_present: bool) -> Self {
        Self {
            summary_lines: Vec::new(),
            lifecycle_state: LifecycleState::Starting,
            readiness_state: ReadinessState::Starting,
            restart_attempts: 0,
            transport_stage: None,
            protocol_state: initial_protocol_state(protocol_bridge_present),
            proxy_state: None,
            proxy_admissions: 0,
            protocol_registrations: 0,
            transport_failures: 0,
            failure_events: 0,
            protocol_bridge_present,
        }
    }

    pub(super) fn into_summary_lines(self) -> Vec<String> {
        self.summary_lines
    }

    pub(super) fn push_summary(&mut self, line: impl Into<String>) {
        self.summary_lines.push(line.into());
    }

    pub(super) fn lifecycle_state(&self) -> LifecycleState {
        self.lifecycle_state
    }

    pub(super) fn protocol_state(&self) -> ProtocolBridgeState {
        self.protocol_state
    }

    pub(super) fn protocol_bridge_is_present(&self) -> bool {
        self.protocol_bridge_present
    }

    pub(super) fn restart_attempts(&self) -> u32 {
        self.restart_attempts
    }

    pub(super) fn increment_transport_failures(&mut self) {
        self.transport_failures += 1;
    }

    pub(super) fn record_restart_attempt(&mut self, service: &'static str, detail: &str) -> u32 {
        self.restart_attempts += 1;
        self.summary_lines.push(format!(
            "supervision-restart-attempt: {} service={} detail={detail}",
            self.restart_attempts, service
        ));
        warn!(
            "runtime-restart service={service} attempt={} detail={detail}",
            self.restart_attempts
        );
        self.restart_attempts
    }

    pub(super) fn record_runtime_owner(&mut self) {
        self.push_summary("runtime-owner: initialized");
        self.push_summary("config-ownership: runtime-owned");
    }

    pub(super) fn record_runtime_config(&mut self, config: &RuntimeConfig) {
        self.push_summary(format!(
            "runtime-config-source: {}",
            config_source_label(config.config_source())
        ));
        self.push_summary(format!("runtime-config-path: {}", config.config_path().display()));
        self.push_summary(format!(
            "runtime-ingress-rules: {}",
            config.normalized().ingress.len()
        ));
    }

    pub(super) fn record_supervision_policy(&mut self, policy: &RuntimePolicy) {
        self.push_summary(format!(
            "supervision-policy: primary-service={} max-restarts={} restart-backoff-ms={} \
             shutdown-grace-ms={}",
            super::PRIMARY_SERVICE_NAME,
            policy.max_restart_attempts,
            policy.restart_backoff.as_millis(),
            policy.shutdown_grace_period.as_millis()
        ));
    }

    pub(super) fn record_readiness_scope(&mut self) {
        self.push_summary(format!("readiness-scope: {READINESS_SCOPE}"));
    }

    pub(super) fn record_service_status(&mut self, service: &'static str, detail: String) {
        self.summary_lines
            .push(format!("service-status[{service}]: {detail}"));
        info!("service-status service={service} detail={detail}");
    }

    pub(super) fn record_transport_stage(
        &mut self,
        service: &'static str,
        stage: TransportLifecycleStage,
        detail: String,
    ) {
        self.transport_stage = Some(stage);
        self.summary_lines
            .push(format!("transport-stage[{service}]: {}", stage.as_str()));
        self.summary_lines
            .push(format!("transport-detail[{service}]: {detail}"));
        info!(
            "transport-stage service={service} stage={} detail={detail}",
            stage.as_str()
        );
    }

    pub(super) fn record_proxy_state(&mut self, state: ProxySeamState, detail: String) {
        self.proxy_state = Some(state);
        self.proxy_admissions += u32::from(state == ProxySeamState::Admitted);

        self.summary_lines
            .push(format!("proxy-state: {}", state.as_str()));
        self.summary_lines.push(format!("proxy-detail: {detail}"));
        info!("proxy-state state={} detail={detail}", state.as_str());
    }

    pub(super) fn record_protocol_state(&mut self, state: ProtocolBridgeState, detail: impl Into<String>) {
        self.protocol_state = state;
        self.protocol_registrations += u32::from(state == ProtocolBridgeState::RegistrationObserved);

        let detail = detail.into();
        self.summary_lines
            .push(format!("protocol-state: {}", state.as_str()));
        self.summary_lines.push(format!("protocol-detail: {detail}"));
        info!("protocol-state state={} detail={detail}", state.as_str());
    }

    pub(super) fn record_shutdown_reason(&mut self, reason: &ShutdownReason) {
        let reason = reason.as_str();
        self.summary_lines.push(format!("shutdown-reason: {reason}"));
        info!("runtime-shutdown-request reason={reason}");
    }

    pub(super) fn record_failure_boundary(&mut self, owner: &'static str, class: &'static str, detail: &str) {
        self.failure_events += 1;
        self.summary_lines.push(format!(
            "failure-visibility: owner={owner} class={class} detail={detail}"
        ));
        error!("failure-boundary owner={owner} class={class} detail={detail}");
    }

    pub(super) fn record_state(&mut self, state: LifecycleState, reason: impl Into<String>) {
        self.lifecycle_state = state;
        let reason = reason.into();
        self.summary_lines
            .push(format!("lifecycle-state: {}", state.as_str()));
        self.summary_lines.push(format!("lifecycle-reason: {reason}"));
        info!("lifecycle-transition state={} reason={reason}", state.as_str());
    }
}

fn initial_protocol_state(protocol_bridge_present: bool) -> ProtocolBridgeState {
    if protocol_bridge_present {
        ProtocolBridgeState::BridgeCreated
    } else {
        ProtocolBridgeState::BridgeUnavailable
    }
}
